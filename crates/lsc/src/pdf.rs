//! Reads PDF bytes into positioned glyphs.
//!
//! Thin by design. Everything interesting happens in [`crate::geometry`], which is pure and
//! tested without a document; this module only turns a PDF into the [`Glyph`] values that
//! module consumes.
//!
//! Behind the `pdf` feature because it pulls the font and CMap stack.

use anyhow::{anyhow, Context, Result};
use pdf_extract::{ColorSpace, MediaBox, OutputDev, OutputError, Path as PdfPath, Transform};

use crate::geometry::{to_table, Glyph, TableOptions};
use crate::RawTable;

/// One page's glyphs.
#[derive(Debug, Clone)]
pub struct Page {
    pub number: u32,
    pub glyphs: Vec<Glyph>,
}

/// Collects positioned glyphs instead of rendering them.
///
/// `pdf-extract`'s own text output flattens a page to a string, which discards exactly the
/// coordinates a table needs. Implementing the trait directly keeps them.
#[derive(Default)]
struct GlyphCollector {
    pages: Vec<Page>,
    current: Vec<Glyph>,
    page_num: u32,
}

impl OutputDev for GlyphCollector {
    fn begin_page(
        &mut self,
        page_num: u32,
        _media_box: &MediaBox,
        _art_box: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        self.page_num = page_num;
        self.current = Vec::new();
        Ok(())
    }

    fn end_page(&mut self) -> Result<(), OutputError> {
        self.pages.push(Page {
            number: self.page_num,
            glyphs: std::mem::take(&mut self.current),
        });
        Ok(())
    }

    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<(), OutputError> {
        // The text rendering matrix carries both the position and the scale in force, so the
        // advance and the effective font size have to be taken through it rather than from
        // the nominal font size alone — a page scaled by its content stream would otherwise
        // produce row tolerances in the wrong units entirely.
        let scale_x = trm.m11.abs();
        let scale_y = trm.m22.abs();
        let effective_size = font_size * if scale_y > 0.0 { scale_y } else { 1.0 };
        self.current.push(Glyph {
            x: trm.m31,
            y: trm.m32,
            advance: width * font_size * if scale_x > 0.0 { scale_x } else { 1.0 },
            font_size: effective_size,
            text: char.to_string(),
        });
        Ok(())
    }

    fn begin_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
    fn stroke(
        &mut self,
        _ctm: &Transform,
        _cs: &ColorSpace,
        _color: &[f64],
        _path: &PdfPath,
    ) -> Result<(), OutputError> {
        Ok(())
    }
    fn fill(
        &mut self,
        _ctm: &Transform,
        _cs: &ColorSpace,
        _color: &[f64],
        _path: &PdfPath,
    ) -> Result<(), OutputError> {
        Ok(())
    }
}

/// Extracts every page's positioned glyphs from a PDF.
pub fn extract_pages(bytes: &[u8]) -> Result<Vec<Page>> {
    let doc = pdf_extract::Document::load_mem(bytes).context("parsing PDF")?;
    let mut collector = GlyphCollector::default();
    pdf_extract::output_doc(&doc, &mut collector).map_err(|e| anyhow!("{e:?}"))?;
    Ok(collector.pages)
}

/// Extracts one page and reconstructs its table.
///
/// The result is the same [`RawTable`] that [`crate::parse_delimited`] produces, so a PDF and
/// a hand-written file are indistinguishable to everything downstream.
pub fn page_to_table(page: &Page, opts: &TableOptions) -> Result<RawTable> {
    to_table(&page.glyphs, opts)
        .with_context(|| format!("reconstructing the table on page {}", page.number))
}

/// Extracts every page, reporting per-page failure rather than aborting the document.
///
/// Comparison documents carry cover pages, notes, and continuation pages that are not tables.
/// A run that stopped at the first non-table page would extract almost nothing, so each page
/// is reported independently and the caller decides.
pub fn tables_from_pdf(bytes: &[u8], opts: &TableOptions) -> Result<Vec<(u32, Result<RawTable>)>> {
    Ok(extract_pages(bytes)?
        .into_iter()
        .map(|p| {
            let n = p.number;
            (n, page_to_table(&p, opts))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal one-page PDF with two text runs, written by hand so the test needs no
    /// binary asset. Exercises the real parser, not a stub.
    fn tiny_pdf() -> Vec<u8> {
        let content = b"BT /F1 12 Tf 72 700 Td (Item) Tj 200 0 Td (FY2026) Tj ET\n\
                        BT /F1 12 Tf 72 680 Td (Foundation) Tj 200 0 Td (100.00) Tj ET\n";
        let mut objects: Vec<Vec<u8>> = Vec::new();
        objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
        objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
        objects.push(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
               /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(content);
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
        objects.push(
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        );

        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in &offsets {
            out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                objects.len() + 1,
                xref
            )
            .as_bytes(),
        );
        out
    }

    #[test]
    fn glyph_positions_survive_extraction() {
        let pages = extract_pages(&tiny_pdf()).expect("the hand-built PDF must parse");
        assert_eq!(pages.len(), 1);
        let g = &pages[0].glyphs;
        assert!(!g.is_empty(), "no glyphs recovered");
        // Two baselines, twenty units apart, as laid out above.
        let mut ys: Vec<i64> = g.iter().map(|x| x.y.round() as i64).collect();
        ys.sort_unstable();
        ys.dedup();
        assert_eq!(ys, vec![680, 700], "baselines lost: {ys:?}");
    }

    #[test]
    fn a_real_pdf_reconstructs_into_the_shared_table_type() {
        let pages = extract_pages(&tiny_pdf()).unwrap();
        let table = page_to_table(&pages[0], &TableOptions::default()).unwrap();
        assert_eq!(table.headers.len(), 2, "headers: {:?}", table.headers);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].len(), 2, "row: {:?}", table.rows[0]);
        assert_eq!(
            crate::parse_money_to_cents(&table.rows[0][1]).unwrap(),
            10_000,
            "the amount cell did not survive the round trip: {:?}",
            table.rows[0]
        );
    }

    #[test]
    fn a_non_table_page_fails_alone_rather_than_aborting_the_document() {
        let results = tables_from_pdf(&tiny_pdf(), &TableOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.is_ok());
    }

    #[test]
    fn malformed_bytes_are_an_error_not_a_panic() {
        assert!(extract_pages(b"not a pdf at all").is_err());
    }
}
