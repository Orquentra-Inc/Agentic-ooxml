//! `validate`: structural checks on a package. It never modifies the file.
//!
//! What it checks:
//! 1. ZIP integrity: the archive opens, every entry reads to its end (which verifies its CRC),
//!    no entry name escapes the package, and no entry or compression ratio looks like a zip bomb.
//! 2. Every `.xml` and `.rels` part is well-formed XML (single root, balanced, valid entities and
//!    attributes, no DOCTYPE).
//! 3. `[Content_Types].xml` covers every part (an override or an extension default).
//! 4. Every internal relationship target exists, ids are unique, targets stay inside the package.
//! 5. Every AO part reachable from the package parses.
//! 6. Source claims: stale claims (changed or missing since verification) are reported.
//!
//! What it does NOT check: schema validity (no XSD validation of WordprocessingML, PresentationML
//! or SpreadsheetML), formulas, fonts, images, or anything about how Word, PowerPoint or Excel
//! would open the file. `ok: true` means "no structural errors found by these checks", not "Office
//! will render it". Staleness is reported as a warning: it is a state of the knowledge, not
//! corruption of the package.

use super::inspect::ContentTypes;
use super::*;
use std::collections::{HashMap, HashSet};

/// Limits applied while validating. Defaults suit real Office files; tests lower them.
#[derive(Debug, Clone)]
pub struct ValidateOptions {
    /// Largest uncompressed size accepted for one entry.
    pub max_entry_bytes: u64,
    /// Largest uncompressed size accepted for the whole package.
    pub max_total_bytes: u64,
    /// Largest uncompressed:compressed ratio accepted for an entry over `ratio_floor_bytes`.
    pub max_ratio: u64,
    /// Entries at or under this size are exempt from the ratio check (tiny parts compress oddly).
    pub ratio_floor_bytes: u64,
    /// XML parts larger than this are not parsed (reported as a warning).
    pub max_xml_bytes: u64,
    /// Where external source claims may be read from (see [`EvalOptions`]).
    pub eval: EvalOptions,
}

