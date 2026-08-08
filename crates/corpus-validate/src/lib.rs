//! Validates corpus instances against the class definitions they claim to instantiate.
//!
//! This checks what a JSON Schema cannot: that a relationship asserted by an instance is
//! actually declared by its class, pointing at the class it actually points at. During
//! this repository's bootstrap three relationships were asserted that no class declared —
//! `contains` and `precedes` on fiscal-period, and `holds` on agency — and all three were
//! caught by eye rather than by tooling. That is the gap this crate closes.
//!
//! IO lives in [`load`]; the rules in [`check`] are pure over loaded data so they can be
//! tested without a filesystem.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use corpus_schema::{CatalogEntry, ClassDefinition, CorpusInstance, Direction, ScalarValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warn,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warn => " warn",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: String,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LoadedInstance {
    /// Path relative to the repository root, for reporting.
    pub rel_path: String,
    /// Normalized absolute path, used for link resolution.
    pub abs_path: PathBuf,
    pub inst: CorpusInstance,
}

/// A catalog entry as found on disk, parsed or not.
#[derive(Debug, Clone)]
pub struct LoadedCatalog {
    pub rel_path: String,
    pub abs_path: PathBuf,
    /// Filename stem, which must equal the frontmatter slug.
    pub file_slug: String,
    pub parsed: Result<CatalogEntry, String>,
}

/// Splits YAML frontmatter from a markdown body.
pub fn split_frontmatter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Everything the rules operate over.
#[derive(Debug, Clone, Default)]
pub struct Corpus {
    pub classes: BTreeMap<String, ClassDefinition>,
    /// Resolved paths of the `<class>.ont.yml` files, so `instance-of` links can be
    /// distinguished from links to nonexistent nodes.
    pub class_paths: BTreeSet<PathBuf>,
    /// Resolved paths of catalog entries. Provenance edges point here, and they are the
    /// only links in the corpus that cross out of the class system.
    pub catalog_paths: BTreeSet<PathBuf>,
    pub catalog: Vec<LoadedCatalog>,
    pub instances: Vec<LoadedInstance>,
}

