//! Reads a veto message PDF into text.
//!
//! Thin by design, and deliberately unlike [`lsc::pdf`](../../lsc/): that connector needs
//! glyph coordinates because a table's structure is spatial. A veto message's structure is its
//! reading order, which a text extraction already preserves, so taking the flattened text is
//! not a shortcut here — it is the right input.

use anyhow::{Context, Result};

/// Extracts the document's text in reading order.
pub fn text_from_pdf(bytes: &[u8]) -> Result<String> {
    pdf_extract::extract_text_from_mem(bytes).context("extracting text from the veto message")
}