impl Default for ValidateOptions {
    fn default() -> Self {
        ValidateOptions {
            max_entry_bytes: 256 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
            max_ratio: 1000,
            ratio_floor_bytes: 1024 * 1024,
            max_xml_bytes: MAX_CLAIM_PART_BYTES,
            eval: EvalOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    /// Stable machine-readable code, e.g. `zip.entry-too-large`, `rels.target-missing`.
    pub code: &'static str,
    /// The package part the issue is about, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateResult {
    pub file: String,
    /// `docx`, `pptx`, `xlsx`, `unknown`, or absent when the package could not be opened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Messages can quote document data (part names, source ids): treat them as data.
    pub trust: &'static str,
    /// True when there are no errors. Warnings do not change it.
    pub ok: bool,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
    pub entries_checked: usize,
    pub xml_parts_checked: usize,
    pub relationships_checked: usize,
    pub ao_parts_checked: usize,
    pub source_claims_checked: usize,
    /// A reminder of what `ok` means.
    pub note: &'static str,
}

const NOTE: &str = "Structural checks only (zip, XML well-formedness, content types, relationships, AO parts, source staleness). No schema validation; this does not establish that Word, PowerPoint or Excel will open or render the file.";

struct Report {
    errors: Vec<Issue>,
    warnings: Vec<Issue>,
}

impl Report {
    fn err(&mut self, code: &'static str, part: Option<&str>, message: impl Into<String>) {
        self.errors.push(Issue {
            code,
            part: part.map(str::to_string),
            message: message.into(),
        });
    }
    fn warn(&mut self, code: &'static str, part: Option<&str>, message: impl Into<String>) {
        self.warnings.push(Issue {
            code,
            part: part.map(str::to_string),
            message: message.into(),
        });
    }
}

fn check_attrs(e: &BytesStart) -> std::result::Result<(), String> {
    for a in e.attributes() {
        let a = a.map_err(|e| format!("bad attribute: {e}"))?;
        a.unescape_value()
            .map_err(|e| format!("bad attribute value: {e}"))?;
    }
    Ok(())
}

/// Well-formedness of one XML document. `Err` carries a short reason.
fn check_xml(xml: &str) -> std::result::Result<(), String> {
    let mut r = Reader::from_str(xml.trim_start_matches('\u{feff}'));
    let (mut depth, mut roots) = (0usize, 0usize);
    loop {
        let ev = r
            .read_event()
            .map_err(|e| format!("{e} (near byte {})", r.buffer_position()))?;
        match ev {
            Event::Start(e) => {
                if depth == 0 {
                    roots += 1;
                }
                check_attrs(&e)?;
                depth += 1;
            }
            Event::Empty(e) => {
                if depth == 0 {
                    roots += 1;
                }
                check_attrs(&e)?;
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Text(t) => {
                t.unescape().map_err(|e| format!("bad text: {e}"))?;
                if depth == 0 && !t.iter().all(u8::is_ascii_whitespace) {
                    return Err("text outside the root element".into());
                }
            }
            Event::CData(_) if depth == 0 => return Err("CDATA outside the root element".into()),
            Event::DocType(_) => return Err("DOCTYPE is not allowed".into()),
            Event::Eof => break,
            _ => {}
        }
        if roots > 1 {
            return Err("more than one root element".into());
        }
    }
    if depth != 0 {
        return Err("unclosed element at end of file".into());
    }
    if roots == 0 {
        return Err("no root element".into());
    }
    Ok(())
}

fn is_xml_name(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.ends_with(".xml") || l.ends_with(".rels")
}

/// The part a `.rels` file describes: `word/_rels/document.xml.rels` -> `word/document.xml`,
/// `_rels/.rels` -> `""` (the package). `None` when the path is not in a `_rels` folder.
fn rels_source(rels: &str) -> Option<String> {
    let stem = rels.strip_suffix(".rels")?;
    if stem == "_rels/" || stem == "_rels/." || rels == "_rels/.rels" {
        return Some(String::new());
    }
    let (dir, file) = stem.rsplit_once('/')?;
    let parent = if dir == "_rels" {
        ""
    } else {
        dir.strip_suffix("/_rels")?
    };
    Some(if parent.is_empty() {
        file.to_string()
    } else {
        format!("{parent}/{file}")
    })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Validate with the default limits. See the module docs for what is and is not checked.
pub fn validate(file: &Path) -> Result<ValidateResult> {
    validate_with(file, &ValidateOptions::default())
}

/// Validate with explicit limits. `Err` only when the file cannot be opened at all (missing,
/// permissions); every problem with the package itself is returned as an error issue.
pub fn validate_with(file: &Path, opts: &ValidateOptions) -> Result<ValidateResult> {
    let f = std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?;
    let mut rep = Report {
        errors: vec![],
        warnings: vec![],
    };
    let mut res = ValidateResult {
        file: file.display().to_string(),
        format: None,
        trust: "document",
        ok: false,
        errors: vec![],
        warnings: vec![],
        entries_checked: 0,
        xml_parts_checked: 0,
        relationships_checked: 0,
        ao_parts_checked: 0,
        source_claims_checked: 0,
        note: NOTE,
    };
    let finish = |mut res: ValidateResult, rep: Report| {
        res.ok = rep.errors.is_empty();
        res.errors = rep.errors;
        res.warnings = rep.warnings;
        res
    };

    // 1. ZIP integrity ---------------------------------------------------------------------
    let mut zip = match zip::ZipArchive::new(f) {
        Ok(z) => z,
        Err(e) => {
            rep.err(
                "zip.invalid",
                None,
                format!("not a readable zip package: {e}"),
            );
            return Ok(finish(res, rep));
        }
    };
    if zip.len() > MAX_ENTRIES {
        rep.err(
            "zip.too-many-entries",
            None,
            format!("package has {} entries (cap {MAX_ENTRIES})", zip.len()),
        );
        return Ok(finish(res, rep));
    }
    let mut names: Vec<String> = vec![];
    let mut safe = true;
    let mut xml_texts: HashMap<String, String> = HashMap::new();
    let mut total: u64 = 0;
    let mut xml_checked = 0usize;
    for i in 0..zip.len() {
        let entry = match zip.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                rep.err(
                    "zip.entry-unreadable",
                    None,
                    format!("entry #{i} cannot be opened: {e}"),
                );
                continue;
            }
        };
        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }
        if name.split('/').any(|seg| seg == "..")
            || name.starts_with('/')
            || name.contains('\\')
            || name.contains('\0')
        {
            rep.err(
                "zip.unsafe-name",
                Some(&name),
                format!("unsafe entry name '{name}'"),
            );
            safe = false;
        }
        names.push(name.clone());
        let declared = entry.size();
        let csize = entry.compressed_size();
        if declared > opts.max_entry_bytes {
            rep.err(
                "zip.entry-too-large",
                Some(&name),
                format!(
                    "declares {declared} uncompressed bytes (cap {})",
                    opts.max_entry_bytes
                ),
            );
            continue;
        }
        if declared > opts.ratio_floor_bytes && declared / csize.max(1) > opts.max_ratio {
            rep.err(
                "zip.compression-ratio",
                Some(&name),
                format!(
                    "{declared} bytes from {csize} compressed ({}:1, cap {}:1): possible zip bomb",
                    declared / csize.max(1),
                    opts.max_ratio
                ),
            );
            continue;
        }
        // Read through a bounded reader: the declared size is not trusted. Reading to the end
        // also makes the zip layer verify the CRC.
        let xml = is_xml_name(&name) && declared <= opts.max_xml_bytes;
        let mut buf = Vec::new();
        let mut sink = std::io::sink();
        let read = if xml {
            entry
                .take(opts.max_entry_bytes + 1)
                .read_to_end(&mut buf)
                .map(|n| n as u64)
        } else {
            std::io::copy(&mut entry.take(opts.max_entry_bytes + 1), &mut sink)
        };
        match read {
            Ok(n) if n > opts.max_entry_bytes => {
                rep.err(
                    "zip.entry-too-large",
                    Some(&name),
                    format!(
                        "inflates past {} bytes though it declares {declared}",
                        opts.max_entry_bytes
                    ),
                );
                continue;
            }
            Ok(n) => {
                total += n;
                if n != declared {
                    rep.err(
                        "zip.size-mismatch",
                        Some(&name),
                        format!("declares {declared} bytes but holds {n}"),
                    );
                }
            }
            Err(e) => {
                rep.err(
                    "zip.corrupt",
                    Some(&name),
                    format!("cannot be read to the end (damaged data or CRC mismatch): {e}"),
                );
                continue;
            }
        }
        if total > opts.max_total_bytes {
            rep.err(
                "zip.total-too-large",
                None,
                format!("package inflates past {} bytes", opts.max_total_bytes),
            );
            res.entries_checked = names.len();
            return Ok(finish(res, rep));
        }
        // 2. XML well-formedness ---------------------------------------------------------
        if is_xml_name(&name) {
            if !xml {
                rep.warn("xml.not-parsed", Some(&name), format!("{declared} bytes is over the XML parse cap ({}); well-formedness not checked", opts.max_xml_bytes));
                continue;
            }
            xml_checked += 1;
            match String::from_utf8(buf) {
                Ok(text) => {
                    if let Err(why) = check_xml(&text) {
                        rep.err("xml.malformed", Some(&name), format!("not well-formed XML: {why}"));
                    }
                    if name.ends_with(".rels") || name == "[Content_Types].xml" {
                        xml_texts.insert(name.clone(), text);
                    }
                }
                Err(_) => rep.warn("xml.encoding", Some(&name), "not UTF-8 (UTF-16 parts are legal OOXML but are not checked by this validator)"),
            }
        }
    }
    res.entries_checked = names.len();
    res.xml_parts_checked = xml_checked;
    let name_set: HashSet<&str> = names.iter().map(String::as_str).collect();
    let lower_set: HashSet<String> = names.iter().map(|n| n.to_ascii_lowercase()).collect();
    let exists = |p: &str| name_set.contains(p) || lower_set.contains(&p.to_ascii_lowercase());

    // 3. [Content_Types].xml ---------------------------------------------------------------
    match xml_texts.get("[Content_Types].xml") {
        None if exists("[Content_Types].xml") => {} // present but unparsable: reported as xml.*
        None => rep.err(
            "content-types.missing",
            Some("[Content_Types].xml"),
            "package has no [Content_Types].xml",
        ),
        Some(t) => match ContentTypes::parse(t) {
            Err(e) => rep.err(
                "content-types.unreadable",
                Some("[Content_Types].xml"),
                format!("cannot be read: {e}"),
            ),
            Ok(ct) => {
                for n in &names {
                    if n != "[Content_Types].xml" && ct.of(n).is_none() {
                        rep.err("content-types.uncovered", Some(n), format!("no content type declared for {n} (no override and no default for its extension)"));
                    }
                }
                for p in ct.override_parts() {
                    if !lower_set.contains(p) {
                        rep.warn(
                            "content-types.override-missing-part",
                            Some(p),
                            format!("an override names {p}, which is not in the package"),
                        );
                    }
                }
            }
        },
    }

    // 4. Relationships ---------------------------------------------------------------------
    let mut linked_custom_xml: HashSet<String> = HashSet::new();
    let mut rels_checked = 0usize;
    let mut rels_names: Vec<&String> = xml_texts.keys().filter(|k| k.ends_with(".rels")).collect();
    rels_names.sort();
    for rn in rels_names {
        let Some(source) = rels_source(rn) else {
            rep.warn(
                "rels.misplaced",
                Some(rn),
                "a .rels part outside a _rels folder; relationships in it are ignored",
            );
            continue;
        };
        if !source.is_empty() && !exists(&source) {
            rep.warn(
                "rels.source-missing",
                Some(rn),
                format!("describes {source}, which is not in the package"),
            );
        }
        let Ok(rels) = parse_rels(&xml_texts[rn]) else {
            continue;
        };
        let mut ids = HashSet::new();
        for rel in &rels {
            rels_checked += 1;
            if rel.id.is_empty() {
                rep.err("rels.missing-id", Some(rn), "a relationship has no Id");
            } else if !ids.insert(rel.id.clone()) {
                rep.err(
                    "rels.duplicate-id",
                    Some(rn),
                    format!("relationship Id '{}' appears more than once", rel.id),
                );
            }
            if rel.external {
                continue;
            }
            if rel.target.is_empty() {
                rep.err(
                    "rels.missing-target",
                    Some(rn),
                    format!("relationship '{}' has no Target", rel.id),
                );
                continue;
            }
            match resolve_target(&source, &rel.target) {
                Err(e) => rep.err(
                    "rels.target-escapes",
                    Some(rn),
                    format!("relationship '{}': {e}", rel.id),
                ),
                Ok(t) => {
                    let t = if exists(&t) { t } else { percent_decode(&t) };
                    if !exists(&t) {
                        rep.err(
                            "rels.target-missing",
                            Some(rn),
                            format!(
                                "relationship '{}' targets {t}, which is not in the package",
                                rel.id
                            ),
                        );
                    } else if rel.rel_type == REL_CUSTOM_XML {
                        linked_custom_xml.insert(t);
                    }
                }
            }
        }
    }
    res.relationships_checked = rels_checked;

    // The main part and the format.
    if !safe {
        // Unsafe names: the checks below reopen the package by name, which refuses them.
        return Ok(finish(res, rep));
    }
    drop(zip);
    let mut az = match open(file) {
        Ok(z) => z,
        Err(e) => {
            rep.err("zip.invalid", None, format!("{e:#}"));
            return Ok(finish(res, rep));
        }
    };
    let main = match main_part(&mut az) {
        Ok(m) => m,
        Err(e) => {
            rep.err(
                "package.no-main-part",
                Some("_rels/.rels"),
                format!("{e:#}"),
            );
            return Ok(finish(res, rep));
        }
    };
    let format = format_of(&main);
    res.format = Some(format.to_string());
    if format == "unknown" {
        rep.warn(
            "package.unknown-format",
            Some(&main),
            format!("main part {main} is not under word/, ppt/ or xl/"),
        );
    }

    // 5. AO parts --------------------------------------------------------------------------
    match scan(&mut az) {
        Err(e) => rep.err("ao.scan-failed", None, format!("{e:#}")),
        Ok((main, found)) => {
            res.ao_parts_checked = found.iter().filter(|f| f.kind != "unknown").count();
            for f in &found {
                if let Err(e) = &f.parsed {
                    let msg = format!("{e:#}");
                    // A missing target is already reported as rels.target-missing.
                    if !(f.kind == "unknown" && msg.contains("is missing")) {
                        rep.err(
                            "ao.part-invalid",
                            Some(&f.part),
                            format!("AO part ({}) does not parse: {msg}", f.kind),
                        );
                    }
                }
            }
            match load_context(&mut az, &main, &found) {
                Err(e) => rep.err("ao.context-failed", None, format!("{e:#}")),
                Ok(ctx) => {
                    res.ao_parts_checked += ctx.unit_parts.len() + usize::from(ctx.doc.is_some());
                    for (part, msg) in &ctx.errors {
                        // `scan` already reported errors of parts it found itself.
                        if !found.iter().any(|f| &f.part == part && f.parsed.is_err()) {
                            rep.err(
                                "ao.part-invalid",
                                Some(part),
                                format!("v0.2 context part does not parse: {msg}"),
                            );
                        }
                    }
                    let (stale, orphan) = (ctx.stale(), ctx.orphan());
                    if stale > 0 {
                        rep.warn(
                            "context.units-stale",
                            None,
                            format!("{stale} v0.2 unit(s) are marked stale"),
                        );
                    }
                    if orphan > 0 {
                        rep.warn("context.units-orphan", None, format!("{orphan} v0.2 unit(s) refer to content that is no longer in the file"));
                    }
                    for p in &ctx.unit_parts {
                        linked_custom_xml.insert(p.clone());
                    }
                }
            }
            // An AO part under customXml/ that nothing relates to is invisible to every reader.
            let mut custom: Vec<&String> = names
                .iter()
                .filter(|n| n.starts_with("customXml/") && n.ends_with(".xml"))
                .collect();
            custom.sort();
            for n in custom {
                if linked_custom_xml.contains(n.as_str()) {
                    continue;
                }
                if let Ok(Some(x)) = read_entry(&mut az, n) {
                    if matches!(sniff_root(&x), Ok(Some(_))) {
                        rep.warn("ao.part-unlinked", Some(n), "an AO part that no customXml relationship points to; readers will not find it");
                    }
                }
            }
        }
    }

    // 6. Source staleness ------------------------------------------------------------------
    drop(az);
    match sources_with(file, None, None, &opts.eval) {
        Err(e) => rep.err("sources.unreadable", None, format!("{e:#}")),
        Ok(s) => {
            res.source_claims_checked = s.sources.len();
            for v in &s.sources {
                if v.effective_status == "stale" {
                    let why = match (v.record.status.as_str(), v.stale_reason) {
                        ("stale", _) => "recorded as stale".to_string(),
                        (_, Some(_)) if v.stale_reasons.len() > 1 => {
                            format!("verified, but now stale ({})", v.stale_reasons.join(", "))
                        }
                        (_, Some(r)) => format!("verified, but now stale ({r})"),
                        _ => "stale".to_string(),
                    };
                    rep.warn(
                        "sources.stale",
                        Some(&v.record.id),
                        format!("source '{}': {why}", v.record.id),
                    );
                }
            }
        }
    }
    Ok(finish(res, rep))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testpkg::*;

    fn codes(r: &ValidateResult) -> Vec<&'static str> {
        r.errors.iter().map(|i| i.code).collect()
    }
    fn patch_part(f: &Path, name: &str, text: &str) {
        rewrite_package(f, &[(name.to_string(), text.as_bytes().to_vec())]).unwrap();
    }

