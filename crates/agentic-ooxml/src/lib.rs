//! Agentic OOXML v0.1 knowledge parts: custom XML parts embedded in a .docx/.pptx/.xlsx that carry
//! agent editing knowledge (`agents`, `design`, `intent`) and source records (`sources`). The module is format-neutral: the main document part is found through the
//! `officeDocument` relationship in `_rels/.rels`, and every knowledge part is reached through a
//! `customXml` relationship from that main part. Discovery never trusts file names: a part is an
//! AO part only when its root element is in the AO namespace, so `customXml/item3.xml` written by
//! another editor is found just like `customXml/ao-design.xml`.
//!
//! It also READS (never writes) Agentic OOXML v0.2 context (`urn:agentic-office:context:v0.2`,
//! written by the TS MCP servers, model in `mcp/shared/aoContext.ts`): the file-level `<ao:doc>`
//! and its units, and lists the old TS writer's v0.1 `<ao:deck>` as `legacy-deck`. A file that
//! carries a v0.2 `<ao:doc>` refuses `knowledge-write`: one writer per file.
//!
//! Embedded text is untrusted document data. Every result that returns it carries
//! `"trust": "document"`; callers must never treat it as instructions.

use anyhow::{anyhow, bail, Context, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::reader::{NsReader, Reader};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::ops::Range;
use std::path::Path;

mod diff;
mod external;
mod inspect;
#[cfg(test)]
mod testpkg;
mod validate;
pub use diff::{
    diff, diff_with, AoPartChange, DiffResult, PartChange, PartDiff, SourceChange, SourceState,
};
pub use external::{EvalOptions, ExternalClaim, ExternalView};

use external::{evaluate_external, read_value, ExtCtx};
pub use inspect::{inspect, InspectResult, NativeUnit, PartInfo};
pub use validate::{validate, validate_with, Issue, ValidateOptions, ValidateResult};

pub const AO_NS: &str = "urn:agentic-office:ooxml:v0.1";
pub const SCHEMA_VERSION: &str = "0.1";
/// Agentic OOXML v0.2, the canonical embedded context written by the TS MCP servers
/// (`mcp/shared/aoContext.ts`). This kernel reads it and never writes it: one writer per file.
pub const CTX_NS: &str = "urn:agentic-office:context:v0.2";
/// The kinds `knowledge-read` answers from v0.2 (and the legacy v0.1 `<ao:deck>`) parts.
pub const CONTEXT_KINDS: [&str; 4] = ["doc", "units", "legacy-deck", "spec"];
const DS_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/customXml";
const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const REL_OFFICE_DOCUMENT: &str = "/relationships/officeDocument";
const REL_CUSTOM_XML: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";
const REL_CUSTOM_XML_PROPS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXmlProps";
const CT_PROPS: &str = "application/vnd.openxmlformats-officedocument.customXmlProperties+xml";
const PML_NS: &[u8] = b"http://schemas.openxmlformats.org/presentationml/2006/main";

/// Safety caps (spec §3.3, §8): a package with more entries than this is refused, and no AO part
/// (or package part this module edits) may exceed this many uncompressed bytes.
const MAX_ENTRIES: usize = 10_000;
const MAX_PART_BYTES: u64 = 2 * 1024 * 1024;
/// A claim part is native content (a slide, the document body), which can legitimately be larger
/// than an AO part; it is only hashed, never parsed beyond a shape search, so the cap is looser.
const MAX_CLAIM_PART_BYTES: u64 = 64 * 1024 * 1024;

/// The `ao:knowledge` kinds; `sources` has its own root element and model.
const KNOWLEDGE_KINDS: [&str; 3] = ["agents", "design", "intent"];
const SOURCE_STATUSES: [&str; 4] = ["verified", "unverified", "stale", "manual"];
/// The PresentationML elements that carry their own `p:nvXxPr/p:cNvPr` id (an object-level claim).
const PML_SHAPES: [&[u8]; 5] = [b"sp", b"graphicFrame", b"pic", b"cxnSp", b"grpSp"];

// ---------------------------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub format: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeDoc {
    pub kind: String,
    pub schema_version: String,
    pub summary: String,
    pub sections: Vec<Section>,
}

/// One AO part found in a package. A part that cannot be read or parsed is still listed, with
/// `error` set, so one broken part never hides the others.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredPart {
    pub kind: String,
    pub part: String,
    pub schema_version: String,
    pub summary: String,
    pub sections: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_count: Option<usize>,
    /// v0.2 `doc`/`units` and `legacy-deck`: the stored revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<u64>,
    /// v0.2 `units`: how many units, and how many are stale / orphaned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_count: Option<usize>,
    /// v0.2 `units` on a .pptx: one part per slide, so every part is named here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_parts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListResult {
    pub file: String,
    pub format: String,
    pub trust: &'static str,
    pub parts: Vec<DiscoveredPart>,
}

#[derive(Debug, Serialize)]
pub struct ReadResult {
    pub file: String,
    pub kind: String,
    pub trust: &'static str,
    pub summary: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Serialize)]
pub struct WriteResult {
    pub part: String,
    pub created: bool,
    pub sections: Vec<String>,
    /// The record ids after a `sources` write (absent for the other kinds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,
}

/// The `knowledge-write` input: every field optional, sections (or, for kind `sources`, source
/// records) upserted by id.
#[derive(Debug, Default, Deserialize)]
pub struct DocPatch {
    pub summary: Option<String>,
    #[serde(default)]
    pub sections: Vec<SectionPatch>,
    #[serde(default)]
    pub sources: Vec<SourcePatch>,
}

#[derive(Debug, Deserialize)]
pub struct SectionPatch {
    pub id: String,
    pub format: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub delete: bool,
}

/// The optional fields of a source record (Appendix B), shared by the stored record and the
/// write patch. `claim_part` is an OPC part name (`/ppt/slides/slide1.xml`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_part: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_hash_algorithm: Option<String>,
    /// Optional cross-document claim (`ao:externalClaim`): a value in another file next to this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_claim: Option<ExternalClaim>,
}

impl SourceFields {
    /// Overwrite every field the patch sets. A claim part is stored in OPC form (leading `/`).
    fn merge(&mut self, p: &SourceFields) {
        fn set(dst: &mut Option<String>, src: &Option<String>) {
            if src.is_some() {
                *dst = src.clone();
            }
        }
        set(&mut self.source_kind, &p.source_kind);
        set(&mut self.checked_at, &p.checked_at);
        set(&mut self.resource_uri, &p.resource_uri);
        set(&mut self.location, &p.location);
        set(
            &mut self.claim_part,
            &p.claim_part
                .as_ref()
                .map(|v| format!("/{}", v.trim_start_matches('/'))),
        );
        set(&mut self.claim_object_id, &p.claim_object_id);
        set(&mut self.source_hash, &p.source_hash);
        set(&mut self.source_hash_algorithm, &p.source_hash_algorithm);
        if let Some(e) = &p.external_claim {
            self.external_claim = Some(e.merged_over(self.external_claim.as_ref()));
        }
    }
}

/// One `ao:source`. `claim_hash` is the blake3 of the claim (object or part) taken when the record
/// was last written as `verified`; only the kernel sets it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceRecord {
    pub id: String,
    pub status: String,
    #[serde(flatten)]
    pub fields: SourceFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_hash: Option<String>,
}

/// One record in a `knowledge-write --kind sources` input. A supplied `claim_hash` is not a field
/// here, so it is ignored: the kernel computes it.
#[derive(Debug, Deserialize)]
pub struct SourcePatch {
    pub id: String,
    pub status: Option<String>,
    #[serde(flatten)]
    pub fields: SourceFields,
    #[serde(default)]
    pub delete: bool,
}

/// A source record as `knowledge-sources` reports it: the stored record plus the status computed
/// from the current file.
#[derive(Debug, Serialize)]
pub struct SourceView {
    #[serde(flatten)]
    pub record: SourceRecord,
    pub effective_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<&'static str>,
    /// Every reason, when there is more than one (external reasons come first).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stale_reasons: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<&'static str>,
    /// The computed state of the record's external claim: the value read now and whether it matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_check: Option<ExternalView>,
}

#[derive(Debug, Serialize)]
pub struct SourcesResult {
    pub file: String,
    pub trust: &'static str,
    pub sources: Vec<SourceView>,
    /// v0.2 sources (file-scope ones from `<ao:doc>`, unit-scope ones with their unit).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context_sources: Vec<CtxSourceView>,
}

#[derive(Debug, Serialize)]
pub struct Invalidated {
    pub id: String,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ReconcileResult {
    pub file: String,
    pub invalidated: Vec<Invalidated>,
    pub unchanged: usize,
    pub written: bool,
}

/// The current hash of a claim. `granularity` is `object` when the hash covers one slide shape and
/// `part` when it covers the whole part (including every fallback).
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimHash {
    pub hash: Option<String>,
    pub granularity: &'static str,
    pub object_missing: bool,
    pub part_missing: bool,
}

// ---------------------------------------------------------------------------------------------
// XML: parse and serialize
// ---------------------------------------------------------------------------------------------

fn is_ao(ns: &ResolveResult) -> bool {
    matches!(ns, ResolveResult::Bound(Namespace(n)) if *n == AO_NS.as_bytes())
}

fn is_ctx(ns: &ResolveResult) -> bool {
    matches!(ns, ResolveResult::Bound(Namespace(n)) if *n == CTX_NS.as_bytes())
}

fn attr(e: &BytesStart, key: &str) -> Result<Option<String>> {
    for a in e.attributes() {
        let a = a?;
        if a.key.as_ref() == key.as_bytes() {
            return Ok(Some(a.unescape_value()?.into_owned()));
        }
    }
    Ok(None)
}

// `<!DOCTYPE` is rejected where the parser reports it (`Event::DocType`, i.e. in the prolog), not
// by a substring search: section bodies are CDATA and may legitimately mention `<!DOCTYPE html>`.
const DOCTYPE_ERR: &str = "AO part contains <!DOCTYPE, which is not allowed";

/// Read the root element: `Some(kind)` when it is in the AO namespace (`sources` for an
/// `ao:sources` root, the `kind` attribute for `ao:knowledge`, `legacy-deck` for the old TS
/// writer's `ao:deck`; `doc`, `units` or `unit` for a v0.2 root), `None` for a foreign root.
fn sniff_root(xml: &str) -> Result<Option<String>> {
    let mut r = NsReader::from_str(xml);
    loop {
        match r.read_resolved_event()? {
            (ns, Event::Start(e)) | (ns, Event::Empty(e)) => {
                if is_ctx(&ns) {
                    return Ok(Some(match e.local_name().as_ref() {
                        b"doc" => "doc".to_string(),
                        b"units" => "units".to_string(),
                        b"unit" => "unit".to_string(),
                        b"spec" => "spec".to_string(),
                        other => bail!(
                            "unknown v0.2 context root element '{}'",
                            String::from_utf8_lossy(other)
                        ),
                    }));
                }
                if !is_ao(&ns) {
                    return Ok(None);
                }
                return Ok(Some(match e.local_name().as_ref() {
                    b"sources" => "sources".to_string(),
                    b"deck" => "legacy-deck".to_string(),
                    b"knowledge" => attr(&e, "kind")?.unwrap_or_else(|| "unknown".into()),
                    other => bail!(
                        "unknown AO root element '{}'",
                        String::from_utf8_lossy(other)
                    ),
                }));
            }
            (_, Event::DocType(_)) => bail!(DOCTYPE_ERR),
            (_, Event::Eof) => bail!("no root element"),
            _ => {}
        }
    }
}

/// Parse an `ao:knowledge` part. Unknown child elements are skipped (forward compatibility);
/// summary and section bodies keep their text exactly (no trimming).
pub fn parse_knowledge(xml: &str) -> Result<KnowledgeDoc> {
    let mut r = NsReader::from_str(xml);
    let mut doc: Option<KnowledgeDoc> = None;
    let mut depth = 0usize;
    // Where text at depth 2 goes: None, the summary, or the last section.
    let mut capture: Option<bool> = None; // Some(true) = summary, Some(false) = last section
    loop {
        let (ns, ev) = r.read_resolved_event()?;
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(ev, Event::Empty(_));
                if depth == 0 {
                    if !is_ao(&ns) || e.local_name().as_ref() != b"knowledge" {
                        bail!("root element is not ao:knowledge");
                    }
                    let kind = attr(e, "kind")?
                        .ok_or_else(|| anyhow!("ao:knowledge has no kind attribute"))?;
                    if !KNOWLEDGE_KINDS.contains(&kind.as_str()) {
                        bail!("unknown knowledge kind '{kind}'");
                    }
                    let schema_version = attr(e, "schemaVersion")?.unwrap_or_default();
                    doc = Some(KnowledgeDoc {
                        kind,
                        schema_version,
                        summary: String::new(),
                        sections: vec![],
                    });
                } else if depth == 1 && is_ao(&ns) {
                    let d = doc.as_mut().expect("root seen");
                    match e.local_name().as_ref() {
                        b"summary" => capture = Some(true),
                        b"section" => {
                            let id = attr(e, "id")?
                                .ok_or_else(|| anyhow!("ao:section has no id attribute"))?;
                            let format =
                                attr(e, "format")?.unwrap_or_else(|| "text/markdown".into());
                            d.sections.push(Section {
                                id,
                                format,
                                text: String::new(),
                            });
                            capture = Some(false);
                        }
                        _ => {}
                    }
                }
                if empty {
                    if depth == 1 {
                        capture = None;
                    }
                } else {
                    depth += 1;
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| anyhow!("unbalanced end tag"))?;
                if depth == 1 {
                    capture = None;
                }
            }
            Event::Text(ref t) if depth == 2 => {
                if let (Some(kind), Some(d)) = (capture, doc.as_mut()) {
                    let s = t.unescape()?;
                    if kind {
                        d.summary.push_str(&s)
                    } else {
                        d.sections.last_mut().expect("section").text.push_str(&s)
                    }
                }
            }
            Event::CData(ref c) if depth == 2 => {
                if let (Some(kind), Some(d)) = (capture, doc.as_mut()) {
                    let s = std::str::from_utf8(c).context("CDATA is not UTF-8")?;
                    if kind {
                        d.summary.push_str(s)
                    } else {
                        d.sections.last_mut().expect("section").text.push_str(s)
                    }
                }
            }
            Event::DocType(_) => bail!(DOCTYPE_ERR),
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 {
        bail!("unclosed element at end of part");
    }
    doc.ok_or_else(|| anyhow!("no root element"))
}

/// Parse an `ao:sources` part into its schema version and records. Unknown child elements are
/// skipped; a record with no `status` is read as `unverified` (prefer visible unverified).
pub fn parse_sources(xml: &str) -> Result<(String, Vec<SourceRecord>)> {
    let mut r = NsReader::from_str(xml);
    let (mut depth, mut version, mut recs) = (0usize, String::new(), Vec::<SourceRecord>::new());
    let mut in_source = false;
    // Where text inside a level-2 element goes: Some(true) = location, Some(false) = sourceHash.
    let mut capture: Option<bool> = None;
    loop {
        let (ns, ev) = r.read_resolved_event()?;
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let local = e.local_name();
                if depth == 0 {
                    if !is_ao(&ns) || local.as_ref() != b"sources" {
                        bail!("root element is not ao:sources");
                    }
                    version = attr(e, "schemaVersion")?.unwrap_or_default();
                } else if depth == 1 && is_ao(&ns) && local.as_ref() == b"source" {
                    let id = attr(e, "id")?
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| anyhow!("ao:source has no id attribute"))?;
                    recs.push(SourceRecord {
                        id,
                        status: attr(e, "status")?.unwrap_or_else(|| "unverified".into()),
                        fields: SourceFields {
                            source_kind: attr(e, "sourceKind")?,
                            checked_at: attr(e, "checkedAt")?,
                            ..Default::default()
                        },
                        claim_hash: None,
                    });
                    in_source = true;
                } else if depth == 2 && in_source && is_ao(&ns) {
                    let rec = recs.last_mut().expect("in a source");
                    match local.as_ref() {
                        b"resource" => rec.fields.resource_uri = attr(e, "uri")?,
                        b"externalClaim" => {
                            if let Some(file) = attr(e, "file")?.filter(|f| !f.is_empty()) {
                                rec.fields.external_claim = Some(ExternalClaim {
                                    file,
                                    sheet: attr(e, "sheet")?,
                                    cell: attr(e, "cell")?,
                                    part: attr(e, "part")?,
                                    object_id: attr(e, "objectLocalId")?,
                                    claim_hash: attr(e, "claimHash")?,
                                });
                            }
                        }
                        b"claimLocator" => {
                            rec.fields.claim_part = attr(e, "part")?;
                            rec.fields.claim_object_id = attr(e, "objectLocalId")?;
                            rec.claim_hash = attr(e, "claimHash")?;
                        }
                        b"location" => {
                            rec.fields.location = Some(String::new());
                            capture = Some(true);
                        }
                        b"sourceHash" => {
                            rec.fields.source_hash_algorithm = attr(e, "algorithm")?;
                            rec.fields.source_hash = Some(String::new());
                            capture = Some(false);
                        }
                        _ => {}
                    }
                }
                if matches!(ev, Event::Start(_)) {
                    depth += 1;
                } else if depth == 1 {
                    in_source = false;
                } else if depth == 2 {
                    capture = None;
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| anyhow!("unbalanced end tag"))?;
                match depth {
                    1 => in_source = false,
                    2 => capture = None,
                    _ => {}
                }
            }
            Event::Text(ref t) if depth == 3 && capture.is_some() => {
                let rec = recs.last_mut().expect("in a source");
                let dst = if capture == Some(true) {
                    &mut rec.fields.location
                } else {
                    &mut rec.fields.source_hash
                };
                dst.get_or_insert_with(String::new).push_str(&t.unescape()?);
            }
            Event::CData(ref c) if depth == 3 && capture.is_some() => {
                let rec = recs.last_mut().expect("in a source");
                let dst = if capture == Some(true) {
                    &mut rec.fields.location
                } else {
                    &mut rec.fields.source_hash
                };
                dst.get_or_insert_with(String::new)
                    .push_str(std::str::from_utf8(c).context("CDATA is not UTF-8")?);
            }
            Event::DocType(_) => bail!(DOCTYPE_ERR),
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 {
        bail!("unclosed element at end of part");
    }
    Ok((version, recs))
}

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn esc_attr(s: &str) -> String {
    esc_text(s).replace('"', "&quot;").replace('\'', "&apos;")
}

