//! Emits the corpus, the catalog, the decisions, and the calculators' results as a JSON feed.
//!
//! # Why the feed exists
//!
//! The web layer needs the same facts the domain computer holds, and there are only two ways
//! to give them to it: hand it the corpus and let it re-derive, or hand it derived results.
//! This crate chooses the second, because the derivations are not incidental. Whether a gap
//! may be subtracted at all, whether a stage delta may be attributed to an actor, how a
//! variance on a formula-driven line should be read — those are judgments the calculators
//! make under rules recorded in [`.yidam/skills/`](../../../.yidam/skills/). A TypeScript
//! reimplementation of them would be a second set of rules, and the two would drift silently
//! in the direction of whichever was easier to write.
//!
//! So the feed carries *answers*, including the refusals. A refusal is content: it says the
//! corpus holds both sides of a subtraction and the calculator declined it anyway, which is a
//! different and more interesting statement than having no data.
//!
//! # Money never crosses the boundary as a string
//!
//! Every figure is emitted in whole cents as an integer, parsed by
//! [`lsc::parse_money_to_cents`]. The web layer never parses a dollar string, because that
//! would be a second money parser, and money parsers disagree at exactly the inputs that
//! matter.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use corpus_schema::{CatalogEntry, ClaimTag, DecisionRecord, EdgeDef, PropertyDef};
use corpus_validate::{normalize_join, Corpus, LoadedInstance};
use serde::Serialize;

/// Bumped only on a breaking change: a removed or renamed field, a changed type, a removed
/// file. Adding a field or a file is non-breaking, and consumers must ignore what they do
/// not recognize.
pub const CONTRACT_VERSION: &str = "1";

/// Properties whose text is a dollar figure.
///
/// An allowlist rather than "parse whatever parses", because plenty of non-money text parses
/// cleanly as money and produces a confident wrong number — `current_code: "200550"` becomes
/// $2,005.50 without complaint.
const MONEY_PROPERTIES: [&str; 5] = [
    "amount",
    "amount_delta",
    "projected_amount",
    "reversion",
    "variance_from_actual",
];

fn slug_of(rel_path: &str) -> String {
    rel_path
        .rsplit('/')
        .next()
        .unwrap_or(rel_path)
        .trim_end_matches(".yml")
        .trim_end_matches(".md")
        .to_string()
}

// ─── the feed ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Manifest {
    pub contract_version: &'static str,
    /// Short SHA of HEAD, or `null` outside a git worktree.
    pub commit: Option<String>,
    pub counts: Counts,
    pub real_dollars: RealDollars,
}

/// Whether anything in this feed may be compared across fiscal periods.
///
/// [`real_dollars`] ships no deflator on purpose — which index to use is a modeling decision,
/// and a general price index and a state-and-local-purchases index give materially different
/// answers for a budget series. Nothing in the corpus supplies one either, so this export
/// constructs none, and every cross-period comparison downstream is nominal.
///
/// This travels in the manifest because the consumer most likely to get it wrong is a chart.
/// The skill names the failure directly: a chart mixing nominal and real points looks fine.
#[derive(Debug, Clone, Serialize)]
pub struct RealDollars {
    pub deflator_available: bool,
    pub reason: &'static str,
}

