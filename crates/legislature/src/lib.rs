//! Connector for the Ohio General Assembly bill record.
//!
//! Produces [`LegislatureBill`] records: bill identity, enactment dates, and the sequence of
//! versions each appropriation bill passed through. This is the skeleton the
//! [`lsc`](../../lsc/) figures hang on — an appropriation row has nowhere to attach until the
//! bill version it belongs to exists.
//!
//! # Retrieval boundary
//!
//! Everything here except the network call itself is pure and tested: URL construction, the
//! General Assembly's stage vocabulary, and the mapping from a fetched record onto corpus
//! node identities. [`HttpSource`] is the one piece that is not implemented, and it is a thin
//! wrapper over [`document_url`] when it is.
//!
//! Offline operation is not a fallback here, it is the default. Per the directory conventions
//! a connector must run against committed fixtures so tests and analysis stay hermetic.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use corpus_schema::{BillStage, LegislatureBill};

/// A bill, identified the only way that is unambiguous in this domain.
///
/// Bill numbers recur across General Assemblies for entirely unrelated legislation, so
/// `HB 96` alone identifies nothing. Every lookup takes the pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillRef {
    pub assembly: String,
    pub number: String,
}

impl BillRef {
    pub fn new(assembly: &str, number: &str) -> Self {
        Self {
            assembly: assembly.to_string(),
            number: number.to_string(),
        }
    }

    /// `HB 96` -> `hb96`. The form used in document paths and corpus filenames.
    pub fn slug(&self) -> String {
        self.number
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(|c| c.to_lowercase())
            .collect()
    }

    /// The corpus filename this bill should map to, e.g. `hb96-136th`.
    ///
    /// Deriving this rather than hand-writing it is what keeps extracted records and corpus
    /// nodes addressable by the same key.
    pub fn corpus_slug(&self) -> String {
        format!("{}-{}", self.slug(), self.assembly)
    }
}

/// Builds the published record URL for a bill.
///
/// Pure and tested so that the unimplemented [`HttpSource`] is a thin wrapper rather than a
/// place where logic hides behind a network call.
pub fn document_url(base: &str, bill: &BillRef) -> String {
    format!(
        "{}/legislation/{}/{}",
        base.trim_end_matches('/'),
        bill.assembly
            .trim_end_matches("th")
            .trim_end_matches("nd")
            .trim_end_matches("rd")
            .trim_end_matches("st"),
        bill.slug()
    )
}

/// Maps the General Assembly's stage vocabulary onto the corpus's.
///
/// The published record uses several spellings for the same stage, and the corpus uses one.
/// Normalizing here rather than at the call site means the vocabulary lives in exactly one
/// place when it inevitably grows.
pub fn parse_stage(raw: &str) -> Option<BillStage> {
    let n: String = raw
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let n = n.split_whitespace().collect::<Vec<_>>().join(" ");
    match n.as_str() {
        "as introduced" | "introduced" | "as filed" => Some(BillStage::AsIntroduced),
        "as passed by the house" | "as passed house" | "house passed" | "substitute house bill" => {
            Some(BillStage::AsPassedHouse)
        }
        "as passed by the senate" | "as passed senate" | "senate passed" => {
            Some(BillStage::AsPassedSenate)
        }
        "conference report"
        | "as reported by conference committee"
        | "conference committee report" => Some(BillStage::ConferenceReport),
        "as enacted" | "enacted" | "as signed by the governor" | "final" => {
            Some(BillStage::AsEnacted)
        }
        "post veto" | "as vetoed" | "after line item veto" => Some(BillStage::PostVeto),
        _ => None,
    }
}

/// Where bill records come from.
pub trait Source {
    fn fetch(&self, bill: &BillRef) -> Result<LegislatureBill>;
    fn fetch_all(&self) -> Result<Vec<LegislatureBill>>;
}

/// Reads committed fixtures. The default, and the only source usable in tests.
pub struct FixtureSource {
    root: PathBuf,
}

impl FixtureSource {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Locates `.yidam/fixtures/legislature` relative to a repository root.
    pub fn from_repo(repo_root: impl AsRef<Path>) -> Self {
        Self::new(repo_root.as_ref().join(".yidam/fixtures/legislature"))
    }
}

impl Source for FixtureSource {
    fn fetch(&self, bill: &BillRef) -> Result<LegislatureBill> {
        self.fetch_all()?
            .into_iter()
            .find(|b| b.bill_number == bill.number && b.general_assembly == bill.assembly)
            .ok_or_else(|| anyhow!("no fixture for {} ({})", bill.number, bill.assembly))
    }

    fn fetch_all(&self) -> Result<Vec<LegislatureBill>> {
        let mut out = Vec::new();
        let entries = std::fs::read_dir(&self.root)
            .with_context(|| format!("reading {}", self.root.display()))?;
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "yml"))
            .collect();
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let bills: Vec<LegislatureBill> = serde_yaml::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))?;
            out.extend(bills);
        }
        Ok(out)
    }
}