/// Refuse a value that XML 1.0 cannot carry at all (C0 controls other than tab, LF and CR, and the
/// noncharacters U+FFFE/U+FFFF): escaping does not help, and one such character makes the part
/// not well-formed, which Office may report as a corrupt document.
fn check_xml_chars(field: &str, s: &str) -> Result<()> {
    if let Some(c) = s.chars().find(|&c| matches!(c, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')) {
        bail!("{field} contains U+{:04X}, which XML 1.0 cannot represent", c as u32);
    }
    Ok(())
}

/// Whether a stored part's `schemaVersion` is newer than the one this module writes. A v0.1 writer
/// must not rewrite a newer part: it would silently drop the fields it does not know. Versions
/// compare numerically per dot segment; a missing version counts as 0.1 and an unparseable one as
/// newer (we cannot tell what it holds).
fn is_newer_schema(v: &str) -> bool {
    if v.is_empty() {
        return false;
    }
    let parse = |v: &str| {
        v.split('.')
            .map(|x| x.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()
    };
    match (parse(v), parse(SCHEMA_VERSION)) {
        (Ok(a), Ok(b)) => a > b,
        _ => true,
    }
}

/// CDATA cannot contain its own terminator, so `]]>` is split across two CDATA sections.
fn cdata(s: &str) -> String {
    format!("<![CDATA[{}]]>", s.replace("]]>", "]]]]><![CDATA[>"))
}

pub fn serialize_knowledge(doc: &KnowledgeDoc) -> String {
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    s.push_str(&format!(
        "<ao:knowledge xmlns:ao=\"{AO_NS}\" kind=\"{}\" schemaVersion=\"{}\">\n",
        esc_attr(&doc.kind),
        esc_attr(&doc.schema_version)
    ));
    s.push_str(&format!(
        "  <ao:summary>{}</ao:summary>\n",
        esc_text(&doc.summary)
    ));
    for sec in &doc.sections {
        s.push_str(&format!(
            "  <ao:section id=\"{}\" format=\"{}\">{}</ao:section>\n",
            esc_attr(&sec.id),
            esc_attr(&sec.format),
            cdata(&sec.text)
        ));
    }
    s.push_str("</ao:knowledge>\n");
    s
}

pub fn serialize_sources(recs: &[SourceRecord]) -> String {
    fn opt(key: &str, v: &Option<String>) -> String {
        v.as_ref()
            .map(|v| format!(" {key}=\"{}\"", esc_attr(v)))
            .unwrap_or_default()
    }
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    s.push_str(&format!(
        "<ao:sources xmlns:ao=\"{AO_NS}\" schemaVersion=\"{SCHEMA_VERSION}\">\n"
    ));
    for r in recs {
        let f = &r.fields;
        s.push_str(&format!(
            "  <ao:source id=\"{}\" status=\"{}\"{}{}>\n",
            esc_attr(&r.id),
            esc_attr(&r.status),
            opt("sourceKind", &f.source_kind),
            opt("checkedAt", &f.checked_at)
        ));
        if f.resource_uri.is_some() {
            s.push_str(&format!(
                "    <ao:resource{}/>\n",
                opt("uri", &f.resource_uri)
            ));
        }
        if let Some(l) = &f.location {
            s.push_str(&format!("    <ao:location>{}</ao:location>\n", esc_text(l)));
        }
        if f.claim_part.is_some() || f.claim_object_id.is_some() || r.claim_hash.is_some() {
            s.push_str(&format!(
                "    <ao:claimLocator{}{}{}/>\n",
                opt("part", &f.claim_part),
                opt("objectLocalId", &f.claim_object_id),
                opt("claimHash", &r.claim_hash)
            ));
        }
        if let Some(e) = &f.external_claim {
            s.push_str(&format!(
                "    <ao:externalClaim file=\"{}\"{}{}{}{}{}/>\n",
                esc_attr(&e.file),
                opt("sheet", &e.sheet),
                opt("cell", &e.cell),
                opt("part", &e.part),
                opt("objectLocalId", &e.object_id),
                opt("claimHash", &e.claim_hash)
            ));
        }
        if f.source_hash.is_some() || f.source_hash_algorithm.is_some() {
            s.push_str(&format!(
                "    <ao:sourceHash{}>{}</ao:sourceHash>\n",
                opt("algorithm", &f.source_hash_algorithm),
                esc_text(f.source_hash.as_deref().unwrap_or(""))
            ));
        }
        s.push_str("  </ao:source>\n");
    }
    s.push_str("</ao:sources>\n");
    s
}

// ---------------------------------------------------------------------------------------------
// Package: open, relationships, rewrite
// ---------------------------------------------------------------------------------------------

type Archive = zip::ZipArchive<std::fs::File>;

/// Open a package and apply the entry-level safety checks: entry count cap and no entry name
/// that could escape the package (a `..` segment or a leading `/`).
fn open(file: &Path) -> Result<Archive> {
    let f = std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?;
    let zip = zip::ZipArchive::new(f)
        .with_context(|| format!("{} is not a zip package", file.display()))?;
    if zip.len() > MAX_ENTRIES {
        bail!("package has {} entries (cap {MAX_ENTRIES})", zip.len());
    }
    for name in zip.file_names() {
        if name.split('/').any(|seg| seg == "..") || name.starts_with('/') {
            bail!("unsafe zip entry name '{name}'");
        }
    }
    Ok(zip)
}

/// Read one entry through a bounded reader (the declared size is not trusted).
/// `Ok(None)` when the entry does not exist.
fn read_bytes(zip: &mut Archive, name: &str, cap: u64) -> Result<Option<Vec<u8>>> {
    let entry = match zip.by_name(name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut buf = Vec::new();
    entry.take(cap + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > cap {
        bail!("part {name} exceeds {cap} bytes");
    }
    Ok(Some(buf))
}

/// Read one entry as UTF-8, capped at `MAX_PART_BYTES`.
fn read_entry(zip: &mut Archive, name: &str) -> Result<Option<String>> {
    match read_bytes(zip, name, MAX_PART_BYTES)? {
        Some(buf) => Ok(Some(
            String::from_utf8(buf).with_context(|| format!("part {name} is not UTF-8"))?,
        )),
        None => Ok(None),
    }
}

struct Rel {
    id: String,
    rel_type: String,
    target: String,
    external: bool,
}

fn parse_rels(xml: &str) -> Result<Vec<Rel>> {
    let mut r = Reader::from_str(xml);
    let mut out = vec![];
    loop {
        match r.read_event()? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"Relationship" => out
                .push(Rel {
                    id: attr(&e, "Id")?.unwrap_or_default(),
                    rel_type: attr(&e, "Type")?.unwrap_or_default(),
                    target: attr(&e, "Target")?.unwrap_or_default(),
                    external: attr(&e, "TargetMode")?.as_deref() == Some("External"),
                }),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

/// The `.rels` part that holds a part's relationships: `word/document.xml` →
/// `word/_rels/document.xml.rels`.
fn rels_path(part: &str) -> String {
    match part.rsplit_once('/') {
        Some((dir, name)) => format!("{dir}/_rels/{name}.rels"),
        None => format!("_rels/{part}.rels"),
    }
}

/// Resolve a relationship target against its source part (`""` for the package root) into a zip
/// entry name. A target that climbs out of the package is refused.
fn resolve_target(source: &str, target: &str) -> Result<String> {
    let mut segs: Vec<&str> = if target.starts_with('/') {
        vec![]
    } else {
        source
            .rsplit_once('/')
            .map(|(d, _)| d.split('/').collect())
            .unwrap_or_default()
    };
    for seg in target.trim_start_matches('/').split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                segs.pop()
                    .ok_or_else(|| anyhow!("relationship target '{target}' leaves the package"))?;
            }
            s => segs.push(s),
        }
    }
    Ok(segs.join("/"))
}

/// The main document part: the target of the `officeDocument` relationship in `_rels/.rels`.
fn main_part(zip: &mut Archive) -> Result<String> {
    let rels =
        read_entry(zip, "_rels/.rels")?.ok_or_else(|| anyhow!("package has no _rels/.rels"))?;
    let rel = parse_rels(&rels)?
        .into_iter()
        .find(|r| !r.external && r.rel_type.ends_with(REL_OFFICE_DOCUMENT))
        .ok_or_else(|| anyhow!("_rels/.rels has no officeDocument relationship"))?;
    resolve_target("", &rel.target)
}

fn format_of(main: &str) -> &'static str {
    match main.split('/').next() {
        Some("word") => "docx",
        Some("ppt") => "pptx",
        Some("xl") => "xlsx",
        _ => "unknown",
    }
}

/// Rewrite a package atomically: `replace` holds new bytes for existing entry names (written in
/// the original position) and for new names (appended in the given order). Every other entry is
/// copied raw, byte-for-byte, with its original compression. The new package is built in a temp
/// file next to the original and renamed over it.
pub fn rewrite_package(file: &Path, replace: &[(String, Vec<u8>)]) -> Result<()> {
    let mut zip = open(file)?;
    // The rewrite replaces the file, so it keeps the original's permissions, and a read-only file
    // is refused rather than silently replaced.
    let perms = std::fs::metadata(file)
        .with_context(|| format!("stat {}", file.display()))?
        .permissions();
    if perms.readonly() {
        bail!("{} is read-only", file.display());
    }
    let dir = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = file
        .file_name()
        .ok_or_else(|| anyhow!("no file name"))?
        .to_string_lossy();
    let tmp = dir.join(format!(".{name}.ao-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut w = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut written = std::collections::HashSet::new();
        for i in 0..zip.len() {
            let entry = zip.by_index_raw(i)?;
            let entry_name = entry.name().to_string();
            if let Some((_, bytes)) = replace.iter().find(|(n, _)| *n == entry_name) {
                drop(entry);
                w.start_file(entry_name.as_str(), opts)?;
                w.write_all(bytes)?;
                written.insert(entry_name);
            } else {
                w.raw_copy_file(entry)?;
            }
        }
        for (n, bytes) in replace.iter().filter(|(n, _)| !written.contains(n)) {
            w.start_file(n.as_str(), opts)?;
            w.write_all(bytes)?;
        }
        w.finish()?.sync_all()?;
        std::fs::set_permissions(&tmp, perms.clone())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, file).with_context(|| format!("replace {}", file.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Discovery, read, write
// ---------------------------------------------------------------------------------------------

enum Parsed {
    Knowledge(KnowledgeDoc),
    Sources {
        schema_version: String,
        records: Vec<SourceRecord>,
    },
    /// v0.2 file context (`ao:doc`).
    Doc(CtxDoc),
    /// v0.2 units: an `ao:units` container (docx/xlsx), or a single `ao:unit` part.
    Units(Vec<CtxUnit>),
    /// The requirement contract (`ao:spec`): what was asked, assumed, left open and must be proved.
    Spec(CtxSpec),
    /// v0.1 `ao:deck` from the old TS writer.
    LegacyDeck(LegacyDeck),
}

struct Found {
    part: String,
    kind: String,
    parsed: Result<Parsed>,
}

/// Every AO part reachable from the main part by a `customXml` relationship. A target that is
/// missing, oversized or unreadable before its root is known is reported with kind `unknown`.
fn scan(zip: &mut Archive) -> Result<(String, Vec<Found>)> {
    let main = main_part(zip)?;
    let rels = match read_entry(zip, &rels_path(&main))? {
        Some(x) => parse_rels(&x)?,
        None => vec![],
    };
    let mut out = vec![];
    for rel in rels
        .iter()
        .filter(|r| !r.external && r.rel_type == REL_CUSTOM_XML)
    {
        let part = match resolve_target(&main, &rel.target) {
            Ok(p) => p,
            Err(e) => {
                out.push(Found {
                    part: rel.target.clone(),
                    kind: "unknown".into(),
                    parsed: Err(e),
                });
                continue;
            }
        };
        let xml = match read_entry(zip, &part) {
            Ok(Some(x)) => x,
            Ok(None) => {
                out.push(Found {
                    part: part.clone(),
                    kind: "unknown".into(),
                    parsed: Err(anyhow!("part {part} is missing")),
                });
                continue;
            }
            Err(e) => {
                out.push(Found {
                    part,
                    kind: "unknown".into(),
                    parsed: Err(e),
                });
                continue;
            }
        };
        let kind = match sniff_root(&xml) {
            Ok(Some(k)) => k,
            Ok(None) => continue, // a foreign custom XML part (e.g. SharePoint metadata)
            Err(e) => {
                out.push(Found {
                    part,
                    kind: "unknown".into(),
                    parsed: Err(e),
                });
                continue;
            }
        };
        let parsed = match kind.as_str() {
            "sources" => parse_sources(&xml).map(|(schema_version, records)| Parsed::Sources {
                schema_version,
                records,
            }),
            "doc" | "units" | "unit" | "legacy-deck" | "spec" => parse_context_part(&kind, &xml),
            _ => parse_knowledge(&xml).map(Parsed::Knowledge),
        };
        out.push(Found { part, kind, parsed });
    }
    Ok((main, out))
}

/// Every embedded part: v0.1 knowledge/sources parts one per part; v0.2 as one `doc` entry and one
/// `units` entry (aggregating the per-slide parts of a .pptx); a v0.1 `ao:deck` as `legacy-deck`.
pub fn discover(file: &Path) -> Result<Vec<DiscoveredPart>> {
    let mut zip = open(file)?;
    let (main, found) = scan(&mut zip)?;
    let ctx = load_context(&mut zip, &main, &found)?;
    let mut out: Vec<DiscoveredPart> = found
        .iter()
        .filter(|f| !matches!(f.parsed, Ok(Parsed::Doc(_)) | Ok(Parsed::Units(_))))
        .map(to_discovered)
        .collect();
    out.extend(ctx.discovered());
    Ok(out)
}

fn empty_discovered(kind: String, part: String) -> DiscoveredPart {
    DiscoveredPart {
        kind,
        part,
        schema_version: String::new(),
        summary: String::new(),
        sections: vec![],
        source_count: None,
        rev: None,
        unit_count: None,
        stale_count: None,
        orphan_count: None,
        unit_parts: None,
        error: None,
    }
}

fn to_discovered(f: &Found) -> DiscoveredPart {
    let mut d = empty_discovered(f.kind.clone(), f.part.clone());
    match &f.parsed {
        Ok(Parsed::Knowledge(doc)) => {
            d.schema_version = doc.schema_version.clone();
            d.summary = doc.summary.clone();
            d.sections = doc.sections.iter().map(|s| s.id.clone()).collect();
        }
        Ok(Parsed::Sources {
            schema_version,
            records,
        }) => {
            d.schema_version = schema_version.clone();
            d.source_count = Some(records.len());
        }
        Ok(Parsed::LegacyDeck(deck)) => {
            d.schema_version = deck.schema_version.clone();
            d.rev = Some(deck.revision);
            d.source_count = Some(deck.sources);
            d.summary = deck.summary();
        }
        Ok(Parsed::Spec(spec)) => {
            d.schema_version = "0.2".into();
            d.rev = Some(spec.rev);
            d.summary = spec.summary();
        }
        // v0.2 parts are summarized by `CtxState::discovered`; one reaching here was unreadable.
        Ok(Parsed::Doc(_)) | Ok(Parsed::Units(_)) => {}
        Err(e) => d.error = Some(format!("{e:#}")),
    }
    d
}

pub fn list(file: &Path) -> Result<ListResult> {
    let format = format_of(&main_part(&mut open(file)?)?).to_string();
    Ok(ListResult {
        file: file.display().to_string(),
        format,
        trust: "document",
        parts: discover(file)?,
    })
}

pub fn read(file: &Path, kind: &str, section: Option<&str>) -> Result<ReadResult> {
    if kind == "sources" {
        bail!("the sources part is read with knowledge-sources");
    }
    let mut zip = open(file)?;
    let found = scan(&mut zip)?
        .1
        .into_iter()
        .find(|f| f.kind == kind)
        .ok_or_else(|| anyhow!("no '{kind}' knowledge part in {}", file.display()))?;
    let doc = match found.parsed? {
        Parsed::Knowledge(d) => d,
        _ => bail!("the '{kind}' part is not an ao:knowledge part"),
    };
    let sections = match section {
        None => doc.sections,
        Some(id) => {
            let ids: Vec<&str> = doc.sections.iter().map(|s| s.id.as_str()).collect();
            let sec = doc
                .sections
                .iter()
                .find(|s| s.id == id)
                .cloned()
                .ok_or_else(|| {
                    anyhow!(
                        "no section '{id}' in the '{kind}' part; available: [{}]",
                        ids.join(", ")
                    )
                })?;
            vec![sec]
        }
    };
    Ok(ReadResult {
        file: file.display().to_string(),
        kind: kind.to_string(),
        trust: "document",
        summary: doc.summary,
        sections,
    })
}

/// A GUID derived from the kind, so a part's `ds:itemID` is stable across writes and machines.
fn item_guid(kind: &str) -> String {
    let h = blake3::hash(format!("{AO_NS}/{kind}").as_bytes())
        .to_hex()
        .to_uppercase();
    format!(
        "{{{}-{}-{}-{}-{}}}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn apply_patch(doc: &mut KnowledgeDoc, patch: &DocPatch) -> Result<()> {
    if let Some(s) = &patch.summary {
        check_xml_chars("summary", s)?;
        doc.summary = s.clone();
    }
    for p in &patch.sections {
        if p.id.is_empty() {
            bail!("a section needs a non-empty id");
        }
        for (k, v) in [
            ("id", Some(&p.id)),
            ("format", p.format.as_ref()),
            ("text", p.text.as_ref()),
        ] {
            check_xml_chars(&format!("section '{}' {k}", p.id), v.map_or("", |v| v))?;
        }
        let pos = doc.sections.iter().position(|s| s.id == p.id);
        match (pos, p.delete) {
            (Some(i), true) => {
                doc.sections.remove(i);
            }
            (None, true) => {}
            (Some(i), false) => {
                let s = &mut doc.sections[i];
                if let Some(f) = &p.format {
                    s.format = f.clone();
                }
                if let Some(t) = &p.text {
                    s.text = t.clone();
                }
            }
            (None, false) => doc.sections.push(Section {
                id: p.id.clone(),
                format: p.format.clone().unwrap_or_else(|| "text/markdown".into()),
                text: p.text.clone().unwrap_or_default(),
            }),
        }
    }
    Ok(())
}

/// Insert `xml` just before the root's closing tag, turning a self-closing root into an open one.
fn insert_before_close(doc: &str, root: &str, xml: &str) -> Result<String> {
    let close = format!("</{root}>");
    if let Some(i) = doc.rfind(&close) {
        return Ok(format!("{}{xml}{}", &doc[..i], &doc[i..]));
    }
    let trimmed = doc.trim_end();
    if let Some(head) = trimmed.strip_suffix("/>") {
        return Ok(format!("{head}>{xml}{close}"));
    }
    bail!("cannot find the closing tag of <{root}>")
}

/// The `[Content_Types].xml` edits a new props part needs: its Override, and an `xml` Default if
/// the package has none. Already-present entries are left alone.
fn add_content_types(ct: &str, props_part: &str) -> Result<String> {
    let mut r = Reader::from_str(ct);
    let (mut has_default, mut has_override) = (false, false);
    let part_name = format!("/{props_part}");
    loop {
        match r.read_event()? {
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                b"Default" => {
                    has_default |=
                        attr(&e, "Extension")?.is_some_and(|x| x.eq_ignore_ascii_case("xml"))
                }
                b"Override" => {
                    has_override |=
                        attr(&e, "PartName")?.is_some_and(|p| p.eq_ignore_ascii_case(&part_name))
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    let mut add = String::new();
    if !has_default {
        add.push_str("<Default Extension=\"xml\" ContentType=\"application/xml\"/>");
    }
    if !has_override {
        add.push_str(&format!(
            "<Override PartName=\"{}\" ContentType=\"{CT_PROPS}\"/>",
            esc_attr(&part_name)
        ));
    }
    if add.is_empty() {
        Ok(ct.to_string())
    } else {
        insert_before_close(ct, "Types", &add)
    }
}

/// Upsert, verify or delete source records. A record written with `status: "verified"` gets its
/// `claim_hash` computed from the current file; the caller cannot supply one.
fn apply_source_patches(
    zip: &mut Archive,
    recs: &mut Vec<SourceRecord>,
    patches: &[SourcePatch],
    ctx: &ExtCtx,
) -> Result<()> {
    for p in patches {
        if p.id.is_empty() {
            bail!("a source needs a non-empty id");
        }
        let mut fields = p.fields.clone();
        if let Some(e) = fields.external_claim.as_mut() {
            e.normalize()
                .with_context(|| format!("source '{}'", p.id))?;
        }
        let f = &fields;
        for (k, v) in [
            ("id", Some(&p.id)),
            ("source_kind", f.source_kind.as_ref()),
            ("checked_at", f.checked_at.as_ref()),
            ("resource_uri", f.resource_uri.as_ref()),
            ("location", f.location.as_ref()),
            ("claim_part", f.claim_part.as_ref()),
            ("claim_object_id", f.claim_object_id.as_ref()),
            ("source_hash", f.source_hash.as_ref()),
            ("source_hash_algorithm", f.source_hash_algorithm.as_ref()),
        ] {
            check_xml_chars(&format!("source '{}' {k}", p.id), v.map_or("", |v| v))?;
        }
        if let Some(s) = p.status.as_deref().filter(|s| !SOURCE_STATUSES.contains(s)) {
            bail!(
                "unknown source status '{s}' (expected {})",
                SOURCE_STATUSES.join("|")
            );
        }
        let pos = recs.iter().position(|r| r.id == p.id);
        if p.delete {
            if let Some(i) = pos {
                recs.remove(i);
            }
            continue;
        }
        let i = pos.unwrap_or_else(|| {
            recs.push(SourceRecord {
                id: p.id.clone(),
                status: "unverified".into(),
                fields: SourceFields::default(),
                claim_hash: None,
            });
            recs.len() - 1
        });
        let r = &mut recs[i];
        r.fields.merge(&fields);
        if let Some(s) = &p.status {
            r.status = s.clone();
        }
        if r.status == "verified" && p.status.is_some() {
            if r.fields.claim_part.is_none() && r.fields.external_claim.is_none() {
                bail!(
                    "source '{}' cannot be verified without a claim_part or an external_claim",
                    r.id
                );
            }
            if let Some(part) = r.fields.claim_part.clone() {
                let c = claim_hash(zip, &part, r.fields.claim_object_id.as_deref())?;
                if c.part_missing {
                    bail!(
                        "source '{}' cannot be verified: claim part {part} does not exist",
                        r.id
                    );
                }
                if c.object_missing {
                    bail!(
                        "source '{}' cannot be verified: no object with id {} on {part}",
                        r.id,
                        r.fields.claim_object_id.as_deref().unwrap_or("")
                    );
                }
                r.claim_hash = c.hash;
            }
            if let Some(e) = r.fields.external_claim.as_mut() {
                let v = read_value(ctx, e).map_err(|why| {
                    anyhow!(
                        "source '{}' cannot be verified: external claim {}: {why}",
                        r.id,
                        e.locator()
                    )
                })?;
                e.claim_hash = Some(blake3_tag(v.as_bytes()));
            }
        }
    }
    Ok(())
}

pub fn write(file: &Path, kind: &str, patch: &DocPatch, replace: bool) -> Result<WriteResult> {
    write_with(file, kind, patch, replace, &EvalOptions::default())
}

/// `write` with explicit limits for reading external claims (the MCP server passes its roots).
pub fn write_with(
    file: &Path,
    kind: &str,
    patch: &DocPatch,
    replace: bool,
    opts: &EvalOptions,
) -> Result<WriteResult> {
    let is_sources = kind == "sources";
    if !is_sources && !KNOWLEDGE_KINDS.contains(&kind) {
        bail!("unknown knowledge kind '{kind}' (expected agents|design|intent|sources)");
    }
    if is_sources && (patch.summary.is_some() || !patch.sections.is_empty()) {
        bail!("the sources part takes \"sources\", not \"summary\" or \"sections\"");
    }
    if !is_sources && !patch.sources.is_empty() {
        bail!("\"sources\" can only be written with kind sources");
    }
    let mut zip = open(file)?;
    let (main, found) = scan(&mut zip)?;
    // One writer per file: once the canonical v0.2 context is there, the TS servers own it, and a
    // second, v0.1 channel beside it would split what the file says about itself.
    if let Some(f) = found.iter().find(|f| f.kind == "doc") {
        bail!(
            "{} carries Agentic OOXML v0.2 context (<ao:doc> in {}); write it with doc_context_patch, not knowledge-write (one writer per file)",
            file.display(),
            f.part
        );
    }
    let item = format!("customXml/ao-{kind}.xml");
    // With --replace, our own linked `ao-<kind>.xml` is also recovered when its root is unreadable
    // (listed as kind `unknown`); otherwise it would block the create path below for good.
    let existing = found
        .into_iter()
        .find(|f| f.kind == kind || (replace && f.part == item));
    if let Some(Found {
        parsed: Err(e),
        part,
        ..
    }) = &existing
    {
        if !replace {
            bail!("the existing '{kind}' part {part} is malformed ({e:#}); pass --replace to overwrite it");
        }
    }
    let base = existing
        .as_ref()
        .filter(|_| !replace)
        .and_then(|f| f.parsed.as_ref().ok());
    let base_version = match base {
        Some(Parsed::Knowledge(d)) => Some(d.schema_version.as_str()),
        Some(Parsed::Sources { schema_version, .. }) => Some(schema_version.as_str()),
        _ => None,
    };
    if let Some(v) = base_version.filter(|v| is_newer_schema(v)) {
        bail!("the existing '{kind}' part has schemaVersion {v}, newer than {SCHEMA_VERSION}; merging would drop what this version does not know (pass --replace to overwrite it)");
    }

    let (xml, sections, sources) = if is_sources {
        let mut recs = match base {
            Some(Parsed::Sources { records, .. }) => records.clone(),
            _ => vec![],
        };
        apply_source_patches(
            &mut zip,
            &mut recs,
            &patch.sources,
            &ExtCtx::for_file(file, opts),
        )?;
        (
            serialize_sources(&recs),
            vec![],
            Some(recs.into_iter().map(|r| r.id).collect()),
        )
    } else {
        let mut doc = match base {
            Some(Parsed::Knowledge(d)) => d.clone(),
            _ => KnowledgeDoc {
                kind: kind.into(),
                schema_version: SCHEMA_VERSION.into(),
                summary: String::new(),
                sections: vec![],
            },
        };
        apply_patch(&mut doc, patch)?;
        doc.schema_version = SCHEMA_VERSION.into();
        (
            serialize_knowledge(&doc),
            doc.sections.into_iter().map(|s| s.id).collect(),
            None,
        )
    };
    if xml.len() as u64 > MAX_PART_BYTES {
        bail!(
            "the '{kind}' part would be {} bytes (cap {MAX_PART_BYTES})",
            xml.len()
        );
    }

    // An existing part is rewritten in place, under whatever name it was found.
    if let Some(f) = existing {
        drop(zip);
        rewrite_package(file, &[(f.part.clone(), xml.into_bytes())])?;
        return Ok(WriteResult {
            part: f.part,
            created: false,
            sections,
            sources,
        });
    }

    let props = format!("customXml/ao-{kind}Props.xml");
    if zip.file_names().any(|n| n == item || n == props) {
        bail!("{item} or {props} already exists but is not linked as a '{kind}' knowledge part");
    }
    let props_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n<ds:datastoreItem ds:itemID=\"{}\" xmlns:ds=\"{DS_NS}\"><ds:schemaRefs><ds:schemaRef ds:uri=\"{AO_NS}\"/></ds:schemaRefs></ds:datastoreItem>\n",
        item_guid(kind)
    );
    let item_rels = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"{RELS_NS}\"><Relationship Id=\"rId1\" Type=\"{REL_CUSTOM_XML_PROPS}\" Target=\"ao-{kind}Props.xml\"/></Relationships>\n"
    );

    let main_rels_path = rels_path(&main);
    let main_rels = read_entry(&mut zip, &main_rels_path)?;
    let main_rel_list = match &main_rels {
        Some(x) => parse_rels(x)?,
        None => vec![],
    };
    // LibreOffice's .xlsx save drops the part but keeps the relationship to it. Recreating the
    // part under that same name relinks it, so no second relationship is added beside the old one.
    let already_linked = main_rel_list.iter().any(|r| {
        !r.external
            && r.rel_type == REL_CUSTOM_XML
            && resolve_target(&main, &r.target).ok().as_deref() == Some(item.as_str())
    });
    let new_main_rels = if already_linked {
        main_rels.clone().unwrap_or_default()
    } else {
        let mut rid = format!("rIdAo{kind}");
        while main_rel_list.iter().any(|r| r.id == rid) {
            rid.push('x');
        }
        let target = format!("{}{item}", "../".repeat(main.matches('/').count()));
        let rel =
            format!("<Relationship Id=\"{rid}\" Type=\"{REL_CUSTOM_XML}\" Target=\"{target}\"/>");
        match &main_rels {
            Some(x) => insert_before_close(x, "Relationships", &rel)?,
            None => format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"{RELS_NS}\">{rel}</Relationships>\n"),
        }
    };
    let ct = read_entry(&mut zip, "[Content_Types].xml")?
        .ok_or_else(|| anyhow!("package has no [Content_Types].xml"))?;
    let new_ct = add_content_types(&ct, &props)?;
    drop(zip);

    rewrite_package(
        file,
        &[
            ("[Content_Types].xml".into(), new_ct.into_bytes()),
            (main_rels_path, new_main_rels.into_bytes()),
            (item.clone(), xml.into_bytes()),
            (rels_path(&item), item_rels.into_bytes()),
            (props, props_xml.into_bytes()),
        ],
    )?;
    Ok(WriteResult {
        part: item,
        created: true,
        sections,
        sources,
    })
}

// ---------------------------------------------------------------------------------------------
// Sources: claim hashes, effective status, find, reconcile
// ---------------------------------------------------------------------------------------------

fn blake3_tag(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

/// The byte range of the smallest PresentationML shape element whose own `p:nvXxPr/p:cNvPr` has
/// `id` equal to `id` (a group's children carry their own ids, so a group matches only on its own).
fn find_object(xml: &[u8], id: &str) -> Result<Option<Range<usize>>> {
    let mut r = NsReader::from_reader(xml);
    // Open elements: (is a shape, start offset, its own cNvPr matched).
    let mut stack: Vec<(bool, usize, bool)> = vec![];
    let mut best: Option<Range<usize>> = None;
    loop {
        let start = r.buffer_position() as usize;
        let (ns, ev) = r.read_resolved_event()?;
        let pml = matches!(ns, ResolveResult::Bound(Namespace(n)) if n == PML_NS);
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let n = stack.len();
                // shape > nvXxPr > cNvPr: the shape sits two levels above.
                if pml
                    && e.local_name().as_ref() == b"cNvPr"
                    && n >= 2
                    && stack[n - 2].0
                    && attr(e, "id")?.as_deref() == Some(id)
                {
                    stack[n - 2].2 = true;
                }
                if matches!(ev, Event::Start(_)) {
                    stack.push((
                        pml && PML_SHAPES.contains(&e.local_name().as_ref()),
                        start,
                        false,
                    ));
                }
            }
            Event::End(_) => {
                let (shape, s, hit) = stack.pop().ok_or_else(|| anyhow!("unbalanced end tag"))?;
                let end = r.buffer_position() as usize;
                if shape && hit && best.as_ref().is_none_or(|b| end - s < b.len()) {
                    best = Some(s..end);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(best)
}

/// The current claim hash. With an object id on a slide part (`ppt/slides/…`), the raw bytes of
/// that shape; otherwise, and when the object is not found, the whole part.
pub(crate) fn claim_hash(
    zip: &mut Archive,
    part: &str,
    object_local_id: Option<&str>,
) -> Result<ClaimHash> {
    let name = part.trim_start_matches('/');
    let mut out = ClaimHash {
        hash: None,
        granularity: "part",
        object_missing: false,
        part_missing: false,
    };
    let Some(bytes) = read_bytes(zip, name, MAX_CLAIM_PART_BYTES)? else {
        out.part_missing = true;
        return Ok(out);
    };
    if let Some(id) = object_local_id.filter(|_| name.starts_with("ppt/slides/")) {
        match find_object(&bytes, id)? {
            Some(range) => {
                out.granularity = "object";
                out.hash = Some(blake3_tag(&bytes[range]));
                return Ok(out);
            }
            None => out.object_missing = true,
        }
    }
    out.hash = Some(blake3_tag(&bytes));
    Ok(out)
}

/// The result of checking one record against the current files.
struct Evaluation {
    status: String,
    /// Why a `verified` record is stale: external reasons first, then in-package ones.
    reasons: Vec<&'static str>,
    granularity: Option<&'static str>,
    external: Option<ExternalView>,
}

/// The effective status of a record, never trusting a stored `verified` alone: it holds only while
/// the claim exists and still hashes to the stored baseline, and, for an external claim, while the
/// value in the other file still hashes to its baseline.
fn evaluate(zip: &mut Archive, rec: &SourceRecord, ctx: &ExtCtx) -> Evaluation {
    // A claim part that cannot be read (oversized, malformed slide XML) counts as missing: its
    // verification can no longer be shown.
    let ch = rec.fields.claim_part.as_deref().map(|p| {
        claim_hash(zip, p, rec.fields.claim_object_id.as_deref()).unwrap_or(ClaimHash {
            hash: None,
            granularity: "part",
            object_missing: false,
            part_missing: true,
        })
    });
    let granularity = ch.as_ref().map(|c| c.granularity);
    let verified = rec.status == "verified";
    let external = rec
        .fields
        .external_claim
        .as_ref()
        .map(|e| evaluate_external(ctx, e, verified));
    if !verified {
        return Evaluation {
            status: rec.status.clone(),
            reasons: vec![],
            granularity,
            external,
        };
    }
    let mut reasons: Vec<&'static str> = external.iter().filter_map(|e| e.reason).collect();
    let internal = match &ch {
        None if external.is_some() => None,
        None => Some("claim_missing"),
        Some(c) if c.part_missing || c.object_missing => Some("claim_missing"),
        _ if rec.claim_hash.is_none() => Some("no_baseline"),
        Some(c) if c.hash != rec.claim_hash => Some("claim_changed"),
        _ => None,
    };
    reasons.extend(internal);
    Evaluation {
        status: if reasons.is_empty() {
            rec.status.clone()
        } else {
            "stale".into()
        },
        reasons,
        granularity,
        external,
    }
}

/// The linked sources part, its schemaVersion and its records; `None` when the package has none.
fn load_sources(zip: &mut Archive) -> Result<Option<(String, String, Vec<SourceRecord>)>> {
    let Some(f) = scan(zip)?.1.into_iter().find(|f| f.kind == "sources") else {
        return Ok(None);
    };
    match f
        .parsed
        .with_context(|| format!("the sources part {} is malformed", f.part))?
    {
        Parsed::Sources {
            schema_version,
            records,
        } => Ok(Some((f.part, schema_version, records))),
        _ => bail!("the sources part {} is not an ao:sources part", f.part),
    }
}

/// Every source record with its effective status, optionally filtered by claim part and object.
pub fn sources(file: &Path, part: Option<&str>, object: Option<&str>) -> Result<SourcesResult> {
    sources_with(file, part, object, &EvalOptions::default())
}

/// `sources` with explicit limits for reading external claims (the MCP server passes its roots).
pub fn sources_with(
    file: &Path,
    part: Option<&str>,
    object: Option<&str>,
    opts: &EvalOptions,
) -> Result<SourcesResult> {
    let ectx = ExtCtx::for_file(file, opts);
    let mut zip = open(file)?;
    let (main, found) = scan(&mut zip)?;
    let ctx = load_context(&mut zip, &main, &found)?;
    drop(found);
    let context_sources = ctx.source_views(part, object);
    let recs = load_sources(&mut zip)?
        .map(|(_, _, r)| r)
        .unwrap_or_default();
    let want_part = part.map(|p| p.trim_start_matches('/'));
    let sources = recs
        .into_iter()
        .filter(|r| {
            want_part.is_none_or(|p| {
                r.fields
                    .claim_part
                    .as_deref()
                    .map(|c| c.trim_start_matches('/'))
                    == Some(p)
            })
        })
        .filter(|r| object.is_none_or(|o| r.fields.claim_object_id.as_deref() == Some(o)))
        .map(|record| {
            let ev = evaluate(&mut zip, &record, &ectx);
            SourceView {
                record,
                effective_status: ev.status,
                stale_reason: ev.reasons.first().copied(),
                stale_reasons: if ev.reasons.len() > 1 {
                    ev.reasons
                } else {
                    vec![]
                },
                granularity: ev.granularity,
                external_check: ev.external,
            }
        })
        .collect();
    Ok(SourcesResult {
        file: file.display().to_string(),
        trust: "document",
        sources,
        context_sources,
    })
}

/// Persist `status="stale"` on every stored-verified record whose effective status is stale. The
/// old `claim_hash` is kept as the baseline. Nothing to change (or a dry run) writes nothing.
pub fn reconcile(file: &Path, dry_run: bool) -> Result<ReconcileResult> {
    reconcile_with(file, dry_run, &EvalOptions::default())
}

/// `reconcile` with explicit limits for reading external claims (the MCP server passes its roots).
pub fn reconcile_with(file: &Path, dry_run: bool, opts: &EvalOptions) -> Result<ReconcileResult> {
    let ctx = ExtCtx::for_file(file, opts);
    let mut zip = open(file)?;
    let mut result = ReconcileResult {
        file: file.display().to_string(),
        invalidated: vec![],
        unchanged: 0,
        written: false,
    };
    let Some((part, version, mut recs)) = load_sources(&mut zip)? else {
        return Ok(result);
    };
    if is_newer_schema(&version) {
        bail!("the sources part has schemaVersion {version}, newer than {SCHEMA_VERSION}; reconcile would drop what this version does not know");
    }
    for r in recs.iter_mut() {
        let ev = evaluate(&mut zip, r, &ctx);
        match ev.reasons.first() {
            Some(&reason) if r.status == "verified" && ev.status == "stale" => {
                r.status = "stale".into();
                result.invalidated.push(Invalidated {
                    id: r.id.clone(),
                    reason,
                });
            }
            _ => result.unchanged += 1,
        }
    }
    if !dry_run && !result.invalidated.is_empty() {
        drop(zip);
        rewrite_package(file, &[(part, serialize_sources(&recs).into_bytes())])?;
        result.written = true;
    }
    Ok(result)
}

// ---------------------------------------------------------------------------------------------
// Agentic OOXML v0.2 (read-only): the file context `<ao:doc>` and its units
// ---------------------------------------------------------------------------------------------
//
// The model mirrors `mcp/shared/aoContext.ts` (the source of truth and the only writer). Parts are
// found by relationship and root element, never by name: `<ao:doc>` and (docx/xlsx) `<ao:units>`
// are related from the main part, a .pptx keeps one `<ao:unit>` per slide related from that
// slide's own `.rels`. Every value is document data; strings and lists are bounded as the TS
// writer bounds them, so a hand-made part cannot blow up a result.

type Attrs = std::collections::BTreeMap<String, String>;

const CTX_MAX_UNITS: usize = 400;
const CTX_MAX_LIST: usize = 20;
const CTX_MAX_SOURCES: usize = 60;
const CTX_MAX_ATTR_CHARS: usize = 600;
const CTX_MAX_NODES: usize = 20_000;
const CTX_MAX_DEPTH: usize = 16;

/// A parsed element: just what the v0.2 grammar needs (namespace flags, local name, attributes,
/// direct text, children).
#[derive(Debug, Default)]
struct XNode {
    ctx: bool,
    v01: bool,
    local: String,
    attrs: Attrs,
    text: String,
    children: Vec<XNode>,
}

fn clip_chars(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        t.chars().take(max).collect()
    }
}

fn parse_tree(xml: &str) -> Result<XNode> {
    let mut r = NsReader::from_str(xml);
    let mut stack: Vec<XNode> = vec![];
    let mut root: Option<XNode> = None;
    let mut nodes = 0usize;
    fn attach(stack: &mut [XNode], root: &mut Option<XNode>, n: XNode) {
        match stack.last_mut() {
            Some(p) => p.children.push(n),
            None => {
                if root.is_none() {
                    *root = Some(n);
                }
            }
        }
    }
    loop {
        let (ns, ev) = r.read_resolved_event()?;
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                nodes += 1;
                if nodes > CTX_MAX_NODES || stack.len() >= CTX_MAX_DEPTH {
                    bail!("context part is too large or nests too deep");
                }
                let mut attrs = Attrs::new();
                for a in e.attributes() {
                    let a = a?;
                    let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
                    if key == "xmlns" || key.starts_with("xmlns:") {
                        continue;
                    }
                    attrs.insert(key, clip_chars(&a.unescape_value()?, CTX_MAX_ATTR_CHARS));
                }
                let node = XNode {
                    ctx: is_ctx(&ns),
                    v01: is_ao(&ns),
                    local: String::from_utf8_lossy(e.local_name().as_ref()).into_owned(),
                    attrs,
                    ..Default::default()
                };
                if matches!(ev, Event::Empty(_)) {
                    attach(&mut stack, &mut root, node);
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                let n = stack.pop().ok_or_else(|| anyhow!("unbalanced end tag"))?;
                attach(&mut stack, &mut root, n);
            }
            Event::Text(ref t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t.unescape()?);
                }
            }
            Event::CData(ref c) => {
                if let Some(top) = stack.last_mut() {
                    top.text
                        .push_str(std::str::from_utf8(c).context("CDATA is not UTF-8")?);
                }
            }
            Event::DocType(_) => bail!(DOCTYPE_ERR),
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        bail!("unclosed element at end of part");
    }
    root.ok_or_else(|| anyhow!("no root element"))
}

/// `<ao:doc>`: what the file is for, design decisions, preferences, file-scope sources, the index.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CtxDoc {
    pub kind: String,
    pub rev: u64,
    pub schema_version: String,
    pub design: Attrs,
    pub intent: Attrs,
    pub prefs: Vec<String>,
    pub sources: Vec<Attrs>,
    #[serde(skip)]
    index: Vec<Attrs>,
}

/// `<ao:spec>`: the requirement contract a file carries (document-origin DATA, never instructions).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CtxSpec {
    pub rev: u64,
    pub mode: String,
    pub format: String,
    pub request: String,
    pub requirements: Vec<String>,
    pub assumptions: Vec<CtxAssumption>,
    pub open_questions: Vec<String>,
    pub criteria: Vec<CtxCriterion>,
    /// What the author's numbers rest on: the source of record and the sourced figures it was given (document data, never instructions).
    pub grounding: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CtxAssumption {
    pub text: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CtxCriterion {
    pub text: String,
    pub status: String,
    pub check: String,
}

impl CtxSpec {
    /// One line for a listing or a card: mode, and how much is left to settle or prove.
    pub fn summary(&self) -> String {
        let unconfirmed = self
            .assumptions
            .iter()
            .filter(|a| a.status != "confirmed")
            .count();
        let unmet = self.criteria.iter().filter(|c| c.status != "met").count();
        format!(
            "spec ({}): {} requirement(s), {} unconfirmed assumption(s), {} open question(s), {}/{} criteria met",
            self.mode,
            self.requirements.len(),
            unconfirmed,
            self.open_questions.len(),
            self.criteria.len() - unmet,
            self.criteria.len()
        )
    }
}

/// One `<ao:unit>` (a slide, a heading section, a sheet).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CtxUnit {
    pub id: String,
    #[serde(rename = "ref")]
    pub ref_: String,
    /// The slide / section / sheet number, when the package (or the doc index) says it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u64>,
    pub unit_kind: String,
    pub rev: u64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(skip_serializing_if = "Attrs::is_empty")]
    pub design: Attrs,
    pub constraints: usize,
    pub decisions: usize,
    #[serde(skip)]
    source_list: Vec<Attrs>,
    pub sources: usize,
    pub part: String,
}

/// The v0.1 `<ao:deck>` the old TS pptx writer made: readable, summarized, never written here.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LegacyDeck {
    pub schema_version: String,
    pub revision: u64,
    pub design: Attrs,
    pub intent: Attrs,
    pub slides: usize,
    pub slides_with_role: usize,
    pub sources: usize,
}

impl LegacyDeck {
    fn summary(&self) -> String {
        let mut bits = vec![];
        for k in ["purpose", "audience", "about"] {
            if let Some(v) = self.intent.get(k) {
                bits.push(format!("{k}: {v}"));
            }
        }
        bits.push(format!(
            "{} slide rows ({} with a role), {} sources",
            self.slides, self.slides_with_role, self.sources
        ));
        format!(
            "v0.1 deck context (upgraded to v0.2 by the pptx server on its next write); {}",
            bits.join("; ")
        )
    }
}

fn ctx_children<'a>(n: &'a XNode, local: &'a str) -> impl Iterator<Item = &'a XNode> + 'a {
    n.children.iter().filter(move |c| c.ctx && c.local == local)
}

fn num(a: &Attrs, k: &str) -> u64 {
    a.get(k).and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

fn ctx_doc(n: &XNode) -> CtxDoc {
    let mut d = CtxDoc {
        kind: n.attrs.get("kind").cloned().unwrap_or_default(),
        rev: num(&n.attrs, "rev"),
        schema_version: n.attrs.get("schemaVersion").cloned().unwrap_or_default(),
        ..Default::default()
    };
    for c in n.children.iter().filter(|c| c.ctx) {
        match c.local.as_str() {
            "design" => d.design.extend(c.attrs.clone()),
            "intent" => d.intent.extend(c.attrs.clone()),
            "pref" if d.prefs.len() < CTX_MAX_LIST => d.prefs.push(clip_chars(&c.text, 300)),
            "source" if d.sources.len() < CTX_MAX_SOURCES => d.sources.push(c.attrs.clone()),
            "index" => d.index.extend(
                ctx_children(c, "u")
                    .take(CTX_MAX_UNITS)
                    .map(|u| u.attrs.clone()),
            ),
            _ => {}
        }
    }
    d
}

fn ctx_spec(n: &XNode) -> CtxSpec {
    let mut s = CtxSpec {
        rev: num(&n.attrs, "rev"),
        mode: n
            .attrs
            .get("mode")
            .cloned()
            .unwrap_or_else(|| "none".into()),
        format: n.attrs.get("format").cloned().unwrap_or_default(),
        ..Default::default()
    };
    for c in n.children.iter().filter(|c| c.ctx) {
        match c.local.as_str() {
            "request" => s.request = clip_chars(&c.text, 400),
            "requirement" if s.requirements.len() < CTX_MAX_LIST => {
                s.requirements.push(clip_chars(&c.text, 300))
            }
            "grounding" if s.grounding.len() < CTX_MAX_LIST => {
                s.grounding.push(clip_chars(&c.text, 300))
            }
            "assumption" if s.assumptions.len() < CTX_MAX_LIST => {
                s.assumptions.push(CtxAssumption {
                    text: clip_chars(&c.text, 300),
                    status: if c.attrs.get("status").map(String::as_str) == Some("confirmed") {
                        "confirmed".into()
                    } else {
                        "unconfirmed".into()
                    },
                })
            }
            "criterion" if s.criteria.len() < CTX_MAX_LIST => s.criteria.push(CtxCriterion {
                text: clip_chars(&c.text, 300),
                status: if c.attrs.get("status").map(String::as_str) == Some("met") {
                    "met".into()
                } else {
                    "unmet".into()
                },
                check: c
                    .attrs
                    .get("check")
                    .cloned()
                    .unwrap_or_else(|| "manual".into()),
            }),
            "question"
                if c.attrs.get("status").map(String::as_str) == Some("open")
                    && s.open_questions.len() < CTX_MAX_LIST =>
            {
                let q = c
                    .children
                    .iter()
                    .find(|x| x.local == "text")
                    .map(|x| clip_chars(&x.text, 200))
                    .unwrap_or_default();
                s.open_questions.push(q);
            }
            _ => {}
        }
    }
    s
}

fn ctx_unit(n: &XNode, part: &str) -> CtxUnit {
    let a = &n.attrs;
    let status = a
        .get("status")
        .filter(|s| matches!(s.as_str(), "fresh" | "stale" | "orphan"))
        .cloned()
        .unwrap_or_else(|| "fresh".into());
    let mut u = CtxUnit {
        id: a.get("id").cloned().unwrap_or_default(),
        ref_: a.get("ref").cloned().unwrap_or_default(),
        unit_kind: a.get("kind").cloned().unwrap_or_default(),
        rev: num(a, "rev"),
        status,
        part: part.to_string(),
        ..Default::default()
    };
    for c in n.children.iter().filter(|c| c.ctx) {
        match c.local.as_str() {
            "role" => u.role = Some(clip_chars(&c.text, 80)),
            "intent" => u.intent = Some(clip_chars(&c.text, 600)),
            "constraint" => u.constraints = (u.constraints + 1).min(CTX_MAX_LIST),
            "decision" => u.decisions = (u.decisions + 1).min(CTX_MAX_LIST),
            "design" => u.design.extend(c.attrs.clone()),
            "source" if u.source_list.len() < CTX_MAX_SOURCES => {
                u.source_list.push(c.attrs.clone())
            }
            _ => {}
        }
    }
    u.sources = u.source_list.len();
    u
}

fn legacy_deck(n: &XNode) -> LegacyDeck {
    let mut d = LegacyDeck {
        schema_version: n.attrs.get("schemaVersion").cloned().unwrap_or_default(),
        revision: num(&n.attrs, "revision"),
        ..Default::default()
    };
    for c in n.children.iter().filter(|c| c.v01) {
        match c.local.as_str() {
            "design" => d.design.extend(c.attrs.clone()),
            "intent" => d.intent.extend(c.attrs.clone()),
            "slide" => {
                d.slides += 1;
                d.slides_with_role +=
                    usize::from(c.attrs.get("role").is_some_and(|r| !r.is_empty()));
            }
            "source" => d.sources += 1,
            _ => {}
        }
    }
    d
}

/// Parse a part whose root `sniff_root` reported as `doc`, `units`, `unit` or `legacy-deck`. The
/// part name is filled in by the caller (`CtxState`), which knows where each unit lives.
fn parse_context_part(kind: &str, xml: &str) -> Result<Parsed> {
    let root = parse_tree(xml)?;
    Ok(match kind {
        "doc" => Parsed::Doc(ctx_doc(&root)),
        "spec" => Parsed::Spec(ctx_spec(&root)),
        "unit" => Parsed::Units(vec![ctx_unit(&root, "")]),
        "units" => {
            let kind = root.attrs.get("kind").cloned().unwrap_or_default();
            Parsed::Units(
                ctx_children(&root, "unit")
                    .take(CTX_MAX_UNITS)
                    .map(|n| {
                        let mut u = ctx_unit(n, "");
                        if u.unit_kind.is_empty() {
                            u.unit_kind = kind.clone();
                        }
                        u
                    })
                    .collect(),
            )
        }
        _ => Parsed::LegacyDeck(legacy_deck(&root)),
    })
}

/// Everything v0.2 a package carries, with each unit's number resolved where the package says it.
struct CtxState {
    format: String,
    doc: Option<(String, CtxDoc)>,
    units: Vec<CtxUnit>,
    unit_parts: Vec<String>,
    legacy: Option<(String, LegacyDeck)>,
    spec: Option<(String, CtxSpec)>,
    errors: Vec<(String, String)>,
    /// Whether every unit ref could be checked against the native units (pptx, xlsx): a unit whose
    /// ref is not among them is then an orphan whatever its stored status says.
    natives_known: bool,
}

/// The native units in document order: (ref, part). pptx = `p:sldId` ids, xlsx = sheet names.
fn native_units(
    zip: &mut Archive,
    main: &str,
    format: &str,
) -> Result<Option<Vec<(String, String)>>> {
    let (elem, key): (&[u8], &str) = match format {
        "pptx" => (b"sldId", "id"),
        "xlsx" => (b"sheet", "name"),
        _ => return Ok(None),
    };
    let Some(xml) = read_entry(zip, main)? else {
        return Ok(None);
    };
    let rels = match read_entry(zip, &rels_path(main))? {
        Some(x) => parse_rels(&x)?,
        None => vec![],
    };
    let mut out = vec![];
    let mut r = Reader::from_str(&xml);
    loop {
        match r.read_event()? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == elem => {
                let mut native = None;
                let mut rid = None;
                for a in e.attributes() {
                    let a = a?;
                    let k = a.key.as_ref();
                    if k == key.as_bytes() {
                        native = Some(a.unescape_value()?.into_owned());
                    } else if a.key.local_name().as_ref() == b"id" && a.key.prefix().is_some() {
                        rid = Some(a.unescape_value()?.into_owned());
                    }
                }
                let part = rid
                    .and_then(|rid| rels.iter().find(|x| x.id == rid && !x.external))
                    .and_then(|x| resolve_target(main, &x.target).ok());
                if let (Some(n), Some(p)) = (native, part) {
                    out.push((n, p));
                    if out.len() >= CTX_MAX_UNITS {
                        break;
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(Some(out))
}

/// Collect the v0.2 state: the main part's `doc` / `units` / legacy deck from `found`, plus, for a
/// .pptx, the unit part related from each slide.
fn load_context(zip: &mut Archive, main: &str, found: &[Found]) -> Result<CtxState> {
    let format = format_of(main).to_string();
    let mut st = CtxState {
        format: format.clone(),
        doc: None,
        units: vec![],
        unit_parts: vec![],
        legacy: None,
        spec: None,
        errors: vec![],
        natives_known: false,
    };
    for f in found {
        match (&f.parsed, f.kind.as_str()) {
            (Ok(Parsed::Doc(d)), _) if st.doc.is_none() => {
                st.doc = Some((f.part.clone(), d.clone()))
            }
            (Ok(Parsed::Units(us)), _) => {
                st.unit_parts.push(f.part.clone());
                st.units.extend(us.iter().cloned().map(|mut u| {
                    u.part = f.part.clone();
                    u
                }));
            }
            (Ok(Parsed::LegacyDeck(d)), _) if st.legacy.is_none() => {
                st.legacy = Some((f.part.clone(), d.clone()))
            }
            (Ok(Parsed::Spec(sp)), _) if st.spec.is_none() => {
                st.spec = Some((f.part.clone(), sp.clone()))
            }
            (Err(e), "doc" | "units" | "unit" | "spec") => {
                st.errors.push((f.part.clone(), format!("{e:#}")))
            }
            _ => {}
        }
    }
    let natives = native_units(zip, main, &format).unwrap_or(None);
    if format == "pptx" {
        for (slide_ref, slide_part) in natives.iter().flatten() {
            let Ok(Some(rels)) = read_entry(zip, &rels_path(slide_part)) else {
                continue;
            };
            let Ok(rels) = parse_rels(&rels) else {
                continue;
            };
            for rel in rels
                .iter()
                .filter(|r| !r.external && r.rel_type == REL_CUSTOM_XML)
            {
                let Ok(part) = resolve_target(slide_part, &rel.target) else {
                    continue;
                };
                let xml = match read_entry(zip, &part) {
                    Ok(Some(x)) => x,
                    Ok(None) => continue,
                    Err(e) => {
                        st.errors.push((part, format!("{e:#}")));
                        continue;
                    }
                };
                if !matches!(sniff_root(&xml), Ok(Some(k)) if k == "unit") {
                    continue;
                }
                match parse_tree(&xml) {
                    Ok(root) => {
                        let mut u = ctx_unit(&root, &part);
                        // The slide that holds the part is the unit (the TS adapter does the same
                        // after a reorder or a duplicate).
                        u.ref_ = slide_ref.clone();
                        if u.unit_kind.is_empty() {
                            u.unit_kind = "slide".into();
                        }
                        st.unit_parts.push(part);
                        st.units.push(u);
                    }
                    Err(e) => st.errors.push((part, format!("{e:#}"))),
                }
                break; // one unit part per slide
            }
            if st.units.len() >= CTX_MAX_UNITS {
                break;
            }
        }
    }
    // Numbers: the package's own order first, then the doc index's `n` (docx sections).
    let index: Vec<Attrs> = st
        .doc
        .as_ref()
        .map(|(_, d)| d.index.clone())
        .unwrap_or_default();
    if let Some(n) = &natives {
        st.natives_known = format == "pptx" || format == "xlsx";
        for u in st.units.iter_mut() {
            u.n = n
                .iter()
                .position(|(r, _)| *r == u.ref_)
                .map(|i| i as u64 + 1);
        }
    }
    for u in st.units.iter_mut().filter(|u| u.n.is_none()) {
        u.n = index
            .iter()
            .find(|x| x.get("ref") == Some(&u.ref_))
            .and_then(|x| x.get("n"))
            .and_then(|v| v.parse().ok());
    }
    if st.natives_known {
        for u in st.units.iter_mut().filter(|u| u.n.is_none()) {
            u.status = "orphan".into();
        }
    }
    Ok(st)
}

impl CtxState {
    fn stale(&self) -> usize {
        self.units.iter().filter(|u| u.status == "stale").count()
    }
    fn orphan(&self) -> usize {
        self.units.iter().filter(|u| u.status == "orphan").count()
    }

    fn discovered(&self) -> Vec<DiscoveredPart> {
        let mut out = vec![];
        if let Some((part, d)) = &self.doc {
            let mut p = empty_discovered("doc".into(), part.clone());
            p.schema_version = d.schema_version.clone();
            p.rev = Some(d.rev);
            p.source_count = Some(d.sources.len());
            p.summary = ["purpose", "audience", "about"]
                .iter()
                .filter_map(|k| d.intent.get(*k).map(|v| format!("{k}: {v}")))
                .collect::<Vec<_>>()
                .join("; ");
            out.push(p);
        }
        if !self.units.is_empty() {
            let part = match self.unit_parts.as_slice() {
                [one] => one.clone(),
                _ => format!("({} unit parts)", self.unit_parts.len()),
            };
            let mut p = empty_discovered("units".into(), part);
            p.schema_version = CTX_NS
                .rsplit(':')
                .next()
                .unwrap_or("")
                .trim_start_matches('v')
                .to_string();
            p.unit_count = Some(self.units.len());
            p.stale_count = Some(self.stale());
            p.orphan_count = Some(self.orphan());
            p.source_count = Some(self.units.iter().map(|u| u.sources).sum());
            if self.unit_parts.len() > 1 {
                p.unit_parts = Some(self.unit_parts.clone());
            }
            out.push(p);
        }
        for (part, e) in &self.errors {
            let mut p = empty_discovered("unknown".into(), part.clone());
            p.error = Some(e.clone());
            out.push(p);
        }
        out
    }

    fn source_views(&self, part: Option<&str>, object: Option<&str>) -> Vec<CtxSourceView> {
        if object.is_some() {
            return vec![]; // v0.2 sources name a unit, not a shape id
        }
        let want = part.map(|p| p.trim_start_matches('/'));
        let mut out = vec![];
        if let Some((dpart, d)) = &self.doc {
            if want.is_none_or(|w| w == dpart) {
                out.extend(
                    d.sources
                        .iter()
                        .map(|a| CtxSourceView::new("file", None, a)),
                );
            }
        }
        for u in &self.units {
            if want.is_none_or(|w| w == u.part) {
                out.extend(
                    u.source_list
                        .iter()
                        .map(|a| CtxSourceView::new("unit", Some(u), a)),
                );
            }
        }
        out
    }
}

/// A v0.2 source as `knowledge-sources` reports it. v0.2 keeps no claim hash, so the status is the
/// stored one (absent = `unverified`, as the TS writer defaults it).
#[derive(Debug, Serialize)]
pub struct CtxSourceView {
    pub scope: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_n: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_kind: Option<String>,
    pub effective_status: String,
    pub record: Attrs,
}

impl CtxSourceView {
    fn new(scope: &'static str, u: Option<&CtxUnit>, a: &Attrs) -> Self {
        CtxSourceView {
            scope,
            unit: u.map(|u| u.ref_.clone()),
            unit_n: u.and_then(|u| u.n),
            unit_kind: u.map(|u| u.unit_kind.clone()),
            effective_status: a
                .get("status")
                .filter(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| "unverified".into()),
            record: a.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CtxDocView {
    pub part: String,
    #[serde(flatten)]
    pub doc: CtxDoc,
}

/// `knowledge-read --kind doc|units|legacy-deck`.
#[derive(Debug, Serialize)]
pub struct ContextReadResult {
    pub file: String,
    pub format: String,
    pub kind: String,
    pub trust: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<CtxDocView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<CtxUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_deck: Option<LegacyDeck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<CtxSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Read the v0.2 context: `doc` = the file context plus every unit, `units` = the units alone,
/// `legacy-deck` = the v0.1 deck summary. `unit` narrows the units to one ref or number.
pub fn read_context(file: &Path, kind: &str, unit: Option<&str>) -> Result<ContextReadResult> {
    if !CONTEXT_KINDS.contains(&kind) {
        bail!(
            "unknown context kind '{kind}' (expected {})",
            CONTEXT_KINDS.join("|")
        );
    }
    let mut zip = open(file)?;
    let (main, found) = scan(&mut zip)?;
    let st = load_context(&mut zip, &main, &found)?;
    let (stale, orphan) = (st.stale(), st.orphan());
    let mut out = ContextReadResult {
        file: file.display().to_string(),
        format: st.format.clone(),
        kind: kind.to_string(),
        trust: "document",
        doc: None,
        units: vec![],
        legacy_deck: None,
        spec: None,
        stale: None,
        orphan: None,
        errors: st.errors.iter().map(|(p, e)| format!("{p}: {e}")).collect(),
    };
    if kind == "legacy-deck" {
        out.legacy_deck = Some(
            st.legacy
                .ok_or_else(|| anyhow!("no v0.1 <ao:deck> part in {}", file.display()))?
                .1,
        );
        return Ok(out);
    }
    if kind == "spec" {
        out.spec = Some(
            st.spec
                .ok_or_else(|| anyhow!("no <ao:spec> part in {}", file.display()))?
                .1,
        );
        return Ok(out);
    }
    if kind == "doc" {
        let (part, doc) = st
            .doc
            .ok_or_else(|| anyhow!("no v0.2 <ao:doc> part in {}", file.display()))?;
        out.doc = Some(CtxDocView { part, doc });
    } else if st.units.is_empty() {
        bail!("no v0.2 unit context in {}", file.display());
    }
    out.units = match unit {
        None => st.units,
        Some(want) => {
            let hit: Vec<CtxUnit> = st
                .units
                .iter()
                .filter(|u| u.ref_ == want || u.n.map(|n| n.to_string()).as_deref() == Some(want))
                .cloned()
                .collect();
            if hit.is_empty() {
                let refs: Vec<String> = st.units.iter().map(|u| u.ref_.clone()).collect();
                bail!(
                    "no unit '{want}' in the v0.2 context; available refs: [{}]",
                    refs.join(", ")
                );
            }
            hit
        }
    };
    out.stale = Some(stale);
    out.orphan = Some(orphan);
    Ok(out)
}

/// The compact v0.2 overview an OKF card renders; `None` when the package carries no v0.2 context
/// (or cannot be read).
pub struct ContextOverview {
    pub doc: Option<CtxDoc>,
    pub unit_label: &'static str,
    pub units: usize,
    pub with_role: usize,
    pub stale: usize,
    pub orphan: usize,
    pub sources: usize,
    pub unverified: usize,
    pub spec: Option<CtxSpec>,
}

pub fn context_overview(file: &Path) -> Option<ContextOverview> {
    let mut zip = open(file).ok()?;
    let (main, found) = scan(&mut zip).ok()?;
    let st = load_context(&mut zip, &main, &found).ok()?;
    if st.doc.is_none() && st.units.is_empty() && st.spec.is_none() {
        return None;
    }
    let all: Vec<&Attrs> = st
        .doc
        .iter()
        .flat_map(|(_, d)| d.sources.iter())
        .chain(st.units.iter().flat_map(|u| u.source_list.iter()))
        .collect();
    Some(ContextOverview {
        unit_label: match st.format.as_str() {
            "pptx" => "slides",
            "xlsx" => "sheets",
            _ => "sections",
        },
        units: st.units.len(),
        with_role: st
            .units
            .iter()
            .filter(|u| u.role.as_deref().is_some_and(|r| !r.is_empty()))
            .count(),
        stale: st.stale(),
        orphan: st.orphan(),
        sources: all.len(),
        unverified: all
            .iter()
            .filter(|a| {
                a.get("status")
                    .is_none_or(|s| s.is_empty() || s == "unverified")
            })
            .count(),
        doc: st.doc.map(|(_, d)| d),
        spec: st.spec.map(|(_, sp)| sp),
    })
}

/// v0.2 fixtures, built in memory the way the TS servers lay a package out, then written to disk.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::{CTX_NS, RELS_NS, REL_CUSTOM_XML};
    use std::io::Write;
    use std::path::Path;

    const R_OFFICE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
    const R_SLIDE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";

    pub(crate) fn zip_in_memory(entries: &[(&str, String)]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (n, b) in entries {
            w.start_file(*n, opts).unwrap();
            w.write_all(b.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn rels(items: &[(&str, &str, &str)]) -> String {
        let body: String = items
            .iter()
            .map(|(id, t, target)| {
                format!("<Relationship Id=\"{id}\" Type=\"{t}\" Target=\"{target}\"/>")
            })
            .collect();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{RELS_NS}\">{body}</Relationships>")
    }

    fn ct() -> String {
        "<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/></Types>".into()
    }

    pub(crate) const PPTX_DOC: &str = r#"<ao:doc xmlns:ao="NS" schemaVersion="0.2" kind="pptx" rev="4">
  <ao:design register="keynote" visualRegister="visual" visualPlan="photo cover; one designed slide in 3"/>
  <ao:intent purpose="persuade" audience="seed investors" about="coffee subscription"/>
  <ao:pref>No stock photos of handshakes</ao:pref>
  <ao:source id="f1" kind="data" uri="https://example.org/market" status="verified"/>
  <ao:index>
    <ao:u ref="256" n="1" role="cover"/>
    <ao:u ref="257" n="2" role="evidence"/>
  </ao:index>
</ao:doc>"#;

    /// A .pptx with a v0.2 `<ao:doc>` related from the presentation and one `<ao:unit>` per slide
    /// related from that slide's `.rels` (slide 1 = id 256, slide 2 = id 257).
    pub(crate) fn v02_pptx(file: &Path) {
        let unit1 = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ao:unit xmlns:ao="{CTX_NS}" schemaVersion="0.2" id="u-aaaa1111" ref="256" kind="slide" rev="2" status="fresh" contentHash="abc">
  <ao:role>cover</ao:role>
  <ao:intent>Open on the harbour photo; one line of promise.</ao:intent>
  <ao:constraint>Title under 8 words</ao:constraint>
  <ao:constraint>No logo wall</ao:constraint>
  <ao:design recipe="photo-bleed"/>
  <ao:source id="p1" kind="photo" uri="https://example.org/harbour.jpg" licence="CC0"/>
  <ao:decision at="2026-10-18">Chose the dawn photo over the cafe interior</ao:decision>
</ao:unit>"#
        );
        let unit2 = format!(
            r#"<ao:unit xmlns:ao="{CTX_NS}" id="u-bbbb2222" ref="257" kind="slide" rev="1" status="stale"><ao:role>evidence</ao:role><ao:constraint>Cite the survey</ao:constraint></ao:unit>"#
        );
        let pres = r#"<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId3"/></p:sldIdLst></p:presentation>"#;
        let bytes = zip_in_memory(&[
            ("[Content_Types].xml", ct()),
            (
                "_rels/.rels",
                rels(&[("rId1", R_OFFICE, "ppt/presentation.xml")]),
            ),
            ("ppt/presentation.xml", pres.into()),
            (
                "ppt/_rels/presentation.xml.rels",
                rels(&[
                    ("rId2", R_SLIDE, "slides/slide1.xml"),
                    ("rId3", R_SLIDE, "slides/slide2.xml"),
                    ("rId9", REL_CUSTOM_XML, "../customXml/item1.xml"),
                ]),
            ),
            (
                "ppt/slides/slide1.xml",
                "<p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"/>"
                    .into(),
            ),
            (
                "ppt/slides/slide2.xml",
                "<p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"/>"
                    .into(),
            ),
            (
                "ppt/slides/_rels/slide1.xml.rels",
                rels(&[("rId7", REL_CUSTOM_XML, "../../customXml/item2.xml")]),
            ),
            (
                "ppt/slides/_rels/slide2.xml.rels",
                rels(&[("rId7", REL_CUSTOM_XML, "../../customXml/item3.xml")]),
            ),
            ("customXml/item1.xml", PPTX_DOC.replace("NS", CTX_NS)),
            ("customXml/item2.xml", unit1),
            ("customXml/item3.xml", unit2),
        ]);
        std::fs::write(file, bytes).unwrap();
    }

    /// A .docx with a v0.2 `<ao:doc>` and one `<ao:units kind="section">` part, both related from
    /// the document part; the third unit is stored as an orphan and is not in the index.
    pub(crate) fn v02_docx(file: &Path) {
        let doc = format!(
            r#"<ao:doc xmlns:ao="{CTX_NS}" schemaVersion="0.2" kind="docx" rev="7"><ao:intent purpose="inform" audience="the board"/><ao:design styleSet="memo"/><ao:index><ao:u ref="1A2B3C4D" n="1"/><ao:u ref="t:9f8e7d6c" n="2"/></ao:index></ao:doc>"#
        );
        let units = format!(
            r#"<?xml version="1.0"?><ao:units xmlns:ao="{CTX_NS}" schemaVersion="0.2" kind="section">
  <ao:unit id="u-1" ref="1A2B3C4D" rev="1" status="fresh"><ao:role>summary</ao:role><ao:source id="s1" kind="report" status="verified"/></ao:unit>
  <ao:unit id="u-2" ref="t:9f8e7d6c" rev="3" status="stale"><ao:role>risks</ao:role><ao:constraint>Name an owner per risk</ao:constraint><ao:source id="s2" kind="memo"/></ao:unit>
  <ao:unit id="u-3" ref="DEADBEEF" rev="1" status="orphan"><ao:intent>Was the appendix</ao:intent></ao:unit>
</ao:units>"#
        );
        let bytes = zip_in_memory(&[
            ("[Content_Types].xml", ct()),
            ("_rels/.rels", rels(&[("rId1", R_OFFICE, "word/document.xml")])),
            ("word/document.xml", "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body/></w:document>".into()),
            ("word/_rels/document.xml.rels", rels(&[("rId5", REL_CUSTOM_XML, "../customXml/item1.xml"), ("rId6", REL_CUSTOM_XML, "../customXml/item2.xml")])),
            ("customXml/item1.xml", doc),
            ("customXml/item2.xml", units),
        ]);
        std::fs::write(file, bytes).unwrap();
    }

    /// A .docx carrying a v0.2 `<ao:doc>` AND a requirement contract (`<ao:spec>`) as two custom XML parts.
    pub(crate) fn docx_with_spec(file: &Path) {
        let doc = format!(
            r#"<ao:doc xmlns:ao="{CTX_NS}" schemaVersion="0.2" kind="docx" rev="2"><ao:intent purpose="inform"/></ao:doc>"#
        );
        let spec = format!(
            r#"<?xml version="1.0"?><ao:spec xmlns:ao="{CTX_NS}" schemaVersion="0.2" rev="4" mode="deep" format="docx"><ao:request>Write the memo</ao:request>
<ao:requirement id="r1" source="file">Use the Q3 actuals</ao:requirement><ao:assumption id="a1" status="unconfirmed">British spelling</ao:assumption><ao:assumption id="a2" status="confirmed">One page</ao:assumption>
<ao:question id="q1" status="open"><ao:text>Which audience?</ao:text></ao:question><ao:question id="q2" status="answered" answer="x"><ao:text>Done?</ao:text></ao:question>
<ao:criterion id="c1" check="tool" status="met">No overlaps</ao:criterion><ao:criterion id="c2" check="visual" status="unmet">Cover has a photo</ao:criterion>
<ao:grounding>Q3 revenue (actual): 18.6m (Q3 Actuals.xlsx)</ao:grounding></ao:spec>"#
        );
        let bytes = zip_in_memory(&[
            ("[Content_Types].xml", ct()),
            ("_rels/.rels", rels(&[("rId1", R_OFFICE, "word/document.xml")])),
            ("word/document.xml", "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body/></w:document>".into()),
            ("word/_rels/document.xml.rels", rels(&[("rId5", REL_CUSTOM_XML, "../customXml/item1.xml"), ("rId6", REL_CUSTOM_XML, "../customXml/item2.xml")])),
            ("customXml/item1.xml", doc),
            ("customXml/item2.xml", spec),
        ]);
        std::fs::write(file, bytes).unwrap();
    }

    /// A .pptx carrying only the old TS writer's v0.1 `<ao:deck>`.
    pub(crate) fn legacy_deck_pptx(file: &Path) {
        let deck = r#"<ao:deck xmlns:ao="urn:agentic-office:ooxml:v0.1" schemaVersion="0.1" revision="3"><ao:design register="keynote"/><ao:intent purpose="persuade" audience="seed investors"/><ao:slide ref="256" role="role-photo-bleed"/><ao:slide ref="257"/><ao:source id="photo-1" kind="photo" slide="256" uri="https://x"/></ao:deck>"#;
        let bytes = zip_in_memory(&[
            ("[Content_Types].xml", ct()),
            ("_rels/.rels", rels(&[("rId1", R_OFFICE, "ppt/presentation.xml")])),
            ("ppt/presentation.xml", "<p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"/>".into()),
            ("ppt/_rels/presentation.xml.rels", rels(&[("rId9", REL_CUSTOM_XML, "../customXml/item1.xml")])),
            ("customXml/item1.xml", deck.into()),
        ]);
        std::fs::write(file, bytes).unwrap();
    }
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    const CT_HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>";

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("ak-ooxml-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_requirement_contract_is_listed_read_and_summarised_never_reported_as_an_unknown_root() {
        let dir = tmp_dir();
        let file = dir.join("spec.docx");
        fixtures::docx_with_spec(&file);
        let listed = list(&file).unwrap();
        let spec = listed
            .parts
            .iter()
            .find(|p| p.kind == "spec")
            .expect("the spec part is listed");
        assert!(spec.error.is_none(), "{:?}", spec.error);
        assert_eq!(spec.rev, Some(4));
        assert_eq!(spec.summary, "spec (deep): 1 requirement(s), 1 unconfirmed assumption(s), 1 open question(s), 1/2 criteria met");
        let read = read_context(&file, "spec", None).unwrap();
        let sp = read.spec.unwrap();
        assert_eq!(sp.request, "Write the memo");
        assert_eq!(
            sp.open_questions,
            vec!["Which audience?".to_string()],
            "an answered question is not open"
        );
        assert_eq!(sp.criteria.iter().filter(|c| c.status == "met").count(), 1);
        assert_eq!(
            sp.grounding,
            vec!["Q3 revenue (actual): 18.6m (Q3 Actuals.xlsx)".to_string()],
            "the figures the author was given are read back as data"
        );
        // the v0.2 doc part next to it is untouched by the new part
        assert_eq!(
            read_context(&file, "doc", None)
                .unwrap()
                .doc
                .unwrap()
                .doc
                .rev,
            2
        );
    }

    #[test]
    fn a_file_without_a_spec_says_so_and_a_spec_alone_still_gives_an_overview() {
        let dir = tmp_dir();
        let plain = dir.join("plain.docx");
        fixtures::v02_docx(&plain);
        assert!(read_context(&plain, "spec", None)
            .unwrap_err()
            .to_string()
            .contains("no <ao:spec> part"));
        let file = dir.join("spec.docx");
        fixtures::docx_with_spec(&file);
        assert!(context_overview(&file).unwrap().spec.is_some());
    }

    fn zip_entries(file: &Path, entries: &[(&str, &str)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(file).unwrap());
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (n, b) in entries {
            w.start_file(*n, opts).unwrap();
            w.write_all(b.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }

    /// A minimal package: `[Content_Types].xml`, `_rels/.rels`, the main part and its `.rels`.
    fn make_pkg(format: &str) -> PathBuf {
        let (main, ct) = match format {
            "docx" => ("word/document.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"),
            "pptx" => ("ppt/presentation.xml", "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"),
            _ => ("xl/workbook.xml", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"),
        };
        let file = tmp_dir().join(format!("t.{format}"));
        let ct_xml =
            format!("{CT_HEAD}<Override PartName=\"/{main}\" ContentType=\"{ct}\"/></Types>");
        let root_rels = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{RELS_NS}\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"{main}\"/></Relationships>");
        let main_rels = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{RELS_NS}\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>");
        zip_entries(
            &file,
            &[
                ("[Content_Types].xml", &ct_xml),
                ("_rels/.rels", &root_rels),
                (main, "<root>native content</root>"),
                (&rels_path(main), &main_rels),
            ],
        );
        file
    }

    fn contents(file: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut z = zip::ZipArchive::new(std::fs::File::open(file).unwrap()).unwrap();
        (0..z.len())
            .map(|i| {
                let mut e = z.by_index(i).unwrap();
                let mut b = vec![];
                e.read_to_end(&mut b).unwrap();
                (e.name().to_string(), b)
            })
            .collect()
    }

    fn text(file: &Path, name: &str) -> String {
        String::from_utf8(contents(file)[name].clone()).unwrap()
    }

    fn patch(json: &str) -> DocPatch {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn ooxml_knowledge_write_list_read_all_formats() {
        for (fmt, main) in [
            ("docx", "word/document.xml"),
            ("pptx", "ppt/presentation.xml"),
            ("xlsx", "xl/workbook.xml"),
        ] {
            let file = make_pkg(fmt);
            let before = contents(&file);
            let r = write(
                &file,
                "design",
                &patch(
                    r#"{"summary":"Brand rules","sections":[{"id":"type","text":"Use Inter."}]}"#,
                ),
                false,
            )
            .unwrap();
            assert!(r.created);
            assert_eq!(r.part, "customXml/ao-design.xml");

            let l = list(&file).unwrap();
            assert_eq!(l.format, fmt);
            assert_eq!(l.trust, "document");
            assert_eq!(l.parts.len(), 1);
            assert_eq!(l.parts[0].kind, "design");
            assert_eq!(l.parts[0].summary, "Brand rules");
            assert_eq!(l.parts[0].sections, vec!["type"]);
            assert!(l.parts[0].error.is_none());

            let rd = read(&file, "design", None).unwrap();
            assert_eq!(rd.trust, "document");
            assert_eq!(rd.sections[0].text, "Use Inter.");
            assert_eq!(rd.sections[0].format, "text/markdown");
            let one = read(&file, "design", Some("type")).unwrap();
            assert_eq!(one.sections.len(), 1);
            let err = read(&file, "design", Some("nope")).unwrap_err().to_string();
            assert!(err.contains("type"), "{err}");

            // Every original entry is still there; untouched ones are byte-identical.
            let after = contents(&file);
            let main_rels = rels_path(main);
            for (name, bytes) in &before {
                assert!(after.contains_key(name), "{name} lost");
                if name != "[Content_Types].xml" && *name != main_rels {
                    assert_eq!(&after[name], bytes, "{name} changed");
                }
            }
            // Packaging: the new parts and exactly the expected additions.
            assert!(after.contains_key("customXml/ao-designProps.xml"));
            assert!(after.contains_key("customXml/_rels/ao-design.xml.rels"));
            let props = text(&file, "customXml/ao-designProps.xml");
            assert!(props.contains(&item_guid("design")) && props.contains(AO_NS));
            let ct = text(&file, "[Content_Types].xml");
            let rels = text(&file, &main_rels);
            assert_eq!(ct.matches("/customXml/ao-designProps.xml").count(), 1);
            assert_eq!(ct.matches("Extension=\"xml\"").count(), 1);
            assert_eq!(rels.matches(REL_CUSTOM_XML).count(), 1);
            assert!(rels.contains("Target=\"../customXml/ao-design.xml\""));

            // A second write (and a second kind) does not duplicate what is there.
            write(&file, "design", &patch(r#"{"summary":"v2"}"#), false).unwrap();
            assert_eq!(text(&file, "[Content_Types].xml"), ct);
            assert_eq!(text(&file, &main_rels), rels);
            write(&file, "agents", &patch(r#"{"summary":"a"}"#), false).unwrap();
            let ct2 = text(&file, "[Content_Types].xml");
            assert_eq!(ct2.matches("/customXml/ao-designProps.xml").count(), 1);
            assert_eq!(ct2.matches("/customXml/ao-agentsProps.xml").count(), 1);
            assert_eq!(ct2.matches("Extension=\"xml\"").count(), 1);
            assert_eq!(list(&file).unwrap().parts.len(), 2);
        }
    }

    #[test]
    fn ooxml_knowledge_merge_upsert_and_delete() {
        let file = make_pkg("docx");
        write(&file, "intent", &patch(r#"{"summary":"s1","sections":[{"id":"a","text":"A"},{"id":"b","text":"B"},{"id":"c","text":"C"}]}"#), false).unwrap();
        let r = write(&file, "intent", &patch(r#"{"sections":[{"id":"b","text":"B2"},{"id":"d","text":"D"},{"id":"a","delete":true}]}"#), false).unwrap();
        assert!(!r.created);
        assert_eq!(r.sections, vec!["b", "c", "d"]);
        let rd = read(&file, "intent", None).unwrap();
        assert_eq!(rd.summary, "s1");
        assert_eq!(rd.sections[0].text, "B2");
        // Replace starts from an empty document.
        let r = write(
            &file,
            "intent",
            &patch(r#"{"sections":[{"id":"z"}]}"#),
            true,
        )
        .unwrap();
        assert_eq!(r.sections, vec!["z"]);
        assert_eq!(read(&file, "intent", None).unwrap().summary, "");
        assert!(write(
            &file,
            "intent",
            &patch(r#"{"sources":[{"id":"s"}]}"#),
            false
        )
        .is_err());
    }

    #[test]
    fn ooxml_knowledge_renamed_part_is_discovered_and_updated_in_place() {
        let file = make_pkg("docx");
        write(
            &file,
            "agents",
            &patch(r#"{"summary":"orig","sections":[{"id":"x","text":"X"}]}"#),
            false,
        )
        .unwrap();
        // Another editor renamed the item to customXml/item7.xml and updated the relationship.
        let mut entries: Vec<(String, String)> = contents(&file)
            .into_iter()
            .map(|(n, b)| {
                let n = match n.as_str() {
                    "customXml/ao-agents.xml" => "customXml/item7.xml".to_string(),
                    "customXml/_rels/ao-agents.xml.rels" => {
                        "customXml/_rels/item7.xml.rels".to_string()
                    }
                    _ => n,
                };
                (
                    n,
                    String::from_utf8(b)
                        .unwrap()
                        .replace("../customXml/ao-agents.xml", "../customXml/item7.xml"),
                )
            })
            .collect();
        entries.sort();
        let refs: Vec<(&str, &str)> = entries
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        zip_entries(&file, &refs);

        let l = list(&file).unwrap();
        assert_eq!(l.parts.len(), 1);
        assert_eq!(l.parts[0].part, "customXml/item7.xml");
        assert_eq!(l.parts[0].kind, "agents");

        let r = write(&file, "agents", &patch(r#"{"summary":"updated"}"#), false).unwrap();
        assert_eq!(r.part, "customXml/item7.xml");
        assert!(!r.created);
        let after = contents(&file);
        assert!(!after.contains_key("customXml/ao-agents.xml"));
        assert_eq!(read(&file, "agents", None).unwrap().summary, "updated");
        assert_eq!(
            text(&file, "word/_rels/document.xml.rels")
                .matches(REL_CUSTOM_XML)
                .count(),
            1
        );
    }

    #[test]
    fn ooxml_knowledge_cdata_terminator_round_trip() {
        let doc = KnowledgeDoc {
            kind: "agents".into(),
            schema_version: "0.1".into(),
            summary: "a < b && c > d ]]> \"q\" 'x'".into(),
            sections: vec![Section {
                id: "s&\"1".into(),
                format: "text/markdown".into(),
                text: "x]]>y <tag> & ]]]]> end\n  ".into(),
            }],
        };
        let xml = serialize_knowledge(&doc);
        assert_eq!(parse_knowledge(&xml).unwrap(), doc);
    }

    #[test]
    fn ooxml_knowledge_doctype_in_section_text_round_trips() {
        let file = make_pkg("docx");
        let body = "Use <!DOCTYPE html> in templates ]]> <!DOCTYPE x>";
        write(&file, "agents", &patch(&serde_json::json!({"summary": "<!DOCTYPE s>", "sections": [{"id": "t", "text": body}]}).to_string()), false).unwrap();
        let l = list(&file).unwrap();
        assert_eq!(l.parts[0].kind, "agents");
        assert!(l.parts[0].error.is_none());
        let rd = read(&file, "agents", None).unwrap();
        assert_eq!(rd.summary, "<!DOCTYPE s>");
        assert_eq!(rd.sections[0].text, body);
        write(&file, "agents", &patch(r#"{"summary":"again"}"#), false).unwrap();
        assert_eq!(read(&file, "agents", None).unwrap().sections[0].text, body);
    }

    #[test]
    fn ooxml_knowledge_replace_recovers_part_with_corrupt_root() {
        let file = make_pkg("pptx");
        write(&file, "design", &patch(r#"{"summary":"ok"}"#), false).unwrap();
        let mut entries: Vec<(String, String)> = contents(&file)
            .into_iter()
            .map(|(n, b)| (n, String::from_utf8(b).unwrap()))
            .collect();
        for (n, b) in entries.iter_mut() {
            if n == "customXml/ao-design.xml" {
                *b = "not xml at all <<<".into();
            }
        }
        let refs: Vec<(&str, &str)> = entries
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        zip_entries(&file, &refs);
        let l = list(&file).unwrap();
        assert_eq!(l.parts[0].kind, "unknown");
        assert!(l.parts[0].error.is_some());
        assert!(write(&file, "design", &patch(r#"{"summary":"new"}"#), false).is_err());

        let r = write(
            &file,
            "design",
            &patch(r#"{"summary":"new","sections":[{"id":"a","text":"A"}]}"#),
            true,
        )
        .unwrap();
        assert_eq!(r.part, "customXml/ao-design.xml");
        assert!(!r.created);
        let l = list(&file).unwrap();
        assert_eq!(l.parts.len(), 1);
        assert_eq!(l.parts[0].kind, "design");
        assert!(l.parts[0].error.is_none());
        assert_eq!(
            read(&file, "design", Some("a")).unwrap().sections[0].text,
            "A"
        );
        assert_eq!(
            text(&file, "[Content_Types].xml")
                .matches("/customXml/ao-designProps.xml")
                .count(),
            1
        );
        assert_eq!(
            text(&file, "ppt/_rels/presentation.xml.rels")
                .matches(REL_CUSTOM_XML)
                .count(),
            1
        );
    }

    #[test]
    fn ooxml_knowledge_doctype_rejected() {
        let xml = format!("<?xml version=\"1.0\"?><!DOCTYPE x [<!ENTITY e \"boom\">]><ao:knowledge xmlns:ao=\"{AO_NS}\" kind=\"agents\" schemaVersion=\"0.1\"><ao:summary>&e;</ao:summary></ao:knowledge>");
        assert!(parse_knowledge(&xml)
            .unwrap_err()
            .to_string()
            .contains("DOCTYPE"));
    }

    #[test]
    fn ooxml_knowledge_unsafe_entry_name_rejected() {
        let file = make_pkg("docx");
        let mut entries: Vec<(String, String)> = contents(&file)
            .into_iter()
            .map(|(n, b)| (n, String::from_utf8(b).unwrap()))
            .collect();
        entries.push(("../evil".into(), "x".into()));
        let refs: Vec<(&str, &str)> = entries
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        zip_entries(&file, &refs);
        assert!(discover(&file).unwrap_err().to_string().contains("unsafe"));
        assert!(write(&file, "agents", &DocPatch::default(), false).is_err());
    }

    #[test]
    fn ooxml_knowledge_malformed_part_reported_others_listed() {
        let file = make_pkg("docx");
        write(&file, "design", &patch(r#"{"summary":"ok"}"#), false).unwrap();
        write(
            &file,
            "intent",
            &patch(r#"{"summary":"will break"}"#),
            false,
        )
        .unwrap();
        let mut entries: Vec<(String, String)> = contents(&file)
            .into_iter()
            .map(|(n, b)| (n, String::from_utf8(b).unwrap()))
            .collect();
        for (n, b) in entries.iter_mut() {
            if n == "customXml/ao-intent.xml" {
                *b = format!("<ao:knowledge xmlns:ao=\"{AO_NS}\" kind=\"intent\"><ao:summary>x</ao:knowledge>");
            }
        }
        // A sources part linked under a foreign name is discovered too.
        entries.push((
            "customXml/item9.xml".into(),
            format!("<ao:sources xmlns:ao=\"{AO_NS}\" schemaVersion=\"0.1\"><ao:source id=\"s1\"/><ao:source id=\"s2\"/></ao:sources>"),
        ));
        for (n, b) in entries.iter_mut() {
            if n == "word/_rels/document.xml.rels" {
                *b = b.replace("</Relationships>", &format!("<Relationship Id=\"rId99\" Type=\"{REL_CUSTOM_XML}\" Target=\"../customXml/item9.xml\"/></Relationships>"));
            }
        }
        let refs: Vec<(&str, &str)> = entries
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        zip_entries(&file, &refs);

        let parts = discover(&file).unwrap();
        assert_eq!(parts.len(), 3);
        let by = |k: &str| parts.iter().find(|p| p.kind == k).unwrap();
        assert!(by("design").error.is_none());
        assert_eq!(by("design").summary, "ok");
        assert!(by("intent").error.is_some());
        assert_eq!(by("sources").source_count, Some(2));
        assert_eq!(by("sources").part, "customXml/item9.xml");
        assert!(read(&file, "intent", None).is_err());
        assert_eq!(read(&file, "design", None).unwrap().summary, "ok");
    }

    /// Replace (or add) one entry, the way another editor would rewrite the package.
    fn set_entry(file: &Path, name: &str, body: &str) {
        let mut entries: Vec<(String, String)> = contents(file)
            .into_iter()
            .map(|(n, b)| (n, String::from_utf8(b).unwrap()))
            .collect();
        match entries.iter_mut().find(|(n, _)| n == name) {
            Some(e) => e.1 = body.to_string(),
            None => entries.push((name.to_string(), body.to_string())),
        }
        let refs: Vec<(&str, &str)> = entries
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        zip_entries(file, &refs);
    }

    fn slide(t2: &str, t3: &str) -> String {
        let shape = |id: u32, t: &str| {
            format!("<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Shape {id}\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>{t}</a:t></a:r></a:p></p:txBody></p:sp>")
        };
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{}\n{}</p:spTree></p:cSld></p:sld>",
            shape(2, t2),
            shape(3, t3)
        )
    }

    fn pptx_with_slide() -> PathBuf {
        let file = make_pkg("pptx");
        set_entry(
            &file,
            "ppt/slides/slide1.xml",
            &slide("Title", "Revenue 12.4m"),
        );
        file
    }

    fn one(file: &Path, id: &str) -> SourceView {
        sources(file, None, None)
            .unwrap()
            .sources
            .into_iter()
            .find(|v| v.record.id == id)
            .unwrap()
    }

    #[test]
    fn ooxml_knowledge_sources_object_claim_staleness_and_reconcile() {
        let file = pptx_with_slide();
        let r = write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"rev","status":"verified","source_kind":"workbook","resource_uri":"relative:Model.xlsx","location":"Forecast!D12","claim_part":"ppt/slides/slide1.xml","claim_object_id":"3","source_hash":"abc","source_hash_algorithm":"blake3"}]}"#),
            false,
        )
        .unwrap();
        assert!(r.created);
        assert_eq!(r.part, "customXml/ao-sources.xml");
        assert_eq!(r.sources, Some(vec!["rev".to_string()]));
        assert!(text(&file, "[Content_Types].xml").contains("/customXml/ao-sourcesProps.xml"));
        assert!(text(&file, "ppt/_rels/presentation.xml.rels")
            .contains("Target=\"../customXml/ao-sources.xml\""));

        // The stored baseline is the hash of exactly shape 3's bytes.
        let xml = slide("Title", "Revenue 12.4m");
        let s3 = &xml[xml.find("<p:sp><p:nvSpPr><p:cNvPr id=\"3\"").unwrap()..];
        let s3 = &s3[..s3.find("</p:sp>").unwrap() + "</p:sp>".len()];
        let v = one(&file, "rev");
        assert_eq!(
            v.record.claim_hash.as_deref(),
            Some(blake3_tag(s3.as_bytes()).as_str())
        );
        assert_eq!(
            v.record.fields.claim_part.as_deref(),
            Some("/ppt/slides/slide1.xml")
        );
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason, v.granularity),
            ("verified", None, Some("object"))
        );
        assert_eq!(v.record.fields.location.as_deref(), Some("Forecast!D12"));
        assert_eq!(list(&file).unwrap().parts[0].source_count, Some(1));

        // Editing another shape on the same slide does not invalidate the claim.
        set_entry(
            &file,
            "ppt/slides/slide1.xml",
            &slide("New title", "Revenue 12.4m"),
        );
        assert_eq!(one(&file, "rev").effective_status, "verified");
        assert!(reconcile(&file, false).unwrap().invalidated.is_empty());

        // Editing the claim shape does.
        set_entry(
            &file,
            "ppt/slides/slide1.xml",
            &slide("New title", "Revenue 99m"),
        );
        let v = one(&file, "rev");
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason),
            ("stale", Some("claim_changed"))
        );
        assert_eq!(v.record.status, "verified");

        // A dry run reports but writes nothing.
        let before = std::fs::read(&file).unwrap();
        let d = reconcile(&file, true).unwrap();
        assert_eq!(d.invalidated.len(), 1);
        assert!(!d.written);
        assert_eq!(std::fs::read(&file).unwrap(), before);

        // Reconcile persists stale, keeps the old baseline, and is idempotent.
        let rc = reconcile(&file, false).unwrap();
        assert_eq!(
            (
                rc.invalidated[0].id.as_str(),
                rc.invalidated[0].reason,
                rc.unchanged,
                rc.written
            ),
            ("rev", "claim_changed", 0, true)
        );
        let v = one(&file, "rev");
        assert_eq!(
            (v.record.status.as_str(), v.effective_status.as_str()),
            ("stale", "stale")
        );
        assert_eq!(
            v.record.claim_hash.as_deref(),
            Some(blake3_tag(s3.as_bytes()).as_str())
        );
        let after = std::fs::read(&file).unwrap();
        let again = reconcile(&file, false).unwrap();
        assert!(again.invalidated.is_empty() && !again.written && again.unchanged == 1);
        assert_eq!(std::fs::read(&file).unwrap(), after);

        // Re-verifying takes a new baseline.
        write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"rev","status":"verified"}]}"#),
            false,
        )
        .unwrap();
        assert_eq!(one(&file, "rev").effective_status, "verified");

        // The object disappearing makes the claim missing; verifying a missing object is refused.
        set_entry(
            &file,
            "ppt/slides/slide1.xml",
            &slide("t", "x").replace("id=\"3\"", "id=\"7\""),
        );
        assert_eq!(one(&file, "rev").stale_reason, Some("claim_missing"));
        assert!(write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"rev","status":"verified"}]}"#),
            false
        )
        .is_err());
    }

    #[test]
    fn ooxml_knowledge_sources_part_claim_on_docx_ignores_supplied_hash() {
        let file = make_pkg("docx");
        write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"a","status":"verified","claim_part":"/word/document.xml","claim_object_id":"12","claim_hash":"blake3:bogus","claimHash":"blake3:bogus"}]}"#),
            false,
        )
        .unwrap();
        let v = one(&file, "a");
        assert_eq!(
            v.record.claim_hash,
            Some(blake3_tag(b"<root>native content</root>"))
        );
        assert_eq!(
            (v.effective_status.as_str(), v.granularity),
            ("verified", Some("part"))
        );
        // A non-verified write keeps the baseline and ignores a supplied hash too.
        write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"a","location":"p2","claim_hash":"blake3:bogus"}]}"#),
            false,
        )
        .unwrap();
        assert_eq!(
            one(&file, "a").record.claim_hash,
            Some(blake3_tag(b"<root>native content</root>"))
        );

        set_entry(&file, "word/document.xml", "<root>edited</root>");
        let v = one(&file, "a");
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason),
            ("stale", Some("claim_changed"))
        );
        assert!(write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"x","status":"bogus"}]}"#),
            false
        )
        .is_err());
        assert!(write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"x","status":"verified"}]}"#),
            false
        )
        .is_err());
    }

    #[test]
    fn ooxml_knowledge_sources_filter_baseline_and_delete() {
        let file = pptx_with_slide();
        assert!(sources(&file, None, None).unwrap().sources.is_empty());
        let none = reconcile(&file, false).unwrap();
        assert!(none.invalidated.is_empty() && !none.written);

        write(
            &file,
            "sources",
            &patch(r#"{"sources":[
                {"id":"s2","status":"verified","claim_part":"/ppt/slides/slide1.xml","claim_object_id":"2"},
                {"id":"s3","status":"manual","claim_part":"/ppt/slides/slide1.xml","claim_object_id":"3"},
                {"id":"p","claim_part":"/ppt/presentation.xml"},
                {"id":"gone","status":"unverified"}]}"#),
            false,
        )
        .unwrap();
        let ids = |r: SourcesResult| {
            r.sources
                .into_iter()
                .map(|v| v.record.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(sources(&file, Some("ppt/slides/slide1.xml"), None).unwrap()),
            vec!["s2", "s3"]
        );
        assert_eq!(
            ids(sources(&file, Some("/ppt/slides/slide1.xml"), Some("3")).unwrap()),
            vec!["s3"]
        );
        assert_eq!(ids(sources(&file, None, Some("2")).unwrap()), vec!["s2"]);
        assert_eq!(sources(&file, None, None).unwrap().trust, "document");
        let p = one(&file, "p");
        assert_eq!(
            (p.record.status.as_str(), p.effective_status.as_str()),
            ("unverified", "unverified")
        );

        let r = write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"gone","delete":true}]}"#),
            false,
        )
        .unwrap();
        assert!(!r.created);
        assert_eq!(r.sources, Some(vec!["s2".into(), "s3".into(), "p".into()]));

        // A verified record written by another tool without a baseline reads as stale.
        let part = text(&file, "customXml/ao-sources.xml");
        set_entry(
            &file,
            "customXml/ao-sources.xml",
            &part.replace("status=\"manual\"", "status=\"verified\""),
        );
        let v = one(&file, "s3");
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason),
            ("stale", Some("no_baseline"))
        );
        let rc = reconcile(&file, false).unwrap();
        assert_eq!(
            rc.invalidated
                .iter()
                .map(|i| (i.id.as_str(), i.reason))
                .collect::<Vec<_>>(),
            vec![("s3", "no_baseline")]
        );
        assert_eq!(rc.unchanged, 2);
    }

    #[test]
    fn ooxml_knowledge_sources_round_trip_escaping() {
        let recs = vec![SourceRecord {
            id: "a&\"b".into(),
            status: "verified".into(),
            fields: SourceFields {
                source_kind: Some("workbook".into()),
                checked_at: Some("2026-10-01".into()),
                resource_uri: Some("relative:a<b>.xlsx".into()),
                location: Some("Sheet 'x'!D12 & <y>".into()),
                claim_part: Some("/ppt/slides/slide7.xml".into()),
                claim_object_id: Some("12".into()),
                source_hash: Some("deadbeef".into()),
                source_hash_algorithm: Some("sha256".into()),
                external_claim: Some(ExternalClaim {
                    file: "sub/a&b.xlsx".into(),
                    sheet: Some("S<1>".into()),
                    cell: Some("B2".into()),
                    claim_hash: Some("blake3:11".into()),
                    ..Default::default()
                }),
            },
            claim_hash: Some("blake3:00".into()),
        }];
        let xml = serialize_sources(&recs);
        assert_eq!(
            parse_sources(&xml).unwrap(),
            (SCHEMA_VERSION.to_string(), recs)
        );
    }

    #[cfg(unix)]
    #[test]
    fn ooxml_knowledge_rewrite_keeps_permissions_and_refuses_readonly() {
        use std::os::unix::fs::PermissionsExt;
        let file = make_pkg("docx");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        write(&file, "agents", &patch(r#"{"summary":"a"}"#), false).unwrap();
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );

        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444)).unwrap();
        let before = contents(&file);
        let err = write(&file, "agents", &patch(r#"{"summary":"b"}"#), false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("read-only"), "{err}");
        assert_eq!(contents(&file), before);
    }

    #[test]
    fn ooxml_knowledge_illegal_xml_chars_rejected() {
        let file = make_pkg("pptx");
        for bad in [
            r#"{"summary":"a\u0001b"}"#,
            r#"{"sections":[{"id":"x","text":"a\u000Bb"}]}"#,
            r#"{"sections":[{"id":"x\u001F"}]}"#,
            r#"{"sections":[{"id":"x","format":"￾"}]}"#,
        ] {
            let err = write(&file, "agents", &patch(bad), false)
                .unwrap_err()
                .to_string();
            assert!(err.contains("XML 1.0"), "{bad}: {err}");
        }
        let err = write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"s","location":"D12\u0000"}]}"#),
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("location") && err.contains("XML 1.0"), "{err}");
        // Tab, LF and CR are legal and round-trip.
        write(
            &file,
            "agents",
            &patch(r#"{"summary":"a\tb","sections":[{"id":"x","text":"l1\nl2\r\n"}]}"#),
            false,
        )
        .unwrap();
        assert_eq!(read(&file, "agents", None).unwrap().summary, "a\tb");
    }

    #[test]
    fn ooxml_knowledge_newer_schema_version_not_merged() {
        assert!(!is_newer_schema("0.1") && !is_newer_schema("") && !is_newer_schema("0.0.9"));
        assert!(
            is_newer_schema("0.2")
                && is_newer_schema("0.10")
                && is_newer_schema("1")
                && is_newer_schema("x")
        );

        let file = pptx_with_slide();
        write(&file, "design", &patch(r#"{"summary":"v1"}"#), false).unwrap();
        let v2 = text(&file, "customXml/ao-design.xml")
            .replace("schemaVersion=\"0.1\"", "schemaVersion=\"0.2\"");
        set_entry(&file, "customXml/ao-design.xml", &v2);
        let err = write(&file, "design", &patch(r#"{"summary":"x"}"#), false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("0.2"), "{err}");
        write(&file, "design", &patch(r#"{"summary":"x"}"#), true).unwrap();

        write(&file, "sources", &patch(r#"{"sources":[{"id":"s1","status":"verified","claim_part":"ppt/slides/slide1.xml","claim_object_id":"3"}]}"#), false).unwrap();
        let v2 = text(&file, "customXml/ao-sources.xml")
            .replace("schemaVersion=\"0.1\"", "schemaVersion=\"0.2\"");
        set_entry(&file, "customXml/ao-sources.xml", &v2);
        set_entry(
            &file,
            "ppt/slides/slide1.xml",
            &slide("Title", "Revenue 99m"),
        );
        assert!(reconcile(&file, false)
            .unwrap_err()
            .to_string()
            .contains("0.2"));
        assert!(write(
            &file,
            "sources",
            &patch(r#"{"sources":[{"id":"s2"}]}"#),
            false
        )
        .is_err());
    }

    #[test]
    fn ooxml_knowledge_dotted_entry_name_accepted() {
        let file = make_pkg("pptx");
        set_entry(&file, "ppt/media/a..b.png", "png");
        write(&file, "agents", &patch(r#"{"summary":"a"}"#), false).unwrap();
        assert_eq!(text(&file, "ppt/media/a..b.png"), "png");
        set_entry(&file, "ppt/../evil", "x");
        assert!(discover(&file).unwrap_err().to_string().contains("unsafe"));
    }

    #[test]
    fn ooxml_knowledge_dangling_relationship_is_relinked_not_duplicated() {
        let file = make_pkg("xlsx");
        write(&file, "intent", &patch(r#"{"summary":"orig"}"#), false).unwrap();
        // LibreOffice's .xlsx save: the part, its props and rels are gone, the relationship stays.
        let entries: Vec<(String, String)> = contents(&file)
            .into_iter()
            .filter(|(n, _)| !n.starts_with("customXml/"))
            .map(|(n, b)| (n, String::from_utf8(b).unwrap()))
            .collect();
        let refs: Vec<(&str, &str)> = entries
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        zip_entries(&file, &refs);
        assert_eq!(
            list(&file).unwrap().parts[0].error.as_deref(),
            Some("part customXml/ao-intent.xml is missing")
        );

        let r = write(&file, "intent", &patch(r#"{"summary":"again"}"#), false).unwrap();
        assert!(r.created);
        assert_eq!(
            text(&file, "xl/_rels/workbook.xml.rels")
                .matches(REL_CUSTOM_XML)
                .count(),
            1
        );
        let l = list(&file).unwrap();
        assert_eq!(l.parts.len(), 1);
        assert_eq!(
            (l.parts[0].kind.as_str(), l.parts[0].error.is_none()),
            ("intent", true)
        );
        assert_eq!(read(&file, "intent", None).unwrap().summary, "again");
    }

    #[test]
    fn v02_pptx_doc_and_slide_units_are_listed_read_and_sourced() {
        let file = tmp_dir().join("v02.pptx");
        fixtures::v02_pptx(&file);
        let l = list(&file).unwrap();
        assert_eq!(l.format, "pptx");
        let doc = l
            .parts
            .iter()
            .find(|p| p.kind == "doc")
            .expect("doc listed");
        assert_eq!(
            (doc.part.as_str(), doc.rev, doc.source_count),
            ("customXml/item1.xml", Some(4), Some(1))
        );
        assert!(
            doc.summary.contains("purpose: persuade") && doc.error.is_none(),
            "{doc:?}"
        );
        let units = l
            .parts
            .iter()
            .find(|p| p.kind == "units")
            .expect("units listed");
        assert_eq!(
            (units.unit_count, units.stale_count, units.orphan_count),
            (Some(2), Some(1), Some(0))
        );
        assert_eq!(
            units.unit_parts.as_deref(),
            Some(
                &[
                    "customXml/item2.xml".to_string(),
                    "customXml/item3.xml".to_string()
                ][..]
            )
        );
        assert!(l.parts.iter().all(|p| p.error.is_none()), "{:?}", l.parts);

        let r = read_context(&file, "doc", None).unwrap();
        assert_eq!(r.trust, "document");
        let d = r.doc.as_ref().unwrap();
        assert_eq!(d.doc.intent["audience"], "seed investors");
        assert_eq!(
            d.doc.design["visualPlan"],
            "photo cover; one designed slide in 3"
        );
        assert_eq!(d.doc.prefs, vec!["No stock photos of handshakes"]);
        assert_eq!(r.units.len(), 2);
        let (u1, u2) = (&r.units[0], &r.units[1]);
        assert_eq!(
            (
                u1.ref_.as_str(),
                u1.n,
                u1.role.as_deref(),
                u1.constraints,
                u1.decisions,
                u1.sources
            ),
            ("256", Some(1), Some("cover"), 2, 1, 1)
        );
        assert_eq!(
            (u2.ref_.as_str(), u2.n, u2.status.as_str(), u2.constraints),
            ("257", Some(2), "stale", 1)
        );
        assert_eq!((r.stale, r.orphan), (Some(1), Some(0)));
        // One unit by number or by ref; the units kind leaves the doc out.
        let one = read_context(&file, "units", Some("2")).unwrap();
        assert!(one.doc.is_none());
        assert_eq!(one.units.len(), 1);
        assert_eq!(one.units[0].ref_, "257");
        assert_eq!(
            read_context(&file, "units", Some("256")).unwrap().units[0].n,
            Some(1)
        );
        assert!(read_context(&file, "units", Some("9"))
            .unwrap_err()
            .to_string()
            .contains("256"));
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["units"][0]["ref"], "256");

        let s = sources(&file, None, None).unwrap();
        assert!(s.sources.is_empty());
        assert_eq!(s.context_sources.len(), 2);
        let file_src = s
            .context_sources
            .iter()
            .find(|v| v.scope == "file")
            .unwrap();
        assert_eq!(
            (
                file_src.record["id"].as_str(),
                file_src.effective_status.as_str()
            ),
            ("f1", "verified")
        );
        let unit_src = s
            .context_sources
            .iter()
            .find(|v| v.scope == "unit")
            .unwrap();
        assert_eq!(
            (
                unit_src.unit.as_deref(),
                unit_src.unit_n,
                unit_src.effective_status.as_str()
            ),
            (Some("256"), Some(1), "unverified")
        );
        // Filter by the unit part; an object filter is a v0.1 notion and matches no v0.2 source.
        assert_eq!(
            sources(&file, Some("/customXml/item2.xml"), None)
                .unwrap()
                .context_sources
                .len(),
            1
        );
        assert!(sources(&file, None, Some("4"))
            .unwrap()
            .context_sources
            .is_empty());
    }

    #[test]
    fn v02_docx_units_take_numbers_from_the_index_and_count_orphans() {
        let file = tmp_dir().join("v02.docx");
        fixtures::v02_docx(&file);
        let l = list(&file).unwrap();
        let units = l.parts.iter().find(|p| p.kind == "units").unwrap();
        assert_eq!(units.part, "customXml/item2.xml");
        assert_eq!(
            (
                units.unit_count,
                units.stale_count,
                units.orphan_count,
                units.source_count
            ),
            (Some(3), Some(1), Some(1), Some(2))
        );
        assert!(units.unit_parts.is_none());
        let r = read_context(&file, "doc", None).unwrap();
        assert_eq!(r.doc.as_ref().unwrap().doc.rev, 7);
        let ns: Vec<Option<u64>> = r.units.iter().map(|u| u.n).collect();
        assert_eq!(ns, vec![Some(1), Some(2), None]);
        assert!(r.units.iter().all(|u| u.unit_kind == "section"));
        assert_eq!(r.units[2].status, "orphan");
        let s = sources(&file, None, None).unwrap();
        assert_eq!(
            s.context_sources
                .iter()
                .map(|v| (v.unit_n, v.effective_status.as_str()))
                .collect::<Vec<_>>(),
            vec![(Some(1), "verified"), (Some(2), "unverified")]
        );
    }

    #[test]
    fn knowledge_write_refuses_a_file_with_v02_context_and_leaves_it_untouched() {
        for (name, make) in [
            ("w.pptx", fixtures::v02_pptx as fn(&Path)),
            ("w.docx", fixtures::v02_docx),
        ] {
            let file = tmp_dir().join(name);
            make(&file);
            let before = contents(&file);
            for kind in ["design", "sources"] {
                let p = if kind == "sources" {
                    patch(r#"{"sources":[{"id":"s"}]}"#)
                } else {
                    patch(r#"{"summary":"x"}"#)
                };
                let err = write(&file, kind, &p, false).unwrap_err().to_string();
                assert!(
                    err.contains("doc_context_patch") && err.contains("one writer per file"),
                    "{err}"
                );
            }
            assert_eq!(contents(&file), before);
        }
    }

    #[test]
    fn a_legacy_deck_is_listed_and_readable_not_an_error() {
        let file = tmp_dir().join("legacy.pptx");
        fixtures::legacy_deck_pptx(&file);
        let l = list(&file).unwrap();
        assert_eq!(l.parts.len(), 1);
        let p = &l.parts[0];
        assert_eq!(
            (p.kind.as_str(), p.rev, p.source_count, p.error.as_deref()),
            ("legacy-deck", Some(3), Some(1), None)
        );
        assert!(
            p.summary.contains("purpose: persuade")
                && p.summary.contains("2 slide rows (1 with a role)"),
            "{}",
            p.summary
        );
        let r = read_context(&file, "legacy-deck", None).unwrap();
        assert_eq!(
            r.legacy_deck.as_ref().unwrap().design["register"],
            "keynote"
        );
        assert!(read_context(&file, "doc", None).is_err());
        // v0.1 writes beside a v0.1 deck still work (the TS server upgrades the deck on its write).
        assert!(
            write(&file, "intent", &patch(r#"{"summary":"s"}"#), false)
                .unwrap()
                .created
        );
    }
}
