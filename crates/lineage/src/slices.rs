//! Asks whether a modelled line item is the whole of what its name suggests.
//!
//! # The error this exists to catch
//!
//! Six times now this corpus has summed a category and called it a programme, and six times the
//! boundary was drawn in the wrong place — by recipient class, by distribution estimate, by fund,
//! by line item, and by agency. The costliest was quiet: `Foundation Funding - All Students` is
//! **three** ALIs differing only in the fund they draw on, and the corpus modelled one of them,
//! calling 80.6% of a programme by the programme's name. Nothing detected it. The figure was
//! correct, its provenance was correct, and it answered a question nobody had asked.
//!
//! The signature is always the same and it is cheap to look for: **another line item, in the same
//! agency, carrying the same name.** That is what this reports.
//!
//! # It proposes and stops
//!
//! Like [`crate::propose`], and for the same reason. Whether two line items sharing a name are
//! one programme is a modelling judgment: `Foundation Funding - All Students` under three funds
//! is one programme, and the nine `Medicaid`-named line items at Developmental Disabilities are
//! several. No edge is written and no total is computed.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use corpus_validate::Corpus;
use lsc::columns::{ColumnPlan, Identity};

/// A line item the corpus models, and what sits beside it in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Sibling {
    pub code: String,
    pub name: String,
    pub cents: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub slug: String,
    pub code: String,
    pub agency: String,
    pub name: String,
    pub cents: i64,
    /// Other line items in the same agency carrying the same name.
    pub siblings: Vec<Sibling>,
}

impl Finding {
    /// This line item's share of itself and its namesakes.
    ///
    /// The number that would have caught foundation funding: 80.6%, on a node whose prose said
    /// it was the programme.
    pub fn share(&self) -> Option<f64> {
        let total = self.cents + self.siblings.iter().map(|s| s.cents).sum::<i64>();
        (total != 0).then(|| self.cents as f64 / total as f64 * 100.0)
    }
}

/// Names that mark administration rather than the programme itself.
///
/// Excluded from the sibling set because they are genuinely a different thing, and reporting
/// `Medicaid Program Support` beside `Medicaid Health Care Services` would bury the real finding
/// under noise. `[inference]` from the name, which is the only basis available — a workbook does
/// not say which of its rows are overhead.
fn is_administrative(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "program support",
        "administration",
        "oversight",
        "fraud",
        "audit",
        "reconciliation",
    ]
    .iter()
    .any(|k| n.contains(k))
}

/// Two names refer to the same thing when either is a prefix of the other.
///
/// Prefix rather than equality because Ohio qualifies rather than renames: `Medicaid Services`
/// and `Medicaid Services - Federal` are the pattern, and requiring equality would miss every
/// one of them. It over-matches by design — the output is read by a person.
///
/// The stem must be **at least two words and twelve characters**. Without that, `Medicaid` — a
/// line item name in its own right — prefixes every Medicaid-qualified item in four agencies and
/// the report becomes a list of everything. A single generic word is not a programme name, and
/// a report nobody can read is the same as no report.
fn same_programme(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim().to_ascii_lowercase(), b.trim().to_ascii_lowercase());
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    short.len() >= 12 && short.split_whitespace().count() >= 2 && long.starts_with(short.as_str())
}