impl Default for RealDollars {
    fn default() -> Self {
        Self {
            deflator_available: false,
            // Plain prose, no markdown: this string is read by a CLI, a JSON consumer, and
            // a web page that renders it verbatim, and only one of those would style it.
            reason: "No deflator is supplied. The real-dollars calculator ships none by \
                     design — which price index to use is a modeling decision, not a \
                     technical detail — and the corpus seeds no index either. Figures from \
                     different fiscal periods are therefore nominal, and must not be drawn \
                     as a trend.",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Counts {
    pub classes: usize,
    pub nodes: usize,
    pub catalog: usize,
    pub decisions: usize,
    pub skills: usize,
    /// Nodes carrying at least one `[verified]` claim.
    pub verified_nodes: usize,
    /// Nodes carrying at least one `[open]` claim — the repository's own question list.
    pub open_nodes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassView {
    pub class: String,
    pub label: String,
    pub description: String,
    pub foundational_type: Option<String>,
    pub properties: Vec<PropertyDef>,
    pub edges: Vec<EdgeDef>,
    pub instance_count: usize,
}

/// What a link points at. `Unresolved` is emitted rather than dropped — a dangling edge is a
/// fact about the corpus, and hiding it would make the web layer look healthier than the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    Node,
    Class,
    Catalog,
    Unresolved,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkView {
    pub relationship: String,
    pub kind: TargetKind,
    /// Slug of the target, for every kind but `unresolved`.
    pub slug: Option<String>,
    /// Class of the target node; absent for catalog and class targets.
    pub class: Option<String>,
    /// The raw `target:` as written, always, so an unresolved edge is still reportable.
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklinkView {
    pub relationship: String,
    pub slug: String,
    pub class: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeView {
    pub slug: String,
    pub class: String,
    pub label: String,
    pub description: String,
    /// Repository-relative path, so a reader can go and check the file.
    pub path: String,
    pub properties: BTreeMap<String, String>,
    /// Money properties in whole cents. Present only where the text parsed and was not
    /// `[open]`; absence means "not stated", never zero.
    pub money_cents: BTreeMap<String, i64>,
    pub links: Vec<LinkView>,
    pub backlinks: Vec<BacklinkView>,
    /// Claim tags appearing anywhere in the description or the property text.
    pub claims: Vec<ClaimTag>,
    /// A node whose label opens with `?` — the corpus's way of writing down a question.
    pub is_open_question: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogView {
    pub slug: String,
    pub path: String,
    /// `Err` carries the parse failure, so a malformed entry is visible rather than missing.
    #[serde(flatten)]
    pub entry: CatalogState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum CatalogState {
    Parsed(Box<CatalogEntry>),
    Malformed { error: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillView {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub path: String,
}

/// What the calculators say, and where they decline to say anything.
#[derive(Debug, Clone, Serialize)]
pub struct Findings {
    pub gap: Vec<gap::Coverage>,
    pub stage_delta: Vec<stage_delta::Decomposition>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Feed {
    pub manifest: Manifest,
    pub classes: Vec<ClassView>,
    pub nodes: Vec<NodeView>,
    pub catalog: Vec<CatalogView>,
    pub decisions: Vec<DecisionRecord>,
    pub skills: Vec<SkillView>,
    pub findings: Findings,
}

// ─── building it ─────────────────────────────────────────────────────────────

fn head_commit(repo_root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Text of every property, joined — the surface a claim marker can appear on besides the body.
fn property_text_blob(inst: &LoadedInstance) -> String {
    inst.inst
        .properties
        .values()
        .filter_map(|v| v.as_text())
        .collect::<Vec<_>>()
        .join("\n")
}

fn link_view(corpus: &Corpus, from: &LoadedInstance, raw: &str, relationship: &str) -> LinkView {
    let dir = from.abs_path.parent().unwrap_or(&from.abs_path);
    let target = normalize_join(dir, raw);

    if let Some(inst) = corpus.instances.iter().find(|i| i.abs_path == target) {
        return LinkView {
            relationship: relationship.to_string(),
            kind: TargetKind::Node,
            slug: Some(slug_of(&inst.rel_path)),
            class: Some(inst.inst.class.clone()),
            raw: raw.to_string(),
        };
    }
    if corpus.class_paths.contains(&target) {
        return LinkView {
            relationship: relationship.to_string(),
            kind: TargetKind::Class,
            slug: target
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.trim_end_matches(".ont.yml").to_string()),
            class: None,
            raw: raw.to_string(),
        };
    }
    if corpus.catalog_paths.contains(&target) {
        return LinkView {
            relationship: relationship.to_string(),
            kind: TargetKind::Catalog,
            slug: target
                .file_stem()
                .and_then(|n| n.to_str())
                .map(str::to_string),
            class: None,
            raw: raw.to_string(),
        };
    }
    LinkView {
        relationship: relationship.to_string(),
        kind: TargetKind::Unresolved,
        slug: None,
        class: None,
        raw: raw.to_string(),
    }
}

fn node_views(corpus: &Corpus) -> Vec<NodeView> {
    let mut nodes: Vec<NodeView> = corpus
        .instances
        .iter()
        .map(|i| {
            let claims = ClaimTag::scan(&format!(
                "{}\n{}",
                i.inst.description,
                property_text_blob(i)
            ));
            let mut properties = BTreeMap::new();
            let mut money_cents = BTreeMap::new();
            for (k, v) in &i.inst.properties {
                let text = match v.as_text() {
                    Some(t) => t.to_string(),
                    None => serde_json::to_string(v).unwrap_or_default(),
                };
                if MONEY_PROPERTIES.contains(&k.as_str()) && !text.contains("[open]") {
                    if let Ok(cents) = lsc::parse_money_to_cents(&text) {
                        money_cents.insert(k.clone(), cents);
                    }
                }
                properties.insert(k.clone(), text);
            }

            NodeView {
                slug: slug_of(&i.rel_path),
                class: i.inst.class.clone(),
                label: i.inst.label.clone(),
                description: i.inst.description.clone(),
                path: i.rel_path.clone(),
                properties,
                money_cents,
                links: i
                    .inst
                    .links
                    .iter()
                    .map(|l| link_view(corpus, i, &l.target, &l.relationship))
                    .collect(),
                backlinks: Vec::new(),
                is_open_question: i.inst.label.trim_start().starts_with('?'),
                claims,
            }
        })
        .collect();

    // Invert the edge set. Done here rather than in the web layer because the resolution
    // above is the expensive half and it has already been paid for.
    let index: BTreeMap<String, (String, String)> = nodes
        .iter()
        .map(|n| (n.slug.clone(), (n.class.clone(), n.label.clone())))
        .collect();
    let mut inbound: BTreeMap<String, Vec<BacklinkView>> = BTreeMap::new();
    for n in &nodes {
        for l in &n.links {
            if l.kind != TargetKind::Node {
                continue;
            }
            let Some(target) = l.slug.as_ref() else {
                continue;
            };
            let Some((class, label)) = index.get(&n.slug) else {
                continue;
            };
            inbound
                .entry(target.clone())
                .or_default()
                .push(BacklinkView {
                    relationship: l.relationship.clone(),
                    slug: n.slug.clone(),
                    class: class.clone(),
                    label: label.clone(),
                });
        }
    }
    for n in &mut nodes {
        if let Some(mut b) = inbound.remove(&n.slug) {
            b.sort_by(|x, y| (&x.class, &x.slug).cmp(&(&y.class, &y.slug)));
            n.backlinks = b;
        }
    }

    nodes.sort_by(|a, b| (&a.class, &a.slug).cmp(&(&b.class, &b.slug)));
    nodes
}

fn load_decisions(repo_root: &Path) -> Result<Vec<DecisionRecord>> {
    let dir = repo_root.join(".yidam").join("decisions");
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yml"))
        .collect();
    paths.sort();
    for path in paths {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        // `proposals.yml` holds a list of proposals, not a decision record; skip what does
        // not parse rather than failing the export over a file shape the class never claimed.
        if let Ok(rec) = serde_yaml::from_str::<DecisionRecord>(&text) {
            out.push(rec);
        }
    }
    Ok(out)
}

fn load_skills(repo_root: &Path) -> Result<Vec<SkillView>> {
    let dir = repo_root.join(".yidam").join("skills");
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("md")
                && p.file_name().and_then(|n| n.to_str()) != Some("README.md")
        })
        .collect();
    paths.sort();

    #[derive(serde::Deserialize)]
    struct Frontmatter {
        name: String,
        description: String,
    }

    for path in paths {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let Some(fm) = corpus_validate::split_frontmatter(&text) else {
            continue;
        };
        let Ok(parsed) = serde_yaml::from_str::<Frontmatter>(fm) else {
            continue;
        };
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        out.push(SkillView {
            slug: slug_of(&rel),
            name: parsed.name,
            description: parsed.description,
            path: rel,
        });
    }
    Ok(out)
}

/// Reads the repository and returns everything the web layer is given.
pub fn build(repo_root: &Path) -> Result<Feed> {
    let corpus = corpus_validate::load(repo_root)?;
    let nodes = node_views(&corpus);

    let mut instances_per_class: BTreeMap<&str, usize> = BTreeMap::new();
    for n in &nodes {
        *instances_per_class.entry(n.class.as_str()).or_default() += 1;
    }

    let classes: Vec<ClassView> = corpus
        .classes
        .values()
        .map(|c| ClassView {
            instance_count: instances_per_class
                .get(c.class.as_str())
                .copied()
                .unwrap_or(0),
            class: c.class.clone(),
            label: c.label.clone(),
            description: c.description.clone(),
            foundational_type: c.foundational_type.as_ref().map(|f| f.type_.clone()),
            properties: c.properties.clone(),
            edges: c.edges.clone(),
        })
        .collect();

    let catalog: Vec<CatalogView> = corpus
        .catalog
        .iter()
        .map(|c| CatalogView {
            slug: c.file_slug.clone(),
            path: c.rel_path.clone(),
            entry: match &c.parsed {
                Ok(e) => CatalogState::Parsed(Box::new(e.clone())),
                Err(error) => CatalogState::Malformed {
                    error: error.clone(),
                },
            },
        })
        .collect();

    let decisions = load_decisions(repo_root)?;
    let skills = load_skills(repo_root)?;

    let counts = Counts {
        classes: classes.len(),
        nodes: nodes.len(),
        catalog: catalog.len(),
        decisions: decisions.len(),
        skills: skills.len(),
        verified_nodes: nodes
            .iter()
            .filter(|n| n.claims.contains(&ClaimTag::Verified))
            .count(),
        open_nodes: nodes
            .iter()
            .filter(|n| n.claims.contains(&ClaimTag::Open))
            .count(),
    };

    Ok(Feed {
        manifest: Manifest {
            contract_version: CONTRACT_VERSION,
            commit: head_commit(repo_root),
            counts,
            real_dollars: RealDollars::default(),
        },
        classes,
        nodes,
        catalog,
        decisions,
        skills,
        findings: Findings {
            gap: gap::all(repo_root)?,
            stage_delta: stage_delta::all(repo_root)?,
        },
    })
}

/// Writes the feed as one JSON file per section.
///
/// Split rather than emitted whole so the wiki pages do not have to load the findings and the
/// report pages do not have to load 236 nodes.
pub fn write(feed: &Feed, out_dir: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let sections: [(&str, serde_json::Value); 6] = [
        ("manifest", serde_json::to_value(&feed.manifest)?),
        (
            "corpus",
            serde_json::json!({ "classes": feed.classes, "nodes": feed.nodes }),
        ),
        ("catalog", serde_json::to_value(&feed.catalog)?),
        ("decisions", serde_json::to_value(&feed.decisions)?),
        ("skills", serde_json::to_value(&feed.skills)?),
        ("findings", serde_json::to_value(&feed.findings)?),
    ];

    let mut written = Vec::new();
    for (name, value) in sections {
        let path = out_dir.join(format!("{name}.json"));
        let mut bytes = serde_json::to_vec_pretty(&value)?;
        bytes.push(b'\n');
        std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpus_schema::CorpusInstance;

    fn inst(rel: &str, yaml: &str) -> LoadedInstance {
        LoadedInstance {
            rel_path: rel.to_string(),
            abs_path: PathBuf::from(rel),
            inst: serde_yaml::from_str::<CorpusInstance>(yaml).unwrap(),
        }
    }

    /// Two nodes, one pointing at the other, plus a class file and a catalog entry.
    fn small_corpus() -> Corpus {
        let mut c = Corpus {
            instances: vec![
                inst(
                    ".yidam/corpus/line-item/foundation-funding.yml",
                    "class: line-item\nlabel: Foundation Funding\n\
                     description: |\n  Its code is 200550. [verified]\n  \
                     [open] Whether it survived is unresolved.\nproperties:\n  \
                     current_code: \"200550\"\n",
                ),
                inst(
                    ".yidam/corpus/appropriation/ff-fy2026.yml",
                    "class: appropriation\nlabel: FF FY2026\ndescription: d\nproperties:\n  \
                     amount: \"$8,457,598,772.00\"\n  reversion: \"[open] not reported\"\n\
                     links:\n  - target: ../line-item/foundation-funding.yml\n    \
                     relationship: grants-authority-for\n  \
                     - target: ../../catalog/lsc-hb96-appropriation-spreadsheet.md\n    \
                     relationship: cites\n  - target: ../appropriation.ont.yml\n    \
                     relationship: instance-of\n  - target: ../fund/nowhere.yml\n    \
                     relationship: draws-from\n",
                ),
            ],
            ..Default::default()
        };
        c.class_paths
            .insert(PathBuf::from(".yidam/corpus/appropriation.ont.yml"));
        c.catalog_paths.insert(PathBuf::from(
            ".yidam/catalog/lsc-hb96-appropriation-spreadsheet.md",
        ));
        c
    }

    fn node<'a>(nodes: &'a [NodeView], slug: &str) -> &'a NodeView {
        nodes.iter().find(|n| n.slug == slug).expect(slug)
    }

    #[test]
    fn a_link_to_another_node_resolves_to_its_class_and_slug() {
        let nodes = node_views(&small_corpus());
        let l = node(&nodes, "ff-fy2026")
            .links
            .iter()
            .find(|l| l.relationship == "grants-authority-for")
            .unwrap();
        assert_eq!(l.kind, TargetKind::Node);
        assert_eq!(l.slug.as_deref(), Some("foundation-funding"));
        assert_eq!(l.class.as_deref(), Some("line-item"));
    }

    #[test]
    fn class_and_catalog_targets_are_distinguished_from_nodes() {
        let nodes = node_views(&small_corpus());
        let kinds: BTreeMap<&str, TargetKind> = node(&nodes, "ff-fy2026")
            .links
            .iter()
            .map(|l| (l.relationship.as_str(), l.kind))
            .collect();
        assert_eq!(kinds["instance-of"], TargetKind::Class);
        assert_eq!(kinds["cites"], TargetKind::Catalog);
    }

    #[test]
    fn a_dangling_edge_is_reported_not_dropped() {
        // Hiding it would make the web layer look healthier than the graph actually is.
        let nodes = node_views(&small_corpus());
        let l = node(&nodes, "ff-fy2026")
            .links
            .iter()
            .find(|l| l.relationship == "draws-from")
            .unwrap();
        assert_eq!(l.kind, TargetKind::Unresolved);
        assert_eq!(l.raw, "../fund/nowhere.yml");
        assert!(l.slug.is_none());
    }

    #[test]
    fn backlinks_are_inverted_onto_the_target() {
        let nodes = node_views(&small_corpus());
        let b = &node(&nodes, "foundation-funding").backlinks;
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].slug, "ff-fy2026");
        assert_eq!(b[0].relationship, "grants-authority-for");
        assert_eq!(b[0].class, "appropriation");
    }

    #[test]
    fn money_is_emitted_in_cents() {
        let nodes = node_views(&small_corpus());
        assert_eq!(
            node(&nodes, "ff-fy2026").money_cents.get("amount"),
            Some(&845_759_877_200)
        );
    }

    #[test]
    fn an_open_money_property_is_absent_rather_than_zero() {
        let nodes = node_views(&small_corpus());
        // Absence means "not stated". A zero here would assert that nothing reverted.
        assert!(!node(&nodes, "ff-fy2026")
            .money_cents
            .contains_key("reversion"));
    }

    #[test]
    fn a_code_that_parses_as_money_is_not_treated_as_money() {
        // `current_code: "200550"` parses cleanly to $2,005.50, which is why the allowlist
        // exists rather than a try-to-parse-everything rule.
        let nodes = node_views(&small_corpus());
        assert!(node(&nodes, "foundation-funding").money_cents.is_empty());
    }

    #[test]
    fn claims_are_scanned_from_the_body() {
        let nodes = node_views(&small_corpus());
        assert_eq!(
            node(&nodes, "foundation-funding").claims,
            vec![ClaimTag::Verified, ClaimTag::Open]
        );
    }
}
