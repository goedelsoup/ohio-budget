//! Typed definitions for every file shape in this repository's corpus, plus the
//! extraction records connectors must produce before their output may become corpus nodes.
//!
//! These types are the single source of truth. The JSON Schemas under `.yidam/schemas/`
//! are generated from them by the `emit-schemas` binary and must never be hand-edited —
//! a hand-maintained second copy drifts, and drift in a validation schema is worse than
//! no schema, because it fails closed on correct data and open on incorrect data.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ─── shared ──────────────────────────────────────────────────────────────────

/// A YAML scalar as it appears in a corpus instance's `properties` map.
///
/// Ordering is load-bearing: serde tries these variants in declaration order, so `Int`
/// must precede `Float` (otherwise every integer deserializes as a float) and `Text`
/// must come last (otherwise it swallows everything).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ScalarValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl ScalarValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ScalarValue::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self, ScalarValue::Float(_))
    }
}

/// Confidence tag carried inline by a claim in a node body.
///
/// The prelude's conduct norms require every non-obvious claim to carry one unless the
/// node is a direct transcription of a primary source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ClaimTag {
    /// Supported by a committed primary source linked from this node or its catalog entry.
    Verified,
    /// A reasonable conclusion drawn from verified facts; not directly witnessed.
    Inference,
    /// A live question — unknown, contested, or under investigation.
    Open,
}

impl ClaimTag {
    pub const ALL: [ClaimTag; 3] = [ClaimTag::Verified, ClaimTag::Inference, ClaimTag::Open];

    pub fn marker(&self) -> &'static str {
        match self {
            ClaimTag::Verified => "[verified]",
            ClaimTag::Inference => "[inference]",
            ClaimTag::Open => "[open]",
        }
    }

    pub fn parse_marker(s: &str) -> Option<ClaimTag> {
        match s {
            "[verified]" => Some(ClaimTag::Verified),
            "[inference]" => Some(ClaimTag::Inference),
            "[open]" => Some(ClaimTag::Open),
            _ => None,
        }
    }
}

/// Where an extracted value came from, carried on every extraction record.
///
/// Without this a value cannot be promoted from `[inference]` to `[verified]`, since
/// verification is defined as resting on a committed, citable source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provenance {
    /// Slug of the catalog entry describing the source, e.g. `lsc-hb96-comparison`.
    pub catalog_slug: String,
    /// Stable locator for the document itself.
    pub document_ref: String,
    /// Where inside the document — page, table, or row identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    /// ISO-8601 date the source was retrieved.
    pub retrieved: String,
}

