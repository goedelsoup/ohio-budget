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
    /// Decision records, as top-level keys only. The rules care which keys are present, not
    /// what is in them.
    pub decisions: Vec<LoadedDecision>,
}

/// A decision record's top-level keys, for the schema-coverage rule.
#[derive(Debug, Clone)]
pub struct LoadedDecision {
    pub rel_path: String,
    pub keys: Vec<String>,
}

/// The fields `corpus_schema::DecisionRecord` deserializes.
///
/// Duplicated as a list rather than derived from the type because serde offers no way to
/// enumerate a struct's fields at runtime, and the alternative — `deny_unknown_fields` — would
/// make `corpus-export` skip the whole record instead of dropping one key, which is worse.
/// [`a_decision_key_outside_the_schema_is_reported`] fails if the two drift.
const DECISION_FIELDS: [&str; 6] = [
    "id",
    "summary",
    "corpus_depth",
    "context",
    "decision",
    "rationale",
];

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

/// The (line item, fiscal period) pair a node attaches to, if it names both.
///
/// Keyed on the *target class* rather than the relationship name. `grants-authority-for` and
/// `disburses-against` name the same subject from the authority side and the outturn side, and
/// pinning this to those two spellings would silently stop working the day either is renamed.
fn subject_of(inst: &CorpusInstance, dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let (mut line_item, mut period) = (None, None);
    for link in inst.domain_links() {
        let target = normalize_join(dir, &link.target);
        match target_class_of(&target) {
            Some((c, false)) if c == "line-item" => line_item = Some(target),
            Some((c, false)) if c == "fiscal-period" => period = Some(target),
            _ => {}
        }
    }
    Some((line_item?, period?))
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

/// Is this value still a placeholder rather than a figure?
fn is_open_placeholder(value: &ScalarValue) -> bool {
    value
        .as_text()
        .is_some_and(|t| t.trim_start().starts_with("[open]"))
}

/// Does the body claim a value has not been filled in yet?
///
/// Narrow on purpose. This exists because a specific defect recurred: when amounts were filled
/// in bulk from the appropriation spreadsheet, the verified figure was appended to the node
/// body and the sentence above it saying the amount was unfilled was left in place. Two nodes
/// carried both at once, one of them also naming a source that does not hold the figure.
///
/// Neither statement is detectably wrong on its own — the property looks fine, the prose looks
/// fine — and nothing else in this validator reads the body against the properties. A reader
/// hitting the contradiction cannot tell which half is stale.
fn says_value_is_unfilled(body: &str) -> Option<&'static str> {
    const PHRASES: [&str; 4] = [
        "amount is unfilled",
        "figure is unfilled",
        "amount is not yet filled",
        "amount is still unfilled",
    ];
    let lower = body.to_ascii_lowercase();
    PHRASES.into_iter().find(|p| lower.contains(p))
}

/// Does the body claim no source carries the outturn for this appropriation's year?
///
/// Narrow on purpose, and a sibling of [`says_value_is_unfilled`] — same defect, one level out.
/// That rule catches a node contradicting its own property; this one catches a node
/// contradicting a *different node* that has since been extracted.
///
/// The instance: four FY2021 appropriations each said "FY2021 actuals are not carried by any
/// committed source; HB 110's with-actuals workbook would close it." The workbook was catalogued,
/// its actuals were extracted, and `gap` was computing all four variances — while the
/// appropriations still told the reader the figure did not exist. The claim went stale in the
/// commit that falsified it, which is the shape this whole family of rules exists to catch.
///
/// Deliberately not a general "no source carries X" matcher: the corpus says that truthfully in
/// many places, about figures nothing has extracted. This fires only where the corpus itself
/// holds the counterpart, so it cannot argue with a claim it has no evidence against.
fn says_no_outturn_exists(body: &str) -> Option<&'static str> {
    const PHRASES: [&str; 4] = [
        "actuals are not carried by any committed source",
        "actual is not carried by any committed source",
        "no committed source carries its actuals",
        "no committed source carries the actuals",
    ];
    let lower = body.to_ascii_lowercase();
    PHRASES.into_iter().find(|p| lower.contains(p))
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

// ─── claims about the repository's own state ─────────────────────────────────