/// Resolves `base_dir/rel`, collapsing `.` and `..` lexically.
///
/// `Path::join` leaves `..` in place, and `Path` equality compares components — so a
/// joined `a/b/../c` never equals `a/c`. That is precisely the bug that makes the upstream
/// `yidam lint` orphan-in check report every cross-class link as missing.
pub fn normalize_join(base_dir: &Path, rel: &str) -> PathBuf {
    let mut out = base_dir.to_path_buf();
    for comp in Path::new(rel).components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The class a link points at, derived from its resolved path.
///
/// A link to `<class>.ont.yml` targets the class definition itself; a link to
/// `<class>/<instance>.yml` targets an instance of that class.
fn target_class_of(path: &Path) -> Option<(String, bool)> {
    let name = path.file_name()?.to_str()?;
    if let Some(stripped) = name.strip_suffix(".ont.yml") {
        return Some((stripped.to_string(), true));
    }
    let parent = path.parent()?.file_name()?.to_str()?;
    Some((parent.to_string(), false))
}

/// Provenance slug carried by every record under `.yidam/fixtures/`.
pub const FIXTURE_MARKER: &str = "synthetic-fixture";

/// The one relationship permitted from a corpus node to a catalog entry.
///
/// Provenance is cross-cutting: every class may cite a source, so declaring a `cites` edge
/// on all fourteen classes would be noise. It is special-cased here instead, and pinned to
/// a single relationship name so provenance edges stay greppable.
pub const CITES: &str = "cites";

fn is_catalog_path(path: &Path) -> bool {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        == Some("catalog")
}

/// Properties whose value is an identifier that something downstream matches on.
///
/// A claim tag belongs in prose. Inside one of these it is not merely untidy: it silently
/// breaks equality. `current_code: "200550 [verified]"` matches no extraction row carrying
/// `200550`, and the failure is invisible — the promotion tool simply reports zero matches
/// and a reader concludes the source lacks the line item.
///
/// That happened. Nine line items were verified against a real spreadsheet, the tag was
/// written into the code property, and the next extraction run matched none of them.
fn is_identifier_property(key: &str) -> bool {
    matches!(
        key,
        "current_code"
            | "agency_code"
            | "fund_number"
            | "fund_code"
            | "identifier"
            | "bill_number"
            | "request_id"
            | "line_item_code"
    )
}

fn looks_like_money(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    ["amount", "cents", "delta", "dollars", "balance", "cost"]
        .iter()
        .any(|needle| k.contains(needle))
}

/// Bracketed tokens that are legitimate outside the claim-tag vocabulary.
fn is_known_non_tag(token: &str) -> bool {
    // Markdown link text is `[label](path)`; those are handled by the caller, which only
    // passes tokens not followed by `(`.
    matches!(token, "[ ]" | "[x]")
}

/// Characters that, immediately before a `[`, mean the bracket is edge notation rather
/// than a claim tag.
///
/// Node bodies routinely describe the schema in prose as `agency →[succeeds]→ agency`.
/// Without this, every such description is reported as a malformed claim tag — which is
/// how this scanner behaved on its first run against the real corpus.
fn is_edge_notation_prefix(c: char) -> bool {
    matches!(c, '→' | '-' | '>' | '=')
}

fn scan_claim_tags(body: &str) -> Vec<String> {
    let mut unknown = Vec::new();
    let bytes: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '[' {
            if let Some(close) = bytes[i..].iter().position(|c| *c == ']') {
                let end = i + close;
                let token: String = bytes[i..=end].iter().collect();
                let followed_by_paren = bytes.get(end + 1) == Some(&'(');
                let followed_by_arrow = matches!(bytes.get(end + 1), Some('→') | Some('-'));
                let preceded_by_arrow = i
                    .checked_sub(1)
                    .and_then(|j| bytes.get(j))
                    .copied()
                    .is_some_and(is_edge_notation_prefix);
                let inner = &token[1..token.len() - 1];
                let is_tagish = !inner.is_empty()
                    && inner
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '-' || c == ' ')
                    && inner.len() <= 12;
                if !followed_by_paren
                    && !followed_by_arrow
                    && !preceded_by_arrow
                    && is_tagish
                    && !is_known_non_tag(&token)
                    && corpus_schema::ClaimTag::parse_marker(&token).is_none()
                {
                    unknown.push(token);
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    unknown
}

/// Runs every rule. Pure over loaded data.
pub fn check(corpus: &Corpus) -> Vec<Finding> {
    let Corpus {
        classes,
        class_paths: class_def_paths,
        catalog_paths,
        catalog,
        instances,
    } = corpus;
    let mut findings = Vec::new();
    let known_instances: BTreeSet<&PathBuf> = instances.iter().map(|i| &i.abs_path).collect();

    // A catalog entry whose frontmatter does not parse is not a provenance anchor — it is a
    // document that looks like one, which is worse than an absent entry.
    for c in catalog {
        match &c.parsed {
            Err(e) => findings.push(Finding {
                path: c.rel_path.clone(),
                rule: "bad-catalog-frontmatter",
                severity: Severity::Error,
                message: format!("frontmatter does not parse as a catalog entry: {e}"),
            }),
            Ok(entry) => {
                if entry.slug != c.file_slug {
                    findings.push(Finding {
                        path: c.rel_path.clone(),
                        rule: "catalog-slug-mismatch",
                        severity: Severity::Error,
                        message: format!(
                            "frontmatter slug '{}' does not match filename '{}'",
                            entry.slug, c.file_slug
                        ),
                    });
                }
                for feed in &entry.feeds {
                    if !classes.contains_key(feed) {
                        findings.push(Finding {
                            path: c.rel_path.clone(),
                            rule: "catalog-unknown-feed",
                            severity: Severity::Error,
                            message: format!("feeds '{feed}', which is not a corpus class"),
                        });
                    }
                }
            }
        }
    }

    // Incoming domain links, keyed by resolved target path. Computed with normalized
    // paths so cross-class links actually register.
    let mut incoming: BTreeMap<PathBuf, usize> = BTreeMap::new();
    for inst in instances {
        let dir = inst.abs_path.parent().unwrap_or(&inst.abs_path);
        for link in inst.inst.domain_links() {
            if link.relationship == CITES {
                continue;
            }
            *incoming
                .entry(normalize_join(dir, &link.target))
                .or_insert(0) += 1;
        }
    }

    for inst in instances {
        let p = &inst.rel_path;
        let dir = inst
            .abs_path
            .parent()
            .unwrap_or(&inst.abs_path)
            .to_path_buf();

        let Some(class) = classes.get(&inst.inst.class) else {
            findings.push(Finding {
                path: p.clone(),
                rule: "unknown-class",
                severity: Severity::Error,
                message: format!(
                    "class '{}' has no matching {}.ont.yml",
                    inst.inst.class, inst.inst.class
                ),
            });
            continue;
        };

        // The directory an instance lives in must match the class it claims.
        if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
            if dir_name != inst.inst.class {
                findings.push(Finding {
                    path: p.clone(),
                    rule: "class-directory-mismatch",
                    severity: Severity::Error,
                    message: format!(
                        "declares class '{}' but lives in directory '{dir_name}'",
                        inst.inst.class
                    ),
                });
            }
        }

        // Properties must be declared by the class.
        let declared: BTreeSet<&str> = class.property_names().into_iter().collect();
        for (key, value) in &inst.inst.properties {
            if !declared.contains(key.as_str()) {
                findings.push(Finding {
                    path: p.clone(),
                    rule: "undeclared-property",
                    severity: Severity::Error,
                    message: format!(
                        "property '{key}' is not declared by class '{}'",
                        inst.inst.class
                    ),
                });
            }
            if is_identifier_property(key) {
                if let Some(text) = value.as_text() {
                    let t = text.trim();
                    // A bare [open] placeholder is the established way to say "no value yet".
                    // A value *plus* a tag is the problem.
                    let is_placeholder = t.starts_with("[open]");
                    let tagged = ["[verified]", "[inference]", "[open]"]
                        .iter()
                        .any(|m| t.contains(m));
                    if tagged && !is_placeholder {
                        findings.push(Finding {
                            path: p.clone(),
                            rule: "claim-tag-in-identifier",
                            severity: Severity::Error,
                            message: format!(
                                "property '{key}' is an identifier and carries a claim tag; the \
                                 tag belongs in the node body, because a tag inside a value \
                                 silently breaks every downstream match on it"
                            ),
                        });
                    }
                }
            }
            if looks_like_money(key) && value.is_float() {
                findings.push(Finding {
                    path: p.clone(),
                    rule: "float-money",
                    severity: Severity::Error,
                    message: format!(
                        "property '{key}' holds a float; money must be integer cents or an explicit string"
                    ),
                });
            }
        }

        let mut has_instance_of = false;
        let mut domain_link_count = 0usize;

        for link in &inst.inst.links {
            let resolved = normalize_join(&dir, &link.target);
            let target = target_class_of(&resolved);

            if link.relationship == CorpusInstance::INSTANCE_OF {
                has_instance_of = true;
                match &target {
                    Some((cls, true)) if *cls == inst.inst.class => {}
                    _ => findings.push(Finding {
                        path: p.clone(),
                        rule: "bad-instance-of",
                        severity: Severity::Error,
                        message: format!(
                            "instance-of must point at ../{}.ont.yml, got '{}'",
                            inst.inst.class, link.target
                        ),
                    }),
                }
                continue;
            }

            // Provenance edges leave the class system, so the declaration check does not
            // apply to them. They are still checked for relationship name and resolution.
            if is_catalog_path(&resolved) || link.relationship == CITES {
                if link.relationship != CITES {
                    findings.push(Finding {
                        path: p.clone(),
                        rule: "bad-catalog-relationship",
                        severity: Severity::Error,
                        message: format!(
                            "'{}' points at a catalog entry; provenance edges must use '{CITES}'",
                            link.relationship
                        ),
                    });
                } else if !catalog_paths.contains(&resolved) {
                    findings.push(Finding {
                        path: p.clone(),
                        rule: "broken-link",
                        severity: Severity::Error,
                        message: format!("cites '{}', which is not a catalog entry", link.target),
                    });
                }
                continue;
            }

            domain_link_count += 1;

            // Link must resolve to a node that exists.
            let exists = known_instances.contains(&resolved) || class_def_paths.contains(&resolved);
            if !exists {
                findings.push(Finding {
                    path: p.clone(),
                    rule: "broken-link",
                    severity: Severity::Error,
                    message: format!("'{}' does not resolve to a corpus node", link.target),
                });
                continue;
            }

            // THE check: the relationship must be declared outgoing by this class, and
            // must point at the class it actually points at.
            let Some((target_class, _)) = target else {
                continue;
            };
            if !class.declares_outgoing(&link.relationship, &target_class) {
                let declared_for_target: Vec<&str> = class
                    .outgoing()
                    .filter(|e| e.target == target_class)
                    .map(|e| e.relationship.as_str())
                    .collect();
                let hint = if declared_for_target.is_empty() {
                    format!(
                        "class '{}' declares no outgoing edge to '{target_class}'",
                        inst.inst.class
                    )
                } else {
                    format!(
                        "declared to '{target_class}': {}",
                        declared_for_target.join(", ")
                    )
                };
                findings.push(Finding {
                    path: p.clone(),
                    rule: "undeclared-relationship",
                    severity: Severity::Error,
                    message: format!(
                        "'{}' -> {target_class} is not declared by class '{}' ({hint})",
                        link.relationship, inst.inst.class
                    ),
                });
            }
        }

        if !has_instance_of {
            findings.push(Finding {
                path: p.clone(),
                rule: "missing-instance-of",
                severity: Severity::Error,
                message: format!("no instance-of link to ../{}.ont.yml", inst.inst.class),
            });
        }

        // A class with no declared outgoing edges is a sink by design; its instances
        // cannot carry domain links and are not faulted for it.
        let is_sink = class.outgoing().count() == 0;
        if domain_link_count == 0 && !is_sink {
            findings.push(Finding {
                path: p.clone(),
                rule: "no-domain-link",
                severity: Severity::Warn,
                message: format!(
                    "only links to its own class file; class '{}' declares {} outgoing edge(s) that could connect it",
                    inst.inst.class,
                    class.outgoing().count()
                ),
            });
        }

        // Symmetric to the sink exemption: a class that declares no inbound edges is a
        // pure source by design. budget-action is the case — actions point at versions,
        // appropriations, and actors, and nothing ever points back at an action.
        let is_pure_source = class.edges.iter().all(|e| e.direction == Direction::Out);
        if !is_pure_source && incoming.get(&inst.abs_path).copied().unwrap_or(0) == 0 {
            findings.push(Finding {
                path: p.clone(),
                rule: "orphan-in",
                severity: Severity::Warn,
                message: "no incoming domain links — consider a reciprocal edge or a node that references this one".into(),
            });
        }

        // A [verified] claim requires a committed source. Registering a catalog entry is
        // not the same as committing its content, so this checks for a catalog link and
        // reports the stronger requirement in the message.
        let body = format!("{} {}", inst.inst.label, inst.inst.description);
        if body.contains("[verified]") {
            let cites_catalog = inst.inst.links.iter().any(|l| l.relationship == CITES);
            if !cites_catalog {
                findings.push(Finding {
                    path: p.clone(),
                    rule: "verified-without-source",
                    severity: Severity::Error,
                    message: "claims [verified] but links no catalog entry; verification requires a committed primary source".into(),
                });
            }
        }

        // Fixture data is fabricated. If it reaches a corpus node it becomes a made-up
        // fact wearing the costume of an extracted one — indistinguishable downstream from
        // a real figure, and citable. The boundary is enforced here rather than trusted.
        let property_text: String = inst
            .inst
            .properties
            .values()
            .filter_map(ScalarValue::as_text)
            .collect::<Vec<_>>()
            .join(" ");
        if body.contains(FIXTURE_MARKER) || property_text.contains(FIXTURE_MARKER) {
            findings.push(Finding {
                path: p.clone(),
                rule: "fixture-data-in-corpus",
                severity: Severity::Error,
                message: format!(
                    "references '{FIXTURE_MARKER}'; fixture values are synthetic and must never become corpus facts"
                ),
            });
        }

        for token in scan_claim_tags(&body) {
            findings.push(Finding {
                path: p.clone(),
                rule: "unknown-claim-tag",
                severity: Severity::Warn,
                message: format!("'{token}' is not one of [verified], [inference], [open]"),
            });
        }
    }

    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then(a.path.cmp(&b.path))
            .then(a.rule.cmp(b.rule))
    });
    findings
}