// ─── corpus: class definitions (`<class>.ont.yml`) ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Out,
    In,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PropertyType {
    String,
    Date,
    Ref,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FoundationalOntology {
    Bfo,
    Ufo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FoundationalType {
    pub ontology: FoundationalOntology,
    /// UFO: kind | subkind | role | phase | relator | mode | quality | event | situation.
    /// BFO: continuant | occurrent | quality | disposition | role | ...
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PropertyDef {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: PropertyType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeDef {
    pub relationship: String,
    /// Name of the class on the other end.
    pub target: String,
    pub direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A `<class>.ont.yml` file — the schema layer of the corpus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClassDefinition {
    pub class: String,
    pub label: String,
    /// Omitted entirely when the corpus alignment is "none".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foundational_type: Option<FoundationalType>,
    pub description: String,
    #[serde(default)]
    pub properties: Vec<PropertyDef>,
    #[serde(default)]
    pub edges: Vec<EdgeDef>,
}

impl ClassDefinition {
    /// Relationships this class may assert *from* an instance of itself.
    pub fn outgoing(&self) -> impl Iterator<Item = &EdgeDef> {
        self.edges.iter().filter(|e| e.direction == Direction::Out)
    }

    pub fn declares_outgoing(&self, relationship: &str, target_class: &str) -> bool {
        self.outgoing()
            .any(|e| e.relationship == relationship && e.target == target_class)
    }

    pub fn property_names(&self) -> Vec<&str> {
        self.properties.iter().map(|p| p.name.as_str()).collect()
    }
}

// ─── corpus: instances (`<class>/<instance>.yml`) ────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Link {
    /// Repository-relative-to-this-file path, e.g. `../fund/general-revenue-fund.yml`.
    pub target: String,
    pub relationship: String,
}

/// A concrete object of some class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CorpusInstance {
    pub class: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub properties: BTreeMap<String, ScalarValue>,
    #[serde(default)]
    pub links: Vec<Link>,
}

impl CorpusInstance {
    /// The `instance-of` link is structural bookkeeping, not a domain edge. Distinguishing
    /// them is what lets the validator tell a genuinely connected node from one whose only
    /// link is to its own class file.
    pub const INSTANCE_OF: &'static str = "instance-of";

    pub fn domain_links(&self) -> impl Iterator<Item = &Link> {
        self.links
            .iter()
            .filter(|l| l.relationship != Self::INSTANCE_OF)
    }
}

// ─── corpus: decision records (`.yidam/decisions/<id>.yml`) ──────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DecisionRecord {
    pub id: String,
    pub summary: String,
    /// Present only on the ontology decision; target instance count for initial seeding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpus_depth: Option<u32>,
    pub context: String,
    pub decision: String,
    pub rationale: String,
}

// ─── catalog entries (`.yidam/catalog/<slug>.md` frontmatter) ────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    Statute,
    LegislativeDocument,
    FinancialReport,
    Dataset,
    Api,
    Minutes,
    Paper,
    Other,
}

/// Frontmatter of a catalog entry. The body carries the human description.
///
/// A catalog entry describes a source; it does not itself verify anything. A claim may
/// only be tagged `[verified]` once the source content is committed, not merely once the
/// source is registered here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CatalogEntry {
    pub slug: String,
    pub name: String,
    pub source_type: SourceType,
    /// URL, citation, or access method.
    pub location: String,
    pub publisher: String,
    /// Whether the source content itself has been committed to this repository, as
    /// opposed to the source merely being registered.
    pub content_committed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_constraints: Option<String>,
    /// Corpus classes this source can supply values for.
    #[serde(default)]
    pub feeds: Vec<String>,
}

// ─── extraction records: connector output contracts ──────────────────────────

/// Money is carried in whole cents as a signed integer, never as a float.
///
/// A binary floating point value cannot represent most decimal cent amounts exactly, and
/// budget figures are summed across thousands of line items — the error compounds and is
/// silent. The validator rejects float-typed money anywhere it appears.
pub type Cents = i64;

/// Stages an Ohio appropriation bill passes through.
///
/// Genesis modelled six. The LSC appropriation spreadsheet publishes nine, distinguishing a
/// chamber's substitute bill, its committee-reported version, and its floor-passed version —
/// which are frequently different figures. The four intermediate variants were added after
/// reading the source rather than reasoned about in advance, and dropping them would discard
/// exactly the resolution that makes stage attribution possible.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum BillStage {
    AsIntroduced,
    HouseSubstitute,
    HouseReported,
    AsPassedHouse,
    SenateSubstitute,
    SenateReported,
    AsPassedSenate,
    ConferenceReport,
    AsEnacted,
    PostVeto,
}