    #[test]
    fn clean_packages_validate_in_all_three_formats() {
        let d = tmp_dir();
        for (n, make) in [
            ("a.pptx", pptx as fn(&Path)),
            ("a.docx", docx),
            ("a.xlsx", xlsx),
        ] {
            let f = d.join(n);
            make(&f);
            let r = validate(&f).unwrap();
            assert!(r.ok, "{n}: {:?}", r.errors);
            assert!(r.warnings.is_empty(), "{n}: {:?}", r.warnings);
            assert!(r.xml_parts_checked >= 3 && r.relationships_checked >= 1);
            assert!(r.note.contains("does not establish"));
        }
    }

    #[test]
    fn a_file_written_by_this_library_still_validates() {
        let f = tmp_dir().join("a.pptx");
        pptx(&f);
        write(
            &f,
            "agents",
            &serde_json::from_str(
                r#"{"sections":[{"id":"policy","text":"Ask before deleting."}]}"#,
            )
            .unwrap(),
            false,
        )
        .unwrap();
        let r = validate(&f).unwrap();
        assert!(
            r.ok && r.warnings.is_empty(),
            "{:?} {:?}",
            r.errors,
            r.warnings
        );
        assert_eq!(r.ao_parts_checked, 1);
    }

    #[test]
    fn not_a_zip_is_an_error_issue_and_a_missing_file_is_an_err() {
        let d = tmp_dir();
        let f = d.join("x.docx");
        std::fs::write(&f, b"plain text").unwrap();
        let r = validate(&f).unwrap();
        assert!(!r.ok);
        assert_eq!(codes(&r), vec!["zip.invalid"]);
        assert!(validate(&d.join("missing.docx")).is_err());
    }