/// Reads one workbook and reports, for every line item the corpus models, its namesakes.
pub fn check(corpus: &Corpus, workbook: &Path, fiscal_year: &str) -> Result<Vec<Finding>> {
    let table = lsc::xlsx::sheet_to_table(workbook, "EN")?;
    let plan = ColumnPlan::of(&table.headers);
    let col = plan
        .appropriation_columns()
        .into_iter()
        .find(|(_, s, y)| *s == corpus_schema::BillStage::AsEnacted && *y == fiscal_year)
        .map(|(i, _, _)| i);
    let (Some(c), Some(a), Some(l), Some(n)) = (
        col,
        plan.identity_column(Identity::Agency),
        plan.identity_column(Identity::LineItemCode),
        plan.identity_column(Identity::LineItemName),
    ) else {
        return Ok(Vec::new());
    };

    // (agency, code) -> (name, cents), keeping the largest row where a code repeats: the
    // memorandum breakdowns beneath a schedule share their parent's code and would otherwise
    // register as siblings of it.
    let mut rows: BTreeMap<(String, String), (String, i64)> = BTreeMap::new();
    for r in &table.rows {
        let Ok(cents) = lsc::parse_money_to_cents(&r[c]) else {
            continue;
        };
        let k = (r[a].trim().to_string(), r[l].trim().to_string());
        let e = rows
            .entry(k)
            .or_insert_with(|| (r[n].trim().to_string(), i64::MIN));
        if cents > e.1 {
            *e = (r[n].trim().to_string(), cents);
        }
    }

    let mut out = Vec::new();
    for inst in corpus
        .instances
        .iter()
        .filter(|i| i.inst.class == "line-item")
    {
        let p = |k: &str| corpus_validate::property_text(&inst.inst, k).map(str::trim);
        let (Some(code), Some(agency)) = (p("current_code"), p("agency_code")) else {
            continue;
        };
        if code.contains("[open]") || agency.contains("[open]") {
            continue;
        }
        let Some((name, cents)) = rows.get(&(agency.to_string(), code.to_string())) else {
            continue;
        };
        let siblings: Vec<Sibling> = rows
            .iter()
            .filter(|((ag, cd), (nm, _))| {
                ag == agency && cd != code && same_programme(nm, name) && !is_administrative(nm)
            })
            .map(|((_, cd), (nm, v))| Sibling {
                code: cd.clone(),
                name: nm.clone(),
                cents: *v,
            })
            .collect();
        if siblings.is_empty() {
            continue;
        }
        out.push(Finding {
            slug: inst
                .rel_path
                .rsplit('/')
                .next()
                .unwrap_or("")
                .trim_end_matches(".yml")
                .to_string(),
            code: code.to_string(),
            agency: agency.to_string(),
            name: name.clone(),
            cents: *cents,
            siblings,
        });
    }
    out.sort_by_key(|f| f.slug.clone());
    Ok(out)
}

pub fn render(findings: &[Finding], fiscal_year: &str) -> String {
    if findings.is_empty() {
        return format!(
            "No modelled line item shares its name with another in the same agency in \
             {fiscal_year}.\n\nThat is the check passing, not the check finding nothing to do: \
             it is the signature that\nmade `Foundation Funding - All Students` three line items \
             and the corpus model one.\n"
        );
    }
    let mut s = format!(
        "{} modelled line item(s) share a name with something else in the same agency, \
         {fiscal_year}\n\n",
        findings.len()
    );
    for f in findings {
        s.push_str(&format!(
            "  {} — ALI {} [{}], {}\n    this node: {:>16} cents{}\n",
            f.slug,
            f.code,
            f.agency,
            f.name,
            f.cents,
            f.share()
                .map(|p| format!("  ({p:.1}% of the namesakes)"))
                .unwrap_or_default()
        ));
        for sib in &f.siblings {
            s.push_str(&format!(
                "    also:      {:>16} cents  ALI {} {}\n",
                sib.cents, sib.code, sib.name
            ));
        }
        s.push('\n');
    }
    s.push_str(
        "Sharing a name is not being one programme. Three ALIs called `Foundation Funding - All\n\
         Students` are one; nine called `Medicaid` at Developmental Disabilities are several.\n\
         Nothing was written.\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_qualified_name_matches_its_stem() {
        assert!(same_programme(
            "Medicaid Services",
            "Medicaid Services - Federal"
        ));
        assert!(same_programme(
            "Foundation Funding - All Students",
            "Foundation Funding - All Students"
        ));
    }

    #[test]
    fn a_short_stem_does_not_match_everything() {
        // `Medicaid` is a line item name in its own right and prefixes Medicaid-qualified items
        // in four agencies. One generic word is not a programme.
        assert!(!same_programme("Medicaid", "Medicaid Services - Federal"));
        assert!(!same_programme("Medicaid Services", "Medicaid"));
        // Two words and twelve characters is the floor, and the real names clear it.
        assert!(same_programme(
            "Pupil Transportation",
            "Pupil Transportation - Federal"
        ));
    }

    #[test]
    fn unrelated_names_sharing_a_first_word_do_not_match() {
        assert!(!same_programme(
            "Property Tax Allocation - Taxation",
            "Property Tax Reimbursement - Education"
        ));
    }

    #[test]
    fn administration_is_not_a_sibling_of_the_programme() {
        // `Medicaid Program Support` beside `Medicaid Health Care Services` is noise, and noise
        // in this report is what stops it being read.
        assert!(is_administrative("Medicaid Program Support - Federal"));
        assert!(is_administrative(
            "State Share of Instruction Reconciliation"
        ));
        assert!(!is_administrative("Medicaid Services - Federal"));
    }
}