/// A node's assertion that some source is not catalogued.
///
/// # Why this needs a convention rather than a cleverer parser
///
/// Node prose routinely asserts things about this repository — a source is not catalogued, a
/// figure is not extracted, a connector is not written. Those claims go stale as the repository
/// fills in, and they go stale **in the same commit that makes them false**, because the person
/// adding the source is not the person re-reading every node that mentioned it. Three separate
/// instances of this have been found and fixed by hand.
///
/// An attempt was made to catch them by matching the prose against catalog entry names. It was
/// measured across seven commits before being abandoned: it missed the real cases when tuned to
/// avoid false positives, and flagged `hb33-as-enacted`'s correct claim about HB 33's veto
/// message against HB 96's entry when tuned to catch them. Free prose does not carry enough to
/// decide.
///
/// So the claim names its own subject instead. `[open] Not catalogued (some-slug)` is exactly
/// checkable: either that slug is in the catalog or it is not.
///
/// # Catalogued is not the same as committed
///
/// The distinction the corpus already draws and this vocabulary kept blurring. A catalog *entry*
/// registers a source; `content_committed` says its bytes are in the repository, which is what
/// a `[verified]` claim requires. `controlling-board-minutes` has been catalogued since genesis
/// with its content uncommitted, and a node saying it is "not yet catalogued" is wrong about
/// which of the two it means.
const NOT_CATALOGUED: &str = "not catalogued";
const NOT_YET_CATALOGUED: &str = "not yet catalogued";

/// Extracts the slug from `Not catalogued (some-slug)`, if the claim names one.
///
/// `at` indexes into the **lowercased** text, which is also what this reads — slugs are
/// lowercase by construction, so nothing is lost, and matching the phrase case-sensitively
/// would silently miss every sentence-initial `Not catalogued`. It did.
fn named_catalogue_slug(lower: &str, at: usize) -> Option<String> {
    let rest = &lower[at..];
    let rest = rest
        .strip_prefix(NOT_YET_CATALOGUED)
        .or_else(|| rest.strip_prefix(NOT_CATALOGUED))?
        .trim_start();
    let inner = rest.strip_prefix('(')?;
    let end = inner.find(')')?;
    let slug = inner[..end].trim();
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    Some(slug.to_string())
}

/// Finds every `not catalogued` assertion in a node's text, with its byte offset.
fn catalogue_claims(text: &str) -> Vec<usize> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = lower[from..].find(NOT_CATALOGUED) {
        let at = from + rel;
        // `not yet catalogued` also contains `not catalogued`? It does not — but it is the
        // other spelling in use, so both are located and the earlier start wins.
        out.push(at);
        from = at + NOT_CATALOGUED.len();
    }
    let mut from = 0;
    while let Some(rel) = lower[from..].find(NOT_YET_CATALOGUED) {
        let at = from + rel;
        if !out.contains(&at) {
            out.push(at);
        }
        from = at + NOT_YET_CATALOGUED.len();
    }
    out.sort_unstable();
    out
}