    #[test]
    fn malformed_xml_is_reported_for_xml_and_rels_parts() {
        let f = tmp_dir().join("a.docx");
        docx(&f);
        patch_part(&f, "word/document.xml", "<w:document><w:body></w:document>");
        let r = validate(&f).unwrap();
        assert!(
            r.errors.iter().any(
                |i| i.code == "xml.malformed" && i.part.as_deref() == Some("word/document.xml")
            ),
            "{:?}",
            r.errors
        );
        let g = tmp_dir().join("b.docx");
        docx(&g);
        patch_part(&g, "_rels/.rels", "<Relationships><Relationship Id=\"a\"");
        assert!(validate(&g)
            .unwrap()
            .errors
            .iter()
            .any(|i| i.code == "xml.malformed" && i.part.as_deref() == Some("_rels/.rels")));
        for bad in [
            "<a/><b/>",
            "<a>&nope;</a>",
            "<!DOCTYPE a [<!ENTITY x \"y\">]><a/>",
            "",
            "text<a/>",
            "<a x=\"1\" x=\"2\"/>",
        ] {
            assert!(check_xml(bad).is_err(), "{bad:?} should be rejected");
        }
        for good in [
            "<a/>",
            "\u{feff}<?xml version=\"1.0\"?>\n<a b=\"1\"><!-- c --><c>t &amp; u</c></a>\n",
        ] {
            assert!(check_xml(good).is_ok(), "{good:?}");
        }
    }

