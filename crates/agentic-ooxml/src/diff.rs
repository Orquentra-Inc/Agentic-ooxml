//! `diff`: a structural comparison of two packages (before, after). It answers "what changed in
//! the knowledge, and in which native parts" without rendering anything.
//!
//! Three views:
//! * `ao_parts`: AO parts added, removed or changed. Knowledge parts are compared section by
//!   section; v0.2 context and legacy parts by their summary fields.
//! * `sources`: source records added, removed, or whose stored or effective status (or claim)
//!   changed. The effective status is recomputed from each file, so an edit to a claimed shape
//!   shows up here as `verified -> stale` even though the sources part itself was not touched.
//! * `native_parts`: every other package part, compared by content hash. `package_plumbing` lists
//!   `[Content_Types].xml`, relationships and `customXml/` payloads that differ, because adding an
//!   AO part touches them; their meaning is covered by `ao_parts`.
//!
//! This is a byte-level and structure-level diff. It does not interpret native content: a changed
//! slide is reported as a changed part with two hashes, not as "Q3 changed from 18.6 to 19.1".

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// A part whose bytes differ, appear or disappear.
#[derive(Debug, Clone, Serialize)]
pub struct PartChange {
    pub part: String,
    /// `blake3:<hex>` (or `crc32:<hex>/<size>` for a part over the read cap); absent when added.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// Absent when removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct PartDiff {
    pub added: Vec<PartChange>,
    pub removed: Vec<PartChange>,
    pub changed: Vec<PartChange>,
}