/// Runs every rule. Pure over loaded data.
pub fn check(corpus: &Corpus) -> Vec<Finding> {
    let Corpus {
        classes,
        class_paths: class_def_paths,
        catalog_paths,
        catalog,
        instances,
        decisions,
    } = corpus;
    let mut findings = Vec::new();
    let known_instances: BTreeSet<&PathBuf> = instances.iter().map(|i| &i.abs_path).collect();
    let catalog_by_slug: BTreeMap<&str, &CatalogEntry> = catalog
        .iter()
        .filter_map(|c| c.parsed.as_ref().ok().map(|e| (e.slug.as_str(), e)))
        .collect();

    // A decision record's top-level key that the schema does not deserialize is dropped in
    // silence: it stays on disk, reads correctly to anyone opening the file, and never reaches
    // the feed or the site. `leadership-and-the-anomalies.yml` carried its central evidence —
    // the four-biennium alignment table the whole record argues about — under `the_alignment:`
    // for as long as the record existed, and no reader of the published version ever saw it.
    //
    // An error rather than a warning. The failure mode is not untidiness: it is a record that
    // is complete in the repository and materially incomplete everywhere it is read, with
    // nothing on either side to indicate a difference.
    for d in decisions {
        // A file that is not a mapping — `proposals.yml` holds a list — has no keys to check,
        // and was never claiming to be a decision record.
        if d.keys.is_empty() {
            continue;
        }
        let stray: Vec<&str> = d
            .keys
            .iter()
            .map(String::as_str)
            .filter(|k| !DECISION_FIELDS.contains(k))
            .collect();
        if !stray.is_empty() {
            findings.push(Finding {
                path: d.rel_path.clone(),
                rule: "decision-key-outside-schema",
                severity: Severity::Error,
                message: format!(
                    "top-level key(s) {stray:?} are not fields of DecisionRecord, so they are \
                     dropped from the exported feed and never reach a reader — fold the content \
                     into {DECISION_FIELDS:?}, or add the field to the schema"
                ),
            });
        }
    }

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

    // Outturn nodes keyed by the (line item, period) they report on. An appropriation claiming
    // no source carries its year's actuals is checked against this and nothing else: the corpus
    // either holds the counterpart or it does not.
    let mut outturn_by_subject: BTreeMap<(PathBuf, PathBuf), Vec<String>> = BTreeMap::new();
    for inst in instances {
        if inst.inst.class != "expenditure" {
            continue;
        }
        let dir = inst.abs_path.parent().unwrap_or(&inst.abs_path);
        if let Some(key) = subject_of(&inst.inst, dir) {
            outturn_by_subject
                .entry(key)
                .or_default()
                .push(inst.rel_path.clone());
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
            if looks_like_money(key) && !is_open_placeholder(value) {
                if let Some(phrase) = says_value_is_unfilled(&inst.inst.description) {
                    findings.push(Finding {
                        path: p.clone(),
                        rule: "body-contradicts-filled-property",
                        severity: Severity::Error,
                        message: format!(
                            "property '{key}' holds a value but the body still says {phrase:?}; \
                             one of the two is wrong and a reader has no way to tell which"
                        ),
                    });
                }
            }
        }

        if let Some(phrase) = says_no_outturn_exists(&inst.inst.description) {
            if let Some(key) = subject_of(&inst.inst, &dir) {
                let holders: Vec<&str> = outturn_by_subject
                    .get(&key)
                    .map(|v| {
                        v.iter()
                            .filter(|h| *h != p)
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !holders.is_empty() {
                    findings.push(Finding {
                        path: p.clone(),
                        rule: "stale-no-outturn-claim",
                        severity: Severity::Error,
                        message: format!(
                            "says {phrase:?}, but the corpus holds the outturn for the same line \
                             item and period ({}); the claim went stale when it was extracted",
                            holders.join(", ")
                        ),
                    });
                }
            }
        }

        // Claims about what this repository contains, checked against what it contains.
        //
        // The whole text is scanned, body and properties together, because these assertions sit
        // in both — `description` prose and `source_document` alike.
        let all_text = {
            let mut t = inst.inst.description.clone();
            for v in inst.inst.properties.values() {
                if let Some(s) = v.as_text() {
                    t.push('\n');
                    t.push_str(s);
                }
            }
            t
        };
        let lower_text = all_text.to_ascii_lowercase();
        for at in catalogue_claims(&all_text) {
            match named_catalogue_slug(&lower_text, at) {
                Some(slug) => {
                    if let Some(entry) = catalog_by_slug.get(slug.as_str()) {
                        let committed = entry.content_committed;
                        findings.push(Finding {
                            path: p.clone(),
                            rule: "stale-not-catalogued-claim",
                            severity: Severity::Error,
                            message: format!(
                                "says '{slug}' is not catalogued, but it is{}; the claim went \
                                 stale when the entry was added",
                                if committed {
                                    " and its content is committed"
                                } else {
                                    ", with content not yet committed — which is the distinction \
                                     the claim probably meant"
                                }
                            ),
                        });
                    }
                }
                None => findings.push(Finding {
                    path: p.clone(),
                    rule: "unnamed-not-catalogued-claim",
                    severity: Severity::Warn,
                    message: "asserts something is not catalogued without naming it; write \
                              'not catalogued (slug)' so the claim can be checked against the \
                              catalog instead of going stale unnoticed"
                        .into(),
                }),
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
                // A rule checking that the cited entry's `feeds` includes this node's class was
                // written here and removed. It fired 44 times, and reading them showed the
                // premise was wrong: `feeds` names what a source primarily populates, not who
                // may cite it. The governor node cites the appropriation spreadsheet to support
                // a finding about vetoes, and every `budget-action` cites it for `amount_delta`;
                // both are correct and neither class is in its `feeds`.
                //
                // The defect that prompted it is not mechanically detectable. Five appropriation
                // nodes cited a comparison document because the *catalog entry* said comparison
                // documents carry per-stage amounts, and it was wrong about the document. No
                // check over the corpus can find that — only reading the source can.
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
        //
        // Neither is a node that carries none but is pointed at. `local-government-fund` has one
        // declared outgoing edge — `transfers-to` another fund — and transfers to nothing, while
        // appropriations draw from it and the general revenue fund transfers into it. It is
        // among the best-connected nodes in the corpus and was being reported as unwired.
        //
        // The symmetric mistake to the one `orphan-in` was making before `EdgeDef::expected`:
        // measuring one direction and calling it connectivity. A node connected either way is in
        // the graph, and the case worth reporting is a node connected neither way.
        let is_sink = class.outgoing().count() == 0;
        let pointed_at = incoming.get(&inst.abs_path).copied().unwrap_or(0) > 0;
        if domain_link_count == 0 && !is_sink && !pointed_at {
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

        // Fires only where the ontology says every instance expects a reciprocal — see
        // `EdgeDef::expected`. Keyed on that rather than on "declares any inbound edge",
        // which is what it used to do and which faulted 91 correct nodes: an appropriation
        // nothing vetoed and an expenditure no appropriation named were both reported as
        // orphans, because those inbound edges exist for the minority of cases that have them.
        //
        // A rule that fires on the ordinary case is a rule nobody reads. At 153 warnings the
        // validator's output was scrolled past, which is a worse failure than any single
        // missing edge — it is how a real warning arrives unnoticed.
        let mut expected_in: Vec<&str> = class
            .edges
            .iter()
            .filter(|e| e.direction == Direction::In && e.expected)
            .map(|e| e.relationship.as_str())
            .collect();
        // Deduped: `fund` declares `draws-from` twice, once from `appropriation` and once from
        // `line-item`, and naming it twice reads as a mistake in the message rather than as
        // two ways of satisfying one requirement.
        expected_in.sort_unstable();
        expected_in.dedup();
        if !expected_in.is_empty() && incoming.get(&inst.abs_path).copied().unwrap_or(0) == 0 {
            findings.push(Finding {
                path: p.clone(),
                rule: "orphan-in",
                severity: Severity::Warn,
                message: format!(
                    "no incoming domain links; class '{}' expects at least one of {expected_in:?}",
                    inst.inst.class
                ),
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

    let mut decisions = Vec::new();
    let decisions_root = repo_root.join(".yidam").join("decisions");
    if decisions_root.is_dir() {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&decisions_root)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yml"))
            .collect();
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            // Top-level keys only. A record that is not a mapping at all — `proposals.yml` is a
            // list — has none, and the rule then has nothing to say about it.
            let keys = serde_yaml::from_str::<serde_yaml::Value>(&text)
                .ok()
                .and_then(|v| v.as_mapping().cloned())
                .map(|m| {
                    m.keys()
                        .filter_map(|k| k.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            decisions.push(LoadedDecision {
                rel_path: path
                    .strip_prefix(repo_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string(),
                keys,
            });
        }
    }

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
        decisions,
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
        run_with_entries(classes, class_paths, instances, &[])
    }

    /// Same, with parsed catalog entries. `(slug, content_committed)` per entry.
    fn run_with_entries(
        classes: BTreeMap<String, ClassDefinition>,
        class_paths: BTreeSet<PathBuf>,
        instances: Vec<LoadedInstance>,
        entries: &[(&str, bool)],
    ) -> Vec<Finding> {
        let catalog = entries
            .iter()
            .map(|(slug, committed)| LoadedCatalog {
                rel_path: format!(".yidam/catalog/{slug}.md"),
                abs_path: PathBuf::from(format!("/repo/.yidam/catalog/{slug}.md")),
                file_slug: slug.to_string(),
                parsed: Ok(CatalogEntry {
                    slug: slug.to_string(),
                    name: slug.to_string(),
                    source_type: corpus_schema::SourceType::Other,
                    location: String::new(),
                    publisher: String::new(),
                    content_committed: *committed,
                    feeds: Vec::new(),
                    access_constraints: None,
                }),
            })
            .collect();
        check(&Corpus {
            classes,
            class_paths,
            catalog_paths: BTreeSet::new(),
            catalog,
            instances,
            decisions: Vec::new(),
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
    fn a_body_saying_the_amount_is_unfilled_over_a_filled_amount_is_rejected() {
        // The real defect: amounts were filled in bulk from the spreadsheet by appending the
        // verified figure to the body, leaving the sentence above it saying the amount was
        // unfilled. Two nodes carried both at once and nothing detected it, because each half
        // is well-formed on its own.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/expenditure/e.yml",
            r#"
class: expenditure
label: E
description: |
  [open] The amount is unfilled, pending the connector.
  Disbursed $1,234.00 in FY2026. [verified]
properties:
  amount: "$1,234.00"
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        assert!(
            findings
                .iter()
                .any(|f| f.rule == "body-contradicts-filled-property"),
            "{findings:?}"
        );
    }

    /// [`fixture`] plus the three classes the outturn rule traverses, and an `expenditure`
    /// that declares the two edges tying it to a line item and a period.
    fn outturn_fixture() -> (BTreeMap<String, ClassDefinition>, BTreeSet<PathBuf>) {
        let (mut classes, mut paths) = fixture();
        for (name, yaml) in [
            (
                "appropriation",
                r#"
class: appropriation
label: Appropriation
description: d
edges:
  - relationship: grants-authority-for
    target: line-item
    direction: out
  - relationship: covers
    target: fiscal-period
    direction: out
"#,
            ),
            (
                "expenditure",
                r#"
class: expenditure
label: Expenditure
description: d
edges:
  - relationship: disburses-against
    target: line-item
    direction: out
  - relationship: occurs-in
    target: fiscal-period
    direction: out
"#,
            ),
            (
                "line-item",
                r#"
class: line-item
label: Line Item
description: d
edges:
  - relationship: grants-authority-for
    target: appropriation
    direction: in
"#,
            ),
            (
                "fiscal-period",
                r#"
class: fiscal-period
label: Fiscal Period
description: d
edges:
  - relationship: covers
    target: appropriation
    direction: in
"#,
            ),
        ] {
            classes.insert(name.to_string(), class(yaml));
            paths.insert(PathBuf::from(format!("corpus/{name}.ont.yml")));
        }
        (classes, paths)
    }

    fn appropriation_claiming_no_outturn() -> LoadedInstance {
        instance(
            "corpus/appropriation/a-fy2021.yml",
            r#"
class: appropriation
label: A FY2021
description: |
  $100.00 appropriated in FY2021. [verified]

  [open] FY2021 actuals are not carried by any committed source.
properties:
  amount: "$100.00"
links:
  - target: ../appropriation.ont.yml
    relationship: instance-of
  - target: ../line-item/a.yml
    relationship: grants-authority-for
  - target: ../fiscal-period/fy2021.yml
    relationship: covers
"#,
        )
    }

    #[test]
    fn an_appropriation_denying_an_outturn_the_corpus_holds_is_rejected() {
        // The real defect: four FY2021 appropriations kept saying the actuals were not carried
        // by any committed source after HB 110's workbook was catalogued and extracted. `gap`
        // was computing all four variances at the time, from the very nodes being denied.
        let (classes, paths) = outturn_fixture();
        let insts = vec![
            appropriation_claiming_no_outturn(),
            instance(
                "corpus/expenditure/a-fy2021-actual.yml",
                r#"
class: expenditure
label: A FY2021 Actual
description: |
  $90.00 charged in FY2021, on closed books. [verified]
properties:
  amount: "$90.00"
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
  - target: ../line-item/a.yml
    relationship: disburses-against
  - target: ../fiscal-period/fy2021.yml
    relationship: occurs-in
"#,
            ),
        ];
        let findings = run(classes, paths, insts);
        assert!(
            findings.iter().any(|f| f.rule == "stale-no-outturn-claim"),
            "{findings:?}"
        );
    }

    #[test]
    fn an_appropriation_denying_an_outturn_nothing_holds_is_fine() {
        // The same sentence is true wherever extraction has not reached, and the corpus says it
        // in many places. The rule must argue only where it has the counterpart in hand.
        let (classes, paths) = outturn_fixture();
        let insts = vec![appropriation_claiming_no_outturn()];
        let findings = run(classes, paths, insts);
        assert!(
            !findings.iter().any(|f| f.rule == "stale-no-outturn-claim"),
            "{findings:?}"
        );
    }

    #[test]
    fn an_outturn_for_a_different_period_does_not_close_the_claim() {
        // The pair is (line item, period). An actual for a neighbouring year says nothing about
        // the year the claim is about, and matching on the line item alone would silence it.
        let (classes, paths) = outturn_fixture();
        let insts = vec![
            appropriation_claiming_no_outturn(),
            instance(
                "corpus/expenditure/a-fy2022-actual.yml",
                r#"
class: expenditure
label: A FY2022 Actual
description: d
properties:
  amount: "$90.00"
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
  - target: ../line-item/a.yml
    relationship: disburses-against
  - target: ../fiscal-period/fy2022.yml
    relationship: occurs-in
"#,
            ),
        ];
        let findings = run(classes, paths, insts);
        assert!(
            !findings.iter().any(|f| f.rule == "stale-no-outturn-claim"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_body_saying_the_amount_is_unfilled_over_an_open_amount_is_fine() {
        // The same sentence is correct while the placeholder is still there, and several
        // nodes legitimately carry it.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/expenditure/e.yml",
            r#"
class: expenditure
label: E
description: |
  [open] The amount is unfilled, pending the obm connector.
properties:
  amount: "[open] pending the obm connector"
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run(classes, paths, insts);
        assert!(
            !findings
                .iter()
                .any(|f| f.rule == "body-contradicts-filled-property"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_claim_that_a_catalogued_source_is_not_catalogued_is_rejected() {
        // The exact defect, four instances of which were live when this rule was written:
        // nodes waiting on HB 33's appropriation spreadsheet, which had been catalogued and
        // committed several commits earlier.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/expenditure/e.yml",
            r#"
class: expenditure
label: E
description: d
properties:
  reporting_source: |
    [open] Not catalogued (obm-annual-report).
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run_with_entries(classes, paths, insts, &[("obm-annual-report", true)]);
        assert!(
            findings
                .iter()
                .any(|f| f.rule == "stale-not-catalogued-claim"),
            "{findings:?}"
        );
    }

    #[test]
    fn naming_a_slug_that_does_not_exist_is_a_live_claim_not_an_error() {
        // The point of naming it: the claim becomes a dependency that fires the day the
        // entry appears, instead of prose nobody rereads.
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/expenditure/e.yml",
            r#"
class: expenditure
label: E
description: |
  [open] Not catalogued (lsc-hb1-appropriation-spreadsheet).
properties: {}
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run_with_entries(classes, paths, insts, &[("obm-annual-report", true)]);
        assert!(
            !findings.iter().any(|f| f.rule.contains("not-catalogued")),
            "{findings:?}"
        );
    }

    #[test]
    fn an_unnamed_not_catalogued_claim_is_flagged_as_uncheckable() {
        let (classes, paths) = fixture();
        let insts = vec![instance(
            "corpus/expenditure/e.yml",
            r#"
class: expenditure
label: E
description: |
  The source for this figure is not yet catalogued.
properties: {}
links:
  - target: ../expenditure.ont.yml
    relationship: instance-of
"#,
        )];
        let findings = run_with_entries(classes, paths, insts, &[]);
        assert!(
            findings
                .iter()
                .any(|f| f.rule == "unnamed-not-catalogued-claim"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_sentence_initial_claim_is_matched_despite_its_capital() {
        // Matching case-sensitively silently missed every `Not catalogued (...)` that began a
        // sentence, which is most of them. The rule reported them as unnamed instead.
        assert_eq!(
            named_catalogue_slug(&"Not catalogued (some-slug).".to_ascii_lowercase(), 0),
            Some("some-slug".to_string())
        );
        assert_eq!(
            named_catalogue_slug(&"not yet catalogued (other-slug)".to_ascii_lowercase(), 0),
            Some("other-slug".to_string())
        );
        assert_eq!(
            named_catalogue_slug(&"not catalogued.".to_ascii_lowercase(), 0),
            None
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
            decisions: Vec::new(),
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

    fn decisions_only(records: &[(&str, &[&str])]) -> Vec<Finding> {
        check(&Corpus {
            decisions: records
                .iter()
                .map(|(path, keys)| LoadedDecision {
                    rel_path: (*path).to_string(),
                    keys: keys.iter().map(|k| (*k).to_string()).collect(),
                })
                .collect(),
            ..Default::default()
        })
    }

    #[test]
    fn a_decision_key_outside_the_schema_is_reported() {
        // The real case: `leadership-and-the-anomalies.yml` carried the four-biennium alignment
        // table — the evidence the entire record argues about — under `the_alignment:`. It was
        // on disk and correct, and `DecisionRecord` does not deserialize that field, so the
        // exported feed and every page built from it silently omitted the table.
        let f = decisions_only(&[(
            ".yidam/decisions/x.yml",
            &[
                "id",
                "summary",
                "the_alignment",
                "context",
                "decision",
                "rationale",
            ],
        )]);
        let stray: Vec<&Finding> = f
            .iter()
            .filter(|x| x.rule == "decision-key-outside-schema")
            .collect();
        assert_eq!(stray.len(), 1, "got {f:?}");
        assert_eq!(stray[0].severity, Severity::Error);
        assert!(
            stray[0].message.contains("the_alignment"),
            "{}",
            stray[0].message
        );
    }

    #[test]
    fn a_decision_using_only_schema_fields_is_clean() {
        assert!(decisions_only(&[(
            ".yidam/decisions/x.yml",
            &["id", "summary", "context", "decision", "rationale"],
        )])
        .is_empty());
    }

    #[test]
    fn corpus_depth_is_a_schema_field_and_not_reported() {
        // `ontology.yml` carries it. A rule that flagged it would fire on a record that is
        // correct, which is how a guard gets switched off.
        assert!(decisions_only(&[(
            ".yidam/decisions/ontology.yml",
            &[
                "id",
                "summary",
                "corpus_depth",
                "context",
                "decision",
                "rationale"
            ],
        )])
        .is_empty());
    }

    #[test]
    fn a_file_that_is_not_a_mapping_is_not_a_decision_record() {
        // `proposals.yml` holds a list. It never claimed to be a decision record, and loading
        // it yields no top-level keys, so the rule must have nothing to say about it.
        assert!(decisions_only(&[(".yidam/decisions/proposals.yml", &[])]).is_empty());
    }

    #[test]
    fn the_field_list_matches_what_the_schema_deserializes() {
        // DECISION_FIELDS is written by hand beside a struct it cannot see. If a field is added
        // to `DecisionRecord` and not here, the rule starts reporting a correct record as
        // broken — so round-trip a record naming every field and require it to parse.
        let yaml = DECISION_FIELDS
            .iter()
            .map(|f| match *f {
                "corpus_depth" => format!("{f}: 40"),
                _ => format!("{f}: x"),
            })
            .collect::<Vec<_>>()
            .join("\n");
        let parsed = serde_yaml::from_str::<corpus_schema::DecisionRecord>(&yaml);
        assert!(parsed.is_ok(), "DECISION_FIELDS has drifted: {parsed:?}");
    }

    /// A class with one inbound edge, `expected` or not, plus one lonely instance of it.
    fn orphan_case(expected: bool) -> Vec<Finding> {
        let mut classes = BTreeMap::new();
        classes.insert(
            "appropriation".to_string(),
            class(&format!(
                r#"
class: appropriation
label: Appropriation
description: d
edges:
  - relationship: grants-authority-for
    target: line-item
    direction: out
  - relationship: modifies
    target: budget-action
    direction: in
    expected: {expected}
"#
            )),
        );
        let mut paths = BTreeSet::new();
        paths.insert(PathBuf::from("corpus/appropriation.ont.yml"));
        run(
            classes,
            paths,
            vec![instance(
                "corpus/appropriation/a.yml",
                r#"
class: appropriation
label: A
description: d
links:
  - target: ../appropriation.ont.yml
    relationship: instance-of
  - target: ../line-item/ff.yml
    relationship: grants-authority-for
"#,
            )],
        )
    }

    #[test]
    fn a_node_that_is_pointed_at_is_wired_even_with_no_outgoing_edges() {
        // The regression. `fund` declares one outgoing edge, `transfers-to` another fund, and
        // most funds transfer to nothing while appropriations draw from them constantly. Five
        // of the corpus's eight funds were reported as making no domain links at all.
        let mut classes = BTreeMap::new();
        classes.insert(
            "fund".to_string(),
            class(
                r#"
class: fund
label: Fund
description: d
edges:
  - relationship: transfers-to
    target: fund
    direction: out
  - relationship: draws-from
    target: appropriation
    direction: in
"#,
            ),
        );
        classes.insert(
            "appropriation".to_string(),
            class(
                r#"
class: appropriation
label: Appropriation
description: d
edges:
  - relationship: draws-from
    target: fund
    direction: out
"#,
            ),
        );
        let mut paths = BTreeSet::new();
        paths.insert(PathBuf::from("corpus/fund.ont.yml"));
        paths.insert(PathBuf::from("corpus/appropriation.ont.yml"));
        let findings = run(
            classes,
            paths,
            vec![
                instance(
                    "corpus/fund/grf.yml",
                    "class: fund\nlabel: G\ndescription: d\nlinks:\n  \
                     - target: ../fund.ont.yml\n    relationship: instance-of\n",
                ),
                instance(
                    "corpus/appropriation/a.yml",
                    "class: appropriation\nlabel: A\ndescription: d\nlinks:\n  \
                     - target: ../appropriation.ont.yml\n    relationship: instance-of\n  \
                     - target: ../fund/grf.yml\n    relationship: draws-from\n",
                ),
            ],
        );
        assert!(
            !findings
                .iter()
                .any(|f| f.rule == "no-domain-link" && f.path.contains("grf")),
            "a fund drawn from is wired: {findings:?}"
        );
    }

    #[test]
    fn a_node_connected_in_neither_direction_is_still_reported() {
        let (classes, paths) = fixture();
        let findings = run(
            classes,
            paths,
            vec![instance(
                "corpus/fund/lonely.yml",
                "class: fund\nlabel: L\ndescription: d\nlinks:\n  \
                 - target: ../fund.ont.yml\n    relationship: instance-of\n",
            )],
        );
        assert!(findings.iter().any(|f| f.rule == "no-domain-link"));
    }

    #[test]
    fn an_inbound_edge_that_is_merely_declared_does_not_make_an_orphan() {
        // The regression. `appropriation <- modifies` exists because a veto can strike an
        // appropriation. A veto struck a handful; the other several hundred appropriations
        // were reported as orphans for not having been vetoed, which is 71 warnings about
        // correct data and the reason nobody read the other 29.
        let f = orphan_case(false);
        assert!(
            !f.iter().any(|x| x.rule == "orphan-in"),
            "declaring an inbound edge is not expecting one on every instance: {f:?}"
        );
    }

    #[test]
    fn an_expected_inbound_edge_still_reports_an_orphan() {
        // The case worth keeping: a line item with no appropriation is work not yet done.
        let f = orphan_case(true);
        let orphans: Vec<&Finding> = f.iter().filter(|x| x.rule == "orphan-in").collect();
        assert_eq!(orphans.len(), 1, "got {f:?}");
        assert!(
            orphans[0].message.contains("modifies"),
            "the message must name what would satisfy it: {}",
            orphans[0].message
        );
    }

    #[test]
    fn the_same_relationship_declared_twice_is_named_once() {
        // `fund` declares `draws-from` inbound from both `appropriation` and `line-item`.
        // Listing it twice reads as a bug in the validator rather than as two ways to satisfy
        // one requirement.
        let mut classes = BTreeMap::new();
        classes.insert(
            "fund".to_string(),
            class(
                r#"
class: fund
label: Fund
description: d
edges:
  - relationship: draws-from
    target: appropriation
    direction: in
    expected: true
  - relationship: draws-from
    target: line-item
    direction: in
    expected: true
"#,
            ),
        );
        let mut paths = BTreeSet::new();
        paths.insert(PathBuf::from("corpus/fund.ont.yml"));
        let f = run(
            classes,
            paths,
            vec![instance(
                "corpus/fund/grf.yml",
                "class: fund\nlabel: G\ndescription: d\nlinks:\n  - target: ../fund.ont.yml\n    relationship: instance-of\n",
            )],
        );
        let msg = &f
            .iter()
            .find(|x| x.rule == "orphan-in")
            .expect("orphan")
            .message;
        assert!(msg.contains(r#"["draws-from"]"#), "{msg}");
    }
}