    #[test]
    fn a_part_with_no_content_type_is_an_error() {
        let f = tmp_dir().join("a.docx");
        docx(&f);
        // a .png has neither an override nor a default in the fixture
        rewrite_package(&f, &[("word/media/image1.png".into(), vec![1, 2, 3])]).unwrap();
        let r = validate(&f).unwrap();
        assert!(
            r.errors.iter().any(|i| i.code == "content-types.uncovered"
                && i.part.as_deref() == Some("word/media/image1.png")),
            "{:?}",
            r.errors
        );
    }

    #[test]
    fn missing_content_types_part_is_an_error() {
        let f = tmp_dir().join("a.docx");
        zip_to(
            &f,
            &[
                (
                    "_rels/.rels",
                    rels(&[("rId1", "officeDocument", "word/document.xml")]),
                ),
                ("word/document.xml", "<w:document xmlns:w=\"x\"/>".into()),
            ],
        );
        let r = validate(&f).unwrap();
        assert!(
            codes(&r).contains(&"content-types.missing"),
            "{:?}",
            r.errors
        );
    }

    #[test]
    fn relationship_targets_must_resolve_and_ids_must_be_unique() {
        let f = tmp_dir().join("a.pptx");
        pptx(&f);
        patch_part(
            &f,
            "ppt/_rels/presentation.xml.rels",
            &rels(&[
                ("rId2", "slide", "slides/slide1.xml"),
                ("rId2", "slide", "slides/slide2.xml"),
                ("rId3", "slide", "slides/slide9.xml"),
                ("rId4", "image", "../../../evil.png"),
                ("rId5", "hyperlink", "https://example.org/"),
            ])
            .replace(
                "Target=\"https://example.org/\"",
                "Target=\"https://example.org/\" TargetMode=\"External\"",
            ),
        );
        let r = validate(&f).unwrap();
        let c = codes(&r);
        assert!(c.contains(&"rels.target-missing"), "{c:?}");
        assert!(c.contains(&"rels.duplicate-id"), "{c:?}");
        assert!(c.contains(&"rels.target-escapes"), "{c:?}");
        // the external link is not an error
        assert_eq!(c.iter().filter(|x| **x == "rels.target-missing").count(), 1);
    }

