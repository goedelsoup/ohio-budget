//! Parses every committed fixture against the type it claims to demonstrate.
//!
//! Without this, a fixture drifts the moment a type changes and then quietly stops being a
//! worked example of anything. A failure here means either the fixture or the type moved
//! and the other did not.

use std::path::{Path, PathBuf};

use corpus_schema::*;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.yidam/fixtures")
        .canonicalize()
        .expect("fixtures directory must exist")
}

fn yaml_files(sub: &str) -> Vec<PathBuf> {
    let dir = fixtures_dir().join(sub);
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no fixtures under {}", dir.display());
    out
}

fn parse_all<T: serde::de::DeserializeOwned>(sub: &str) -> Vec<T> {
    let mut all = Vec::new();
    for path in yaml_files(sub) {
        let text = std::fs::read_to_string(&path).unwrap();
        let records: Vec<T> = serde_yaml::from_str(&text)
            .unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
        assert!(!records.is_empty(), "{} is empty", path.display());
        all.extend(records);
    }
    all
}

#[test]
fn lsc_fixtures_parse() {
    let rows: Vec<LscComparisonRow> = parse_all("lsc");
    assert!(rows.len() >= 3);
    // The point of the fixture: one line item observed at more than one stage.
    let foundation: Vec<&LscComparisonRow> = rows
        .iter()
        .filter(|r| r.line_item_code == "200550" && r.fiscal_year == "FY2026")
        .collect();
    assert!(
        foundation.len() >= 3,
        "stage-delta needs the same line item at multiple stages"
    );
}

#[test]
fn obm_fixtures_parse_and_cover_both_bases() {
    let rows: Vec<ObmExpenditureRow> = parse_all("obm");
    assert!(rows
        .iter()
        .any(|r| r.basis == ExpenditureBasis::ActualClosed));
    assert!(rows.iter().any(|r| r.basis == ExpenditureBasis::Disbursed));
    assert!(
        rows.iter().any(|r| r.reversion_cents.is_some()),
        "gap reports a reversion rate and needs a fixture that has one"
    );
    assert!(
        rows.iter().any(|r| r.reversion_cents.is_none()),
        "absent and zero are different claims; both must be exercised"
    );
}

#[test]
fn controlling_board_fixtures_include_a_non_approved_disposition() {
    let rows: Vec<ControllingBoardRequest> = parse_all("controlling-board");
    assert!(
        rows.iter()
            .any(|r| r.disposition != ControllingBoardDisposition::Approved),
        "a parser keying only on amount_delta_cents would overstate execution-phase movement"
    );
    assert!(
        rows.iter().any(|r| r.line_item_code.is_none()),
        "some requests operate at fund level with no line item"
    );
}

#[test]
fn legislature_fixtures_include_an_incomplete_stage_chain() {
    let bills: Vec<LegislatureBill> = parse_all("legislature");
    let hb96 = bills
        .iter()
        .find(|b| b.bill_number == "HB 96")
        .expect("HB 96 fixture");
    assert_eq!(hb96.versions.len(), 3);
    assert!(
        !hb96
            .versions
            .iter()
            .any(|v| v.stage == BillStage::AsPassedSenate),
        "the gap between House and enacted is the case downstream code must handle"
    );
    assert!(bills.iter().any(|b| b.versions.is_empty()));
}

#[test]
fn every_fixture_is_marked_synthetic() {
    // The guard that keeps fabricated values from being mistaken for extracted ones.
    for sub in ["lsc", "obm", "controlling-board", "legislature"] {
        for path in yaml_files(sub) {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(
                text.contains("synthetic-fixture"),
                "{} must carry synthetic provenance",
                path.display()
            );
            assert!(
                !text.contains("retrieved: \"20"),
                "{} carries a real-looking retrieval date",
                path.display()
            );
        }
    }
}