impl BillStage {
    /// Corpus spelling, matching the `stage` property on bill-version and appropriation.
    pub fn as_str(&self) -> &'static str {
        match self {
            BillStage::AsIntroduced => "as-introduced",
            BillStage::HouseSubstitute => "house-substitute",
            BillStage::HouseReported => "house-reported",
            BillStage::AsPassedHouse => "as-passed-house",
            BillStage::SenateSubstitute => "senate-substitute",
            BillStage::SenateReported => "senate-reported",
            BillStage::AsPassedSenate => "as-passed-senate",
            BillStage::ConferenceReport => "conference-report",
            BillStage::AsEnacted => "as-enacted",
            BillStage::PostVeto => "post-veto",
        }
    }

    /// Parses a published stage label onto the corpus vocabulary.
    ///
    /// Both the General Assembly's record and LSC's spreadsheet headers name these stages,
    /// in several spellings each. Keeping the vocabulary here means a source that invents a
    /// new spelling is fixed once rather than in every connector.
    ///
    /// Returns `None` rather than guessing: a stage guessed wrong puts figures on the wrong
    /// node, which is worse than a column the caller has to classify by hand.
    pub fn parse_label(raw: &str) -> Option<BillStage> {
        let n: String = raw
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let n = n.split_whitespace().collect::<Vec<_>>().join(" ");
        match n.as_str() {
            "as introduced" | "introduced" | "as filed" => Some(BillStage::AsIntroduced),
            "house substitute" | "substitute house bill" | "sub house" => {
                Some(BillStage::HouseSubstitute)
            }
            "house reported" | "as reported by house finance" => Some(BillStage::HouseReported),
            "as passed by the house" | "as passed house" | "house passed" => {
                Some(BillStage::AsPassedHouse)
            }
            "senate substitute" | "sub senate" => Some(BillStage::SenateSubstitute),
            "senate reported" | "as reported by senate finance" => Some(BillStage::SenateReported),
            "as passed by the senate" | "as passed senate" | "senate passed" => {
                Some(BillStage::AsPassedSenate)
            }
            "conference report" | "as reported by conference committee" => {
                Some(BillStage::ConferenceReport)
            }
            "as enacted" | "enacted" | "as signed by the governor" | "final" => {
                Some(BillStage::AsEnacted)
            }
            "post veto" | "as vetoed" | "after line item veto" => Some(BillStage::PostVeto),
            _ => None,
        }
    }

    /// Order in which a bill passes through them. Declaration order is the sequence.
    pub fn sequence() -> [BillStage; 10] {
        [
            BillStage::AsIntroduced,
            BillStage::HouseSubstitute,
            BillStage::HouseReported,
            BillStage::AsPassedHouse,
            BillStage::SenateSubstitute,
            BillStage::SenateReported,
            BillStage::AsPassedSenate,
            BillStage::ConferenceReport,
            BillStage::AsEnacted,
            BillStage::PostVeto,
        ]
    }
}

/// One appropriation figure as published in a Legislative Service Commission comparison
/// document. The load-bearing extraction record: every `[open]` amount in the corpus is
/// waiting on one of these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LscComparisonRow {
    pub bill_number: String,
    pub general_assembly: String,
    pub stage: BillStage,
    pub agency_code: String,
    pub line_item_code: String,
    pub line_item_name: String,
    pub fund_group: String,
    pub fund_code: String,
    /// Fiscal year the figure applies to, e.g. `FY2026`.
    pub fiscal_year: String,
    pub amount_cents: Cents,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ExpenditureBasis {
    /// In-year reporting; provisional.
    Disbursed,
    /// From the closed books; final.
    ActualClosed,
}

/// One disbursement figure from Office of Budget and Management reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ObmExpenditureRow {
    pub agency_code: String,
    pub line_item_code: String,
    pub fund_code: String,
    pub fiscal_year: String,
    /// Month within the fiscal year, 1-12, where the report is monthly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_month: Option<u8>,
    pub basis: ExpenditureBasis,
    pub amount_cents: Cents,
    /// Authority that lapsed unspent, where the report states it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversion_cents: Option<Cents>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ControllingBoardDisposition {
    Approved,
    Denied,
    Withdrawn,
    Tabled,
}

/// One request acted on by the Controlling Board — the execution-phase record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ControllingBoardRequest {
    pub request_id: String,
    /// ISO-8601 meeting date.
    pub meeting_date: String,
    pub agency_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_item_code: Option<String>,
    pub amount_delta_cents: Cents,
    pub disposition: ControllingBoardDisposition,
    /// The reason the agency gave, quoted or closely summarized. Not an inferred motive.
    pub stated_justification: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LegislatureBillVersion {
    pub stage: BillStage,
    /// ISO-8601 date the version was produced or acted on.
    pub stage_date: String,
    pub chamber: String,
    pub document_ref: String,
}