    #[test]
    fn an_unparseable_ao_part_is_an_error_and_an_unlinked_one_a_warning() {
        let f = tmp_dir().join("a.docx");
        docx(&f);
        write(
            &f,
            "design",
            &serde_json::from_str(r#"{"sections":[{"id":"t","text":"x"}]}"#).unwrap(),
            false,
        )
        .unwrap();
        // an AO root with an unknown element: sniffs as AO, fails to parse
        patch_part(
            &f,
            "customXml/ao-design.xml",
            "<ao:bogus xmlns:ao=\"urn:agentic-office:ooxml:v0.1\"/>",
        );
        let r = validate(&f).unwrap();
        assert!(
            r.errors.iter().any(|i| i.code == "ao.part-invalid"
                && i.part.as_deref() == Some("customXml/ao-design.xml")),
            "{:?}",
            r.errors
        );

        let g = tmp_dir().join("b.docx");
        docx(&g);
        rewrite_package(&g, &[("customXml/orphan.xml".into(), b"<ao:knowledge xmlns:ao=\"urn:agentic-office:ooxml:v0.1\" kind=\"design\" schemaVersion=\"0.1\"/>".to_vec())]).unwrap();
        let r = validate(&g).unwrap();
        assert!(
            r.warnings.iter().any(|i| i.code == "ao.part-unlinked"),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn a_changed_claim_is_a_stale_warning_not_an_error() {
        let f = tmp_dir().join("a.pptx");
        pptx(&f);
        let p: DocPatch = serde_json::from_str(r#"{"sources":[{"id":"q3","status":"verified","claim_part":"ppt/slides/slide1.xml","claim_object_id":"3"}]}"#).unwrap();
        write(&f, "sources", &p, false).unwrap();
        let r = validate(&f).unwrap();
        assert!(r.ok && r.warnings.is_empty());
        assert_eq!(r.source_claims_checked, 1);
        let slide = SLIDE1.replace("18.6", "19.1");
        patch_part(&f, "ppt/slides/slide1.xml", &slide);
        let r = validate(&f).unwrap();
        assert!(r.ok, "{:?}", r.errors);
        assert!(
            r.warnings.iter().any(|i| i.code == "sources.stale"
                && i.part.as_deref() == Some("q3")
                && i.message.contains("claim_changed")),
            "{:?}",
            r.warnings
        );
        // reconcile persists it; the warning stays (recorded as stale)
        reconcile(&f, false).unwrap();
        let r = validate(&f).unwrap();
        assert!(r
            .warnings
            .iter()
            .any(|i| i.code == "sources.stale" && i.message.contains("recorded as stale")));
    }

    // --- zip bombs and damaged data ------------------------------------------------------

    fn bomb(file: &Path, size: usize) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(file).unwrap());
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (n, b) in [
            ("[Content_Types].xml", content_types(&[("/word/document.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"), ("/word/media/blob.bin", "application/octet-stream")]).into_bytes()),
            ("_rels/.rels", rels(&[("rId1", "officeDocument", "word/document.xml")]).into_bytes()),
            ("word/document.xml", b"<w:document xmlns:w=\"x\"/>".to_vec()),
            ("word/media/blob.bin", vec![0u8; size]),
        ] {
            w.start_file(n, opts).unwrap();
            w.write_all(&b).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn an_oversized_entry_is_refused_by_declared_size() {
        let f = tmp_dir().join("bomb.docx");
        bomb(&f, 8 * 1024 * 1024); // 8 MiB of zeros deflates to ~8 KiB
        let small = ValidateOptions {
            max_entry_bytes: 1024 * 1024,
            ..Default::default()
        };
        let r = validate_with(&f, &small).unwrap();
        assert!(!r.ok);
        assert!(
            r.errors.iter().any(|i| i.code == "zip.entry-too-large"
                && i.part.as_deref() == Some("word/media/blob.bin")),
            "{:?}",
            r.errors
        );
        // with generous size limits the same file trips the ratio guard instead
        let ratio = ValidateOptions {
            max_ratio: 100,
            ..Default::default()
        };
        let r = validate_with(&f, &ratio).unwrap();
        assert!(
            r.errors.iter().any(|i| i.code == "zip.compression-ratio"),
            "{:?}",
            r.errors
        );
        // and with limits that allow it, it is accepted (the guard is the limit, not the content)
        let lax = ValidateOptions {
            max_ratio: u64::MAX,
            ..Default::default()
        };
        assert!(validate_with(&f, &lax).unwrap().ok);
    }

    #[test]
    fn the_total_inflated_size_is_capped() {
        let f = tmp_dir().join("bomb.docx");
        bomb(&f, 4 * 1024 * 1024);
        let o = ValidateOptions {
            max_total_bytes: 1024 * 1024,
            max_ratio: u64::MAX,
            ..Default::default()
        };
        assert!(validate_with(&f, &o)
            .unwrap()
            .errors
            .iter()
            .any(|i| i.code == "zip.total-too-large"));
    }

    #[test]
    fn a_damaged_entry_fails_the_crc_check() {
        let f = tmp_dir().join("a.docx");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&f).unwrap());
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        w.start_file("[Content_Types].xml", stored).unwrap();
        w.write_all(content_types(&[]).as_bytes()).unwrap();
        w.start_file("word/data.bin", stored).unwrap();
        w.write_all(b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap();
        w.finish().unwrap();
        let mut bytes = std::fs::read(&f).unwrap();
        let at = bytes.windows(8).position(|w| w == b"AAAAAAAA").unwrap();
        bytes[at + 3] = b'B';
        std::fs::write(&f, bytes).unwrap();
        let r = validate(&f).unwrap();
        assert!(
            r.errors
                .iter()
                .any(|i| i.code == "zip.corrupt" && i.part.as_deref() == Some("word/data.bin")),
            "{:?}",
            r.errors
        );
    }

    #[test]
    fn rels_source_maps_back_to_the_described_part() {
        assert_eq!(rels_source("_rels/.rels").as_deref(), Some(""));
        assert_eq!(
            rels_source("word/_rels/document.xml.rels").as_deref(),
            Some("word/document.xml")
        );
        assert_eq!(
            rels_source("ppt/slides/_rels/slide1.xml.rels").as_deref(),
            Some("ppt/slides/slide1.xml")
        );
        assert_eq!(rels_source("odd/place.rels"), None);
    }
}