impl PartDiff {
    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

#[derive(Debug, Serialize)]
pub struct AoPartChange {
    pub kind: String,
    pub part: String,
    /// `added`, `removed` or `changed`.
    pub change: &'static str,
    /// Human-readable specifics for a `changed` part. Quotes document data.
    pub details: Vec<String>,
}

/// One side of a source record change.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SourceState {
    pub status: String,
    pub effective_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_part: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_hash: Option<String>,
    /// The external claim's locator and the value hash read now (when the record has one).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_value_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

impl From<&SourceView> for SourceState {
    fn from(v: &SourceView) -> Self {
        SourceState {
            status: v.record.status.clone(),
            effective_status: v.effective_status.clone(),
            stale_reason: v.stale_reason,
            claim_part: v.record.fields.claim_part.clone(),
            claim_object_id: v.record.fields.claim_object_id.clone(),
            claim_hash: v.record.claim_hash.clone(),
            external_locator: v
                .external_check
                .as_ref()
                .map(|e| format!("{}:{}", e.file, e.locator)),
            external_value_hash: v
                .external_check
                .as_ref()
                .and_then(|e| e.current_hash.clone()),
            external_reason: v.external_check.as_ref().and_then(|e| e.reason),
            resource_uri: v.record.fields.resource_uri.clone(),
            location: v.record.fields.location.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SourceChange {
    pub id: String,
    /// `added`, `removed` or `changed`.
    pub change: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<SourceState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<SourceState>,
}

#[derive(Debug, Serialize)]
pub struct DiffResult {
    pub before: String,
    pub after: String,
    pub trust: &'static str,
    /// True when nothing differs in any of the views below.
    pub identical: bool,
    pub ao_parts: Vec<AoPartChange>,
    pub sources: Vec<SourceChange>,
    pub native_parts: PartDiff,
    pub package_plumbing: PartDiff,
}

struct Snapshot {
    parts: Vec<DiscoveredPart>,
    knowledge: BTreeMap<String, KnowledgeDoc>,
    sources: BTreeMap<String, SourceView>,
    fingerprints: BTreeMap<String, String>,
}

fn fingerprint(zip: &mut Archive, name: &str) -> Result<String> {
    let (size, crc) = {
        let e = zip.by_name(name)?;
        (e.size(), e.crc32())
    };
    Ok(match read_bytes(zip, name, MAX_CLAIM_PART_BYTES) {
        Ok(Some(b)) => blake3_tag(&b),
        _ => format!("crc32:{crc:08x}/{size}"),
    })
}

fn snapshot(file: &Path, opts: &EvalOptions) -> Result<Snapshot> {
    let mut zip = open(file)?;
    let (main, found) = scan(&mut zip)?;
    let ctx = load_context(&mut zip, &main, &found)?;
    let mut parts: Vec<DiscoveredPart> = found
        .iter()
        .filter(|f| !matches!(f.parsed, Ok(Parsed::Doc(_)) | Ok(Parsed::Units(_))))
        .map(to_discovered)
        .collect();
    parts.extend(ctx.discovered());
    let mut knowledge = BTreeMap::new();
    for f in &found {
        if let Ok(Parsed::Knowledge(d)) = &f.parsed {
            knowledge.insert(f.part.clone(), d.clone());
        }
    }
    drop(found);
    let sources = crate::sources_with(file, None, None, opts)?
        .sources
        .into_iter()
        .map(|v| (v.record.id.clone(), v))
        .collect();
    let names: Vec<String> = zip
        .file_names()
        .filter(|n| !n.ends_with('/'))
        .map(str::to_string)
        .collect();
    let mut fingerprints = BTreeMap::new();
    for n in names {
        let fp = fingerprint(&mut zip, &n)?;
        fingerprints.insert(n, fp);
    }
    Ok(Snapshot {
        parts,
        knowledge,
        sources,
        fingerprints,
    })
}

fn short(s: &str) -> String {
    clip_chars(&s.split_whitespace().collect::<Vec<_>>().join(" "), 80)
}

fn knowledge_details(b: &KnowledgeDoc, a: &KnowledgeDoc) -> Vec<String> {
    let mut d = vec![];
    if b.schema_version != a.schema_version {
        d.push(format!(
            "schemaVersion: {} -> {}",
            b.schema_version, a.schema_version
        ));
    }
    if b.summary != a.summary {
        d.push(format!(
            "summary: \"{}\" -> \"{}\"",
            short(&b.summary),
            short(&a.summary)
        ));
    }
    let bs: BTreeMap<&str, &Section> = b.sections.iter().map(|s| (s.id.as_str(), s)).collect();
    let asx: BTreeMap<&str, &Section> = a.sections.iter().map(|s| (s.id.as_str(), s)).collect();
    for id in bs.keys().chain(asx.keys()).collect::<BTreeSet<_>>() {
        match (bs.get(id), asx.get(id)) {
            (None, Some(_)) => d.push(format!("section '{id}' added")),
            (Some(_), None) => d.push(format!("section '{id}' removed")),
            (Some(x), Some(y)) if x != y => {
                if x.format != y.format {
                    d.push(format!(
                        "section '{id}' format: {} -> {}",
                        x.format, y.format
                    ));
                }
                if x.text != y.text {
                    d.push(format!(
                        "section '{id}' text: \"{}\" -> \"{}\"",
                        short(&x.text),
                        short(&y.text)
                    ));
                }
            }
            _ => {}
        }
    }
    d
}

fn field_details(b: &DiscoveredPart, a: &DiscoveredPart) -> Vec<String> {
    let (bv, av) = (
        serde_json::to_value(b).unwrap_or_default(),
        serde_json::to_value(a).unwrap_or_default(),
    );
    let (Some(bo), Some(ao)) = (bv.as_object(), av.as_object()) else {
        return vec![];
    };
    let keys: BTreeSet<&String> = bo.keys().chain(ao.keys()).collect();
    keys.into_iter()
        .filter(|k| k.as_str() != "part" && bo.get(*k) != ao.get(*k))
        .map(|k| {
            let show = |v: Option<&serde_json::Value>| {
                v.map_or("(none)".to_string(), |v| short(&v.to_string()))
            };
            format!("{k}: {} -> {}", show(bo.get(k)), show(ao.get(k)))
        })
        .collect()
}

/// Compare two packages. Both must be readable packages; a broken AO part shows up with its
/// `error` field changing rather than failing the diff.
pub fn diff(before: &Path, after: &Path) -> Result<DiffResult> {
    diff_with(before, after, &EvalOptions::default())
}

/// `diff` with explicit limits for reading external claims (the MCP server passes its roots).
pub fn diff_with(before: &Path, after: &Path, opts: &EvalOptions) -> Result<DiffResult> {
    let b = snapshot(before, opts).with_context(|| format!("reading {}", before.display()))?;
    let a = snapshot(after, opts).with_context(|| format!("reading {}", after.display()))?;

    // AO parts, keyed by (kind, part): a renamed part reads as removed plus added.
    let key = |p: &DiscoveredPart| (p.kind.clone(), p.part.clone());
    let bm: BTreeMap<_, _> = b.parts.iter().map(|p| (key(p), p)).collect();
    let am: BTreeMap<_, _> = a.parts.iter().map(|p| (key(p), p)).collect();
    let mut ao_parts = vec![];
    for k in bm.keys().chain(am.keys()).collect::<BTreeSet<_>>() {
        let (kind, part) = k.clone();
        match (bm.get(k), am.get(k)) {
            (None, Some(_)) => ao_parts.push(AoPartChange {
                kind,
                part,
                change: "added",
                details: vec![],
            }),
            (Some(_), None) => ao_parts.push(AoPartChange {
                kind,
                part,
                change: "removed",
                details: vec![],
            }),
            (Some(x), Some(y)) => {
                let details = match (b.knowledge.get(&part), a.knowledge.get(&part)) {
                    (Some(kb), Some(ka)) => knowledge_details(kb, ka),
                    _ => field_details(x, y),
                };
                if !details.is_empty() {
                    ao_parts.push(AoPartChange {
                        kind,
                        part,
                        change: "changed",
                        details,
                    });
                }
            }
            (None, None) => {}
        }
    }

    // Sources by id.
    let mut sources = vec![];
    for id in b
        .sources
        .keys()
        .chain(a.sources.keys())
        .collect::<BTreeSet<_>>()
    {
        match (b.sources.get(id), a.sources.get(id)) {
            (None, Some(y)) => sources.push(SourceChange {
                id: id.clone(),
                change: "added",
                before: None,
                after: Some(y.into()),
            }),
            (Some(x), None) => sources.push(SourceChange {
                id: id.clone(),
                change: "removed",
                before: Some(x.into()),
                after: None,
            }),
            (Some(x), Some(y)) => {
                let (sx, sy): (SourceState, SourceState) = (x.into(), y.into());
                if sx != sy {
                    sources.push(SourceChange {
                        id: id.clone(),
                        change: "changed",
                        before: Some(sx),
                        after: Some(sy),
                    });
                }
            }
            (None, None) => {}
        }
    }

    // Parts by content hash.
    let is_plumbing =
        |n: &str| n == "[Content_Types].xml" || n.ends_with(".rels") || n.starts_with("customXml/");
    let mut native = PartDiff::default();
    let mut plumbing = PartDiff::default();
    for n in b
        .fingerprints
        .keys()
        .chain(a.fingerprints.keys())
        .collect::<BTreeSet<_>>()
    {
        let (x, y) = (b.fingerprints.get(n), a.fingerprints.get(n));
        if x == y {
            continue;
        }
        let dst = if is_plumbing(n) {
            &mut plumbing
        } else {
            &mut native
        };
        let c = PartChange {
            part: n.clone(),
            before: x.cloned(),
            after: y.cloned(),
        };
        match (x, y) {
            (None, _) => dst.added.push(c),
            (_, None) => dst.removed.push(c),
            _ => dst.changed.push(c),
        }
    }
    let identical =
        ao_parts.is_empty() && sources.is_empty() && native.is_empty() && plumbing.is_empty();
    Ok(DiffResult {
        before: before.display().to_string(),
        after: after.display().to_string(),
        trust: "document",
        identical,
        ao_parts,
        sources,
        native_parts: native,
        package_plumbing: plumbing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testpkg::*;

    fn claim(f: &Path) {
        let p: DocPatch = serde_json::from_str(r#"{"sources":[{"id":"q3","status":"verified","claim_part":"ppt/slides/slide1.xml","claim_object_id":"3","resource_uri":"financial-model.xlsx","location":"Summary!C4"}]}"#).unwrap();
        write(f, "sources", &p, false).unwrap();
    }

    #[test]
    fn identical_files_have_an_empty_diff() {
        let d = tmp_dir();
        let a = d.join("a.pptx");
        pptx(&a);
        let b = d.join("b.pptx");
        std::fs::copy(&a, &b).unwrap();
        let r = diff(&a, &b).unwrap();
        assert!(r.identical, "{r:?}");
    }

    #[test]
    fn diff_reports_ao_parts_native_hashes_and_source_status_changes() {
        let d = tmp_dir();
        let (a, b, c) = (d.join("a.pptx"), d.join("b.pptx"), d.join("c.pptx"));
        pptx(&a);
        // b = a + a design part + a verified claim on shape 3
        std::fs::copy(&a, &b).unwrap();
        write(
            &b,
            "design",
            &serde_json::from_str(r#"{"sections":[{"id":"type","text":"Inter"}]}"#).unwrap(),
            false,
        )
        .unwrap();
        claim(&b);
        let r = diff(&a, &b).unwrap();
        assert!(!r.identical);
        let kinds: Vec<_> = r
            .ao_parts
            .iter()
            .map(|p| (p.kind.as_str(), p.change))
            .collect();
        assert!(
            kinds.contains(&("design", "added")) && kinds.contains(&("sources", "added")),
            "{kinds:?}"
        );
        assert!(
            r.native_parts.is_empty(),
            "native parts must be untouched by AO writes: {:?}",
            r.native_parts
        );
        assert!(r
            .package_plumbing
            .added
            .iter()
            .any(|p| p.part == "customXml/ao-design.xml"));
        assert!(r
            .package_plumbing
            .changed
            .iter()
            .any(|p| p.part == "[Content_Types].xml"));
        assert_eq!(r.sources.len(), 1);
        assert_eq!(
            (
                r.sources[0].change,
                r.sources[0]
                    .after
                    .as_ref()
                    .unwrap()
                    .effective_status
                    .as_str()
            ),
            ("added", "verified")
        );

        // c = b with the claimed shape's text edited: native part changed, source verified -> stale
        std::fs::copy(&b, &c).unwrap();
        let slide = String::from_utf8(contents_of(&c, "ppt/slides/slide1.xml"))
            .unwrap()
            .replace("18.6", "19.1");
        rewrite_package(&c, &[("ppt/slides/slide1.xml".into(), slide.into_bytes())]).unwrap();
        let r = diff(&b, &c).unwrap();
        assert_eq!(r.native_parts.changed.len(), 1);
        assert_eq!(r.native_parts.changed[0].part, "ppt/slides/slide1.xml");
        assert!(
            r.native_parts.changed[0].before.is_some() && r.native_parts.changed[0].after.is_some()
        );
        assert_eq!(r.sources.len(), 1);
        let s = &r.sources[0];
        assert_eq!(s.change, "changed");
        assert_eq!(
            (
                s.before.as_ref().unwrap().effective_status.as_str(),
                s.after.as_ref().unwrap().effective_status.as_str()
            ),
            ("verified", "stale")
        );
        assert_eq!(
            s.after.as_ref().unwrap().stale_reason,
            Some("claim_changed")
        );
        assert!(
            r.ao_parts.is_empty(),
            "the sources part itself did not change"
        );
    }

    #[test]
    fn knowledge_sections_are_diffed_individually() {
        let d = tmp_dir();
        let (a, b) = (d.join("a.docx"), d.join("b.docx"));
        docx(&a);
        write(&a, "intent", &serde_json::from_str(r#"{"summary":"one","sections":[{"id":"goal","text":"Inform"},{"id":"old","text":"x"}]}"#).unwrap(), false).unwrap();
        std::fs::copy(&a, &b).unwrap();
        write(&b, "intent", &serde_json::from_str(r#"{"summary":"two","sections":[{"id":"goal","text":"Persuade"},{"id":"old","delete":true},{"id":"new","text":"y"}]}"#).unwrap(), false).unwrap();
        let r = diff(&a, &b).unwrap();
        let ch = r.ao_parts.iter().find(|p| p.kind == "intent").unwrap();
        assert_eq!(ch.change, "changed");
        let all = ch.details.join("\n");
        for want in [
            "summary:",
            "section 'goal' text: \"Inform\" -> \"Persuade\"",
            "section 'old' removed",
            "section 'new' added",
        ] {
            assert!(all.contains(want), "{want} missing in {all}");
        }
    }

    fn contents_of(f: &Path, name: &str) -> Vec<u8> {
        use std::io::Read;
        let mut z = zip::ZipArchive::new(std::fs::File::open(f).unwrap()).unwrap();
        let mut b = vec![];
        z.by_name(name).unwrap().read_to_end(&mut b).unwrap();
        b
    }
}