/// Bill metadata and stage history from the General Assembly's published record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LegislatureBill {
    pub bill_number: String,
    pub general_assembly: String,
    pub instrument_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub introduced_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enacted_date: Option<String>,
    #[serde(default)]
    pub versions: Vec<LegislatureBillVersion>,
    pub provenance: Provenance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_int_does_not_become_float() {
        let v: ScalarValue = serde_yaml::from_str("42").unwrap();
        assert_eq!(v, ScalarValue::Int(42));
        assert!(!v.is_float());
    }

    #[test]
    fn scalar_float_is_detected() {
        let v: ScalarValue = serde_yaml::from_str("5.75").unwrap();
        assert!(
            v.is_float(),
            "5.75 must round-trip as Float so money checks can reject it"
        );
    }

    #[test]
    fn scalar_open_marker_is_text() {
        let v: ScalarValue = serde_yaml::from_str("\"[open] pending verification\"").unwrap();
        assert_eq!(v.as_text(), Some("[open] pending verification"));
    }

    #[test]
    fn claim_tag_markers_round_trip() {
        for tag in ClaimTag::ALL {
            assert_eq!(ClaimTag::parse_marker(tag.marker()), Some(tag));
        }
        assert_eq!(ClaimTag::parse_marker("[probably]"), None);
    }

    #[test]
    fn class_definition_declares_only_outgoing_edges() {
        let yaml = r#"
class: expenditure
label: Expenditure
description: test
edges:
  - relationship: paid-to
    target: jurisdiction
    direction: out
  - relationship: realized-by
    target: appropriation
    direction: in
"#;
        let def: ClassDefinition = serde_yaml::from_str(yaml).unwrap();
        assert!(def.declares_outgoing("paid-to", "jurisdiction"));
        // Inbound edges are declared on this class but asserted by the *other* class.
        assert!(!def.declares_outgoing("realized-by", "appropriation"));
        assert_eq!(def.outgoing().count(), 1);
    }

    #[test]
    fn instance_of_is_not_a_domain_link() {
        let yaml = r#"
class: actor
label: Governor of Ohio
description: test
links:
  - target: ../actor.ont.yml
    relationship: instance-of
  - target: ../agency/department-of-medicaid.yml
    relationship: participates-as
"#;
        let inst: CorpusInstance = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(inst.links.len(), 2);
        assert_eq!(inst.domain_links().count(), 1);
    }

    #[test]
    fn money_is_integer_cents() {
        let row = r#"
bill_number: HB 96
general_assembly: 136th
stage: as-enacted
agency_code: "200"
line_item_code: "200550"
line_item_name: Foundation Funding
fund_group: general
fund_code: "5000"
fiscal_year: FY2026
amount_cents: 812345678900
provenance:
  catalog_slug: lsc-hb96-comparison
  document_ref: "HB96-EN-comparedoc"
  retrieved: "2026-08-08"
"#;
        let parsed: LscComparisonRow = serde_yaml::from_str(row).unwrap();
        assert_eq!(parsed.amount_cents, 812_345_678_900);
        assert_eq!(parsed.stage, BillStage::AsEnacted);
    }

    #[test]
    fn extraction_record_requires_provenance() {
        let missing = r#"
bill_number: HB 96
general_assembly: 136th
stage: as-enacted
agency_code: "200"
line_item_code: "200550"
line_item_name: Foundation Funding
fund_group: general
fund_code: "5000"
fiscal_year: FY2026
amount_cents: 100
"#;
        assert!(
            serde_yaml::from_str::<LscComparisonRow>(missing).is_err(),
            "an extraction record without provenance must not parse — it could never be verified"
        );
    }
}