/// Reads the corpus from disk.
pub fn load(repo_root: &Path) -> Result<Corpus> {
    let corpus_root = repo_root.join(".yidam").join("corpus");
    let catalog_root = repo_root.join(".yidam").join("catalog");
    let mut classes = BTreeMap::new();
    let mut class_paths = BTreeSet::new();
    let mut catalog_paths = BTreeSet::new();
    let mut instances = Vec::new();

    let mut catalog = Vec::new();
    if catalog_root.is_dir() {
        for entry in walkdir::WalkDir::new(&catalog_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.ends_with(".md") || name == "README.md" {
                continue;
            }
            let abs = normalize_join(Path::new(""), &path.to_string_lossy());
            catalog_paths.insert(abs.clone());
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            let parsed = match split_frontmatter(&text) {
                None => Err("no YAML frontmatter delimited by ---".to_string()),
                Some(fm) => serde_yaml::from_str::<CatalogEntry>(fm).map_err(|e| e.to_string()),
            };
            catalog.push(LoadedCatalog {
                rel_path: path
                    .strip_prefix(repo_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string(),
                abs_path: abs,
                file_slug: name.trim_end_matches(".md").to_string(),
                parsed,
            });
        }
    }

    for entry in walkdir::WalkDir::new(&corpus_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".yml") {
            continue;
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let abs = normalize_join(Path::new(""), &path.to_string_lossy());
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        if name.ends_with(".ont.yml") {
            let def: ClassDefinition = serde_yaml::from_str(&text)
                .with_context(|| format!("parsing class definition {rel}"))?;
            class_paths.insert(abs);
            classes.insert(def.class.clone(), def);
        } else {
            let inst: CorpusInstance =
                serde_yaml::from_str(&text).with_context(|| format!("parsing instance {rel}"))?;
            instances.push(LoadedInstance {
                rel_path: rel,
                abs_path: abs,
                inst,
            });
        }
    }

    Ok(Corpus {
        classes,
        class_paths,
        catalog_paths,
        catalog,
        instances,
    })
}

/// Convenience for property lookups in downstream tooling.
pub fn property_text<'a>(inst: &'a CorpusInstance, key: &str) -> Option<&'a str> {
    inst.properties.get(key).and_then(ScalarValue::as_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(yaml: &str) -> ClassDefinition {
        serde_yaml::from_str(yaml).unwrap()
    }

    fn instance(rel: &str, yaml: &str) -> LoadedInstance {
        LoadedInstance {
            rel_path: rel.to_string(),
            abs_path: PathBuf::from(rel),
            inst: serde_yaml::from_str(yaml).unwrap(),
        }
    }

    fn run(
        classes: BTreeMap<String, ClassDefinition>,
        class_paths: BTreeSet<PathBuf>,
        instances: Vec<LoadedInstance>,
    ) -> Vec<Finding> {
        check(&Corpus {
            classes,
            class_paths,
            catalog_paths: BTreeSet::new(),
            catalog: Vec::new(),
            instances,
        })
    }

    fn fixture() -> (BTreeMap<String, ClassDefinition>, BTreeSet<PathBuf>) {
        let mut classes = BTreeMap::new();
        classes.insert(
            "fund".to_string(),
            class(
                r#"
class: fund
label: Fund
description: d
properties:
  - name: name
    type: string
edges:
  - relationship: transfers-to
    target: fund
    direction: out
"#,
            ),
        );
        classes.insert(
            "expenditure".to_string(),
            class(
                r#"
class: expenditure
label: Expenditure
description: d
properties:
  - name: amount
    type: string
edges:
  - relationship: paid-to
    target: jurisdiction
    direction: out
"#,
            ),
        );
        classes.insert(
            "jurisdiction".to_string(),
            class(
                r#"
class: jurisdiction
label: Jurisdiction
description: d
edges:
  - relationship: paid-to
    target: expenditure
    direction: in
"#,
            ),
        );
        let mut paths = BTreeSet::new();
        paths.insert(PathBuf::from("corpus/fund.ont.yml"));
        paths.insert(PathBuf::from("corpus/expenditure.ont.yml"));
        paths.insert(PathBuf::from("corpus/jurisdiction.ont.yml"));
        (classes, paths)
    }

    #[test]
    fn normalize_join_collapses_parent_dirs() {
        let base = Path::new("/repo/corpus/appropriation");
        assert_eq!(
            normalize_join(base, "../line-item/foundation-funding.yml"),
            PathBuf::from("/repo/corpus/line-item/foundation-funding.yml")
        );
        assert_eq!(
            normalize_join(base, "./sibling.yml"),
            PathBuf::from("/repo/corpus/appropriation/sibling.yml")
        );
    }

    #[test]
    fn undeclared_relationship_is_caught() {
        // This is the bootstrap bug, reproduced: `holds` was asserted from agency to
        // line-item and no class declared it.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: d
links:
  - target: ../fund.ont.yml
    relationship: instance-of
  - target: ./b.yml
    relationship: holds
"#,
        )];
        let insts = {
            let mut v = insts;
            v.push(instance(
                "corpus/fund/b.yml",
                r#"
class: fund
label: B
description: d
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
            ));
            v
        };
        let findings = run(classes, paths, insts);
        let f = findings
            .iter()
            .find(|f| f.rule == "undeclared-relationship")
            .expect("must flag the undeclared 'holds' relationship");
        assert!(f.message.contains("holds"));
        assert!(
            f.message.contains("transfers-to"),
            "should hint at what IS declared"
        );
    }

    #[test]
    fn declared_relationship_passes() {
        let (classes, paths) = fixture();
        let insts = vec![
            instance(
                "corpus/fund/a.yml",
                r#"
class: fund
label: A
description: d
links:
  - target: ../fund.ont.yml
    relationship: instance-of
  - target: ./b.yml
    relationship: transfers-to
"#,
            ),
            instance(
                "corpus/fund/b.yml",
                r#"
class: fund
label: B
description: d
links:
  - target: ../fund.ont.yml
    relationship: instance-of
  - target: ./a.yml
    relationship: transfers-to
"#,
            ),
        ];
        let findings = run(classes, paths, insts);
        assert!(
            findings.iter().all(|f| f.severity != Severity::Error),
            "unexpected errors: {findings:?}"
        );
    }

    #[test]
    fn cross_class_incoming_links_are_counted() {
        // Regression against the upstream orphan-in bug: a node targeted only via a
        // `../class/x.yml` path must not be reported as an orphan.
        let (classes, mut paths) = fixture();
        paths.insert(PathBuf::from("corpus/jurisdiction/ccsd.yml"));
        let insts = vec![
            instance(
                "corpus/expenditure/e.yml",
                r#"
class: expenditure
label: E
description: d
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
  - target: ../jurisdiction/ccsd.yml
    relationship: paid-to
"#,
            ),
            instance(
                "corpus/jurisdiction/ccsd.yml",
                r#"
class: jurisdiction
label: CCSD
description: d
links:
  - target: ../jurisdiction.ont.yml
    relationship: instance-of
"#,
            ),
        ];
        let findings = run(classes, paths, insts);
        assert!(
            !findings
                .iter()
                .any(|f| f.rule == "orphan-in" && f.path.contains("ccsd")),
            "cross-class link must register as incoming: {findings:?}"
        );
    }

    #[test]
    fn sink_class_is_not_faulted_for_lacking_domain_links() {
        let (classes, mut paths) = fixture();
        paths.insert(PathBuf::from("corpus/jurisdiction/ccsd.yml"));
        let insts = vec![instance(
            "corpus/jurisdiction/ccsd.yml",
            r#"
class: jurisdiction
label: CCSD
description: d
links:
  - target: ../jurisdiction.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        assert!(
            !findings.iter().any(|f| f.rule == "no-domain-link"),
            "jurisdiction declares only inbound edges and is a sink by design"
        );
    }

    #[test]
    fn broken_link_is_caught() {
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: d
links:
  - target: ../fund.ont.yml
    relationship: instance-of
  - target: ./ghost.yml
    relationship: transfers-to
"#,
        )];
        let findings = run(classes, paths, insts);
        assert!(findings.iter().any(|f| f.rule == "broken-link"));
    }

    #[test]
    fn undeclared_property_is_caught() {
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: d
properties:
  name: A
  invented: nope
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        let f = findings
            .iter()
            .find(|f| f.rule == "undeclared-property")
            .expect("must flag undeclared property");
        assert!(f.message.contains("invented"));
    }

    #[test]
    fn a_claim_tag_inside_an_identifier_is_rejected() {
        // Regression for a real failure: nine line items were verified against a real
        // spreadsheet and the tag was written into the code property, so the next extraction
        // run matched none of them and reported zero proposals. The break was silent.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: d
properties:
  fund_number: "5000 [verified]"
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        let f = findings
            .iter()
            .find(|f| f.rule == "claim-tag-in-identifier")
            .expect("a tag inside an identifier silently breaks every downstream match");
        assert_eq!(f.severity, Severity::Error);
    }

    #[test]
    fn a_bare_open_placeholder_in_an_identifier_is_allowed() {
        // "[open] pending verification" says there is no value yet, which is the established
        // convention and breaks nothing — there is no identifier to match on.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: d
properties:
  fund_number: "[open] pending verification"
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        assert!(
            !findings.iter().any(|f| f.rule == "claim-tag-in-identifier"),
            "{findings:?}"
        );
    }

    #[test]
    fn float_money_is_rejected() {
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/expenditure/e.yml",
            r#"
class: expenditure
label: E
description: d
properties:
  amount: 1234.56
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        assert!(
            findings.iter().any(|f| f.rule == "float-money"),
            "binary floats cannot represent cents exactly and compound silently across sums"
        );
    }

    #[test]
    fn verified_claim_without_catalog_link_is_rejected() {
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: |
  The rate stood at 5.75 percent. [verified]
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        assert!(findings.iter().any(|f| f.rule == "verified-without-source"));
    }

    fn run_with_catalog(
        classes: BTreeMap<String, ClassDefinition>,
        class_paths: BTreeSet<PathBuf>,
        catalog_paths: BTreeSet<PathBuf>,
        instances: Vec<LoadedInstance>,
    ) -> Vec<Finding> {
        check(&Corpus {
            classes,
            class_paths,
            catalog_paths,
            catalog: Vec::new(),
            instances,
        })
    }

    fn citing_instance() -> LoadedInstance {
        instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: d
links:
  - target: ../fund.ont.yml
    relationship: instance-of
  - target: ../catalog/orc-131.md
    relationship: cites
"#,
        )
    }

    #[test]
    fn cites_to_catalog_needs_no_declared_edge() {
        // Provenance is cross-cutting. Declaring a `cites` edge on all fourteen classes
        // would be noise, so the rule is special-cased rather than restated per class.
        let (classes, paths) = fixture();
        let mut catalog = BTreeSet::new();
        catalog.insert(PathBuf::from("corpus/catalog/orc-131.md"));
        let findings = run_with_catalog(classes, paths, catalog, vec![citing_instance()]);
        assert!(
            findings.iter().all(|f| f.severity != Severity::Error),
            "unexpected errors: {findings:?}"
        );
    }

    #[test]
    fn cites_to_a_missing_catalog_entry_is_broken() {
        let (classes, paths) = fixture();
        let findings = run_with_catalog(classes, paths, BTreeSet::new(), vec![citing_instance()]);
        assert!(findings
            .iter()
            .any(|f| f.rule == "broken-link" && f.severity == Severity::Error));
    }

    #[test]
    fn non_cites_relationship_to_catalog_is_rejected() {
        let (classes, paths) = fixture();
        let mut catalog = BTreeSet::new();
        catalog.insert(PathBuf::from("corpus/catalog/orc-131.md"));
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: d
links:
  - target: ../fund.ont.yml
    relationship: instance-of
  - target: ../catalog/orc-131.md
    relationship: draws-from
"#,
        )];
        let findings = run_with_catalog(classes, paths, catalog, insts);
        assert!(findings
            .iter()
            .any(|f| f.rule == "bad-catalog-relationship"));
    }

    #[test]
    fn a_provenance_edge_is_not_a_domain_link() {
        // A node that only cites a source is still unconnected to the graph.
        let (classes, paths) = fixture();
        let mut catalog = BTreeSet::new();
        catalog.insert(PathBuf::from("corpus/catalog/orc-131.md"));
        let findings = run_with_catalog(classes, paths, catalog, vec![citing_instance()]);
        assert!(
            findings.iter().any(|f| f.rule == "no-domain-link"),
            "citing a source does not connect a node to the corpus: {findings:?}"
        );
    }

    #[test]
    fn fixture_data_in_a_corpus_node_is_rejected() {
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: |
  Balance taken from the connector run.
properties:
  name: "from synthetic-fixture run"
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        let f = findings
            .iter()
            .find(|f| f.rule == "fixture-data-in-corpus")
            .expect("synthetic fixture values must never become corpus facts");
        assert_eq!(f.severity, Severity::Error);
    }

    #[test]
    fn edge_notation_in_prose_is_not_a_claim_tag() {
        // Regression: node bodies describe the schema as `agency →[succeeds]→ agency`,
        // and the first version of this scanner reported every one as a bad claim tag.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: |
  The approved `fund →[transfers-to]→ fund` edge exposes the mechanism.
  See also agency →[succeeds]→ agency and program -[draws-from]-> revenue-source.
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        let tags: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.rule == "unknown-claim-tag")
            .collect();
        assert!(
            tags.is_empty(),
            "edge notation misread as claim tags: {tags:?}"
        );
    }

    #[test]
    fn pure_source_class_is_not_faulted_for_lacking_incoming_links() {
        let mut classes = BTreeMap::new();
        classes.insert(
            "budget-action".to_string(),
            class(
                r#"
class: budget-action
label: Budget Action
description: d
edges:
  - relationship: taken-by
    target: actor
    direction: out
"#,
            ),
        );
        classes.insert(
            "actor".to_string(),
            class(
                r#"
class: actor
label: Actor
description: d
edges:
  - relationship: taken-by
    target: budget-action
    direction: in
"#,
            ),
        );
        let mut paths = BTreeSet::new();
        paths.insert(PathBuf::from("corpus/budget-action.ont.yml"));
        paths.insert(PathBuf::from("corpus/actor.ont.yml"));
        paths.insert(PathBuf::from("corpus/actor/gov.yml"));
        let insts = vec![
            instance(
                "corpus/budget-action/veto.yml",
                r#"
class: budget-action
label: Veto
description: d
links:
  - target: ../budget-action.ont.yml
    relationship: instance-of
  - target: ../actor/gov.yml
    relationship: taken-by
"#,
            ),
            instance(
                "corpus/actor/gov.yml",
                r#"
class: actor
label: Gov
description: d
links:
  - target: ../actor.ont.yml
    relationship: instance-of
"#,
            ),
        ];
        let findings = run(classes, paths, insts);
        assert!(
            !findings
                .iter()
                .any(|f| f.rule == "orphan-in" && f.path.contains("veto")),
            "nothing ever points at an action; that is the class's shape, not a defect: {findings:?}"
        );
    }

    #[test]
    fn unknown_claim_tag_is_flagged_but_markdown_links_are_not() {
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/fund/a.yml",
            r#"
class: fund
label: A
description: |
  See [the fund](../fund/b.yml) for detail. This part is uncertain. [probable]
  And this one is fine. [inference]
links:
  - target: ../fund.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        let tags: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.rule == "unknown-claim-tag")
            .collect();
        assert_eq!(tags.len(), 1, "got {tags:?}");
        assert!(tags[0].message.contains("[probable]"));
    }
}