/// The network source. Not implemented in this environment.
///
/// Left as an explicit failure rather than a silent fallback to fixtures: a connector that
/// quietly serves synthetic data when the network is unavailable would put fabricated figures
/// into an extraction run that believes it fetched them.
pub struct HttpSource {
    pub base: String,
}

impl Source for HttpSource {
    fn fetch(&self, bill: &BillRef) -> Result<LegislatureBill> {
        bail!(
            "HttpSource is not implemented; would fetch {}. Use FixtureSource for offline work.",
            document_url(&self.base, bill)
        )
    }
    fn fetch_all(&self) -> Result<Vec<LegislatureBill>> {
        bail!("HttpSource is not implemented")
    }
}

/// Which corpus `bill-version` node a fetched version corresponds to.
///
/// Returned rather than written, so that reconciling a fetch against the corpus is a
/// reviewable proposal instead of a side effect.
pub fn version_node_slug(bill: &BillRef, stage: BillStage) -> String {
    let suffix = match stage {
        BillStage::AsIntroduced => "as-introduced",
        BillStage::AsPassedHouse => "as-passed-house",
        BillStage::AsPassedSenate => "as-passed-senate",
        BillStage::ConferenceReport => "conference-report",
        BillStage::AsEnacted => "as-enacted",
        BillStage::PostVeto => "post-veto",
    };
    format!("{}-{}", bill.slug(), suffix)
}

/// Stages present in a fetched record that the corpus does not yet hold a node for.
pub fn missing_version_slugs(
    bill: &BillRef,
    fetched: &LegislatureBill,
    have: &[String],
) -> Vec<String> {
    fetched
        .versions
        .iter()
        .map(|v| version_node_slug(bill, v.stage))
        .filter(|slug| !have.contains(slug))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn bill_slug_drops_whitespace_and_lowercases() {
        assert_eq!(BillRef::new("136th", "HB 96").slug(), "hb96");
        assert_eq!(BillRef::new("136th", "HB 96").corpus_slug(), "hb96-136th");
        // Distinct assemblies must not collide — the whole reason BillRef carries both.
        assert_ne!(
            BillRef::new("136th", "HB 96").corpus_slug(),
            BillRef::new("128th", "HB 96").corpus_slug()
        );
    }

    #[test]
    fn stage_vocabulary_normalizes_variants() {
        for raw in ["As Introduced", "as introduced", "As Filed"] {
            assert_eq!(parse_stage(raw), Some(BillStage::AsIntroduced), "{raw}");
        }
        for raw in ["As Passed by the House", "as-passed-house", "House Passed"] {
            assert_eq!(parse_stage(raw), Some(BillStage::AsPassedHouse), "{raw}");
        }
        assert_eq!(
            parse_stage("Conference Report"),
            Some(BillStage::ConferenceReport)
        );
        assert_eq!(
            parse_stage("As Signed by the Governor"),
            Some(BillStage::AsEnacted)
        );
        assert_eq!(parse_stage("something else entirely"), None);
    }

    #[test]
    fn unknown_stage_is_none_rather_than_a_guess() {
        // A stage the vocabulary does not cover must surface, not silently become
        // AsIntroduced and put figures on the wrong node.
        assert_eq!(parse_stage(""), None);
        assert_eq!(parse_stage("As Reported by House Ways and Means"), None);
    }

    #[test]
    fn document_url_is_built_from_the_pair() {
        let url = document_url("https://example.invalid/", &BillRef::new("136th", "HB 96"));
        assert_eq!(url, "https://example.invalid/legislation/136/hb96");
    }

    #[test]
    fn fixture_source_reads_committed_records() {
        let src = FixtureSource::from_repo(repo_root());
        let bill = src.fetch(&BillRef::new("136th", "HB 96")).unwrap();
        assert_eq!(bill.versions.len(), 3);
        assert_eq!(bill.instrument_type, "operating");
    }

    #[test]
    fn fixture_source_errors_on_an_unknown_bill() {
        let src = FixtureSource::from_repo(repo_root());
        assert!(src.fetch(&BillRef::new("999th", "HB 1")).is_err());
    }

    #[test]
    fn http_source_fails_loudly_rather_than_falling_back() {
        // A connector that silently served fixtures on network failure would put synthetic
        // figures into a run that believed it fetched them.
        let src = HttpSource {
            base: "https://example.invalid".into(),
        };
        let err = src.fetch(&BillRef::new("136th", "HB 96")).unwrap_err();
        assert!(err.to_string().contains("not implemented"));
    }

    #[test]
    fn missing_versions_are_reported_against_what_the_corpus_holds() {
        let src = FixtureSource::from_repo(repo_root());
        let r = BillRef::new("136th", "HB 96");
        let bill = src.fetch(&r).unwrap();
        let have = vec![
            "hb96-as-introduced".to_string(),
            "hb96-as-enacted".to_string(),
        ];
        let missing = missing_version_slugs(&r, &bill, &have);
        assert_eq!(missing, vec!["hb96-as-passed-house"]);
    }
}
