//! Cross-document source claims (`ao:externalClaim`, an optional extension of the v0.1 `ao:source`).
//!
//! A source record can name a file next to the document (same directory or below) and a locator
//! inside it, plus a baseline hash of the VALUE text found there. Reading the sources opens the
//! target package read-only, with the same entry/size caps as every other read, extracts the value
//! and compares its blake3 with the baseline.
//!
//! What a "value" is:
//! * `.xlsx`: the cached cell content at `sheet` + `cell`: a number (`<v>`, normalised so `75` and
//!   `75.0` agree), a shared string, an inline string, a formula string (`t="str"`), a boolean or
//!   an error text. Formulas are NOT recalculated: a formula cell without a cached `<v>` has no
//!   readable value (`external_object_missing`).
//! * `.docx` / `.pptx`: the text of a part (or of one slide shape, by id, on `ppt/slides/`): the
//!   `t` runs, one line per paragraph. Formatting-only edits do not change it.
//!
//! The target is untrusted data. The path is checked lexically (no absolute path, no `..`, no
//! backslash, drive letter or empty segment), resolved through symlinks, and must stay under the
//! source file's directory and, when the caller supplies roots (the MCP server does), under one of
//! them. A file outside the roots is never opened.

use super::*;
use std::path::PathBuf;

pub const EXT_CHANGED: &str = "external_claim_changed";
pub const EXT_FILE_MISSING: &str = "external_file_missing";
pub const EXT_OBJECT_MISSING: &str = "external_object_missing";
pub const EXT_PATH_INVALID: &str = "external_path_invalid";
pub const EXT_OUTSIDE_ROOTS: &str = "external_outside_roots";
pub const EXT_UNREADABLE: &str = "external_unreadable";

const MAX_VALUE_VIEW_CHARS: usize = 500;

/// Where an external claim may be read from, beyond the always-on "at or below the source file's
/// directory" rule.
#[derive(Debug, Clone, Default)]
pub struct EvalOptions {
    /// When set, a target must resolve (symlinks followed) under one of these directories, or it is
    /// reported `external_outside_roots` and never opened. `None` = no extra confinement.
    pub allowed_roots: Option<Vec<PathBuf>>,
}

/// A claim about a value in another package. `claim_hash` is kernel-set and ignored on patch input.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExternalClaim {
    /// Relative path of the target file, for example `financial-model.xlsx`.
    pub file: String,
    /// `.xlsx` locator: sheet name and cell reference (`Summary` + `B1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<String>,
    /// `.docx`/`.pptx` locator: an OPC part name, and optionally a slide shape id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    /// `blake3:<hex>` of the canonical value text at the time of the last verification.
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub claim_hash: Option<String>,
}

/// The computed state of an external claim, reported next to the stored record.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExternalView {
    pub file: String,
    /// `Summary!B1`, `/ppt/slides/slide2.xml#5` or a part name.
    pub locator: String,
    /// Why the claim cannot be confirmed; absent when the current value matches the baseline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    /// The value read now (document data, clipped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_hash: Option<String>,
}

/// What evaluating a record needs to know about where it lives.
pub(crate) struct ExtCtx<'a> {
    pub base: PathBuf,
    pub opts: &'a EvalOptions,
}

impl<'a> ExtCtx<'a> {
    pub fn for_file(file: &Path, opts: &'a EvalOptions) -> Self {
        let base = match file.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        ExtCtx { base, opts }
    }
}

impl ExternalClaim {
    fn same_locator(&self, o: &ExternalClaim) -> bool {
        (
            &self.file,
            &self.sheet,
            &self.cell,
            &self.part,
            &self.object_id,
        ) == (&o.file, &o.sheet, &o.cell, &o.part, &o.object_id)
    }

    /// Carry the stored baseline over to a patched claim when the locator did not change.
    pub(crate) fn merged_over(&self, old: Option<&ExternalClaim>) -> ExternalClaim {
        let mut n = self.clone();
        n.claim_hash = old
            .filter(|o| o.same_locator(self))
            .and_then(|o| o.claim_hash.clone());
        n
    }

    fn is_workbook(&self) -> bool {
        ext_of(&self.file) == "xlsx"
    }

    pub fn locator(&self) -> String {
        match (&self.sheet, &self.cell, &self.part) {
            (Some(s), Some(c), _) => format!("{s}!{c}"),
            (_, _, Some(p)) => match &self.object_id {
                Some(o) => format!("{p}#{o}"),
                None => p.clone(),
            },
            _ => String::new(),
        }
    }

    /// Check and normalise a claim supplied by a caller. Errors name the rule that failed.
    pub(crate) fn normalize(&mut self) -> Result<()> {
        let segs = lexical_segments(&self.file)
            .map_err(|why| anyhow!("external claim file '{}' refused: {why}", self.file))?;
        self.file = segs.join("/");
        if !["docx", "pptx", "xlsx"].contains(&ext_of(&self.file).as_str()) {
            bail!(
                "external claim file '{}' must be a .docx, .pptx or .xlsx",
                self.file
            );
        }
        for (k, v) in [
            ("sheet", &self.sheet),
            ("cell", &self.cell),
            ("part", &self.part),
            ("object_id", &self.object_id),
        ] {
            check_xml_chars(&format!("external claim {k}"), v.as_deref().unwrap_or(""))?;
        }
        check_xml_chars("external claim file", &self.file)?;
        if self.is_workbook() {
            let (Some(sheet), Some(cell)) = (&self.sheet, &self.cell) else {
                bail!("an .xlsx external claim needs both sheet and cell");
            };
            if self.part.is_some() || self.object_id.is_some() {
                bail!("an .xlsx external claim takes sheet + cell, not part/object_id");
            }
            if sheet.is_empty() || sheet.chars().count() > 255 {
                bail!("external claim sheet name is empty or too long");
            }
            let c = cell.to_ascii_uppercase();
            let digits = c.trim_start_matches(|ch: char| ch.is_ascii_uppercase());
            let letters = c.len() - digits.len();
            if letters == 0
                || letters > 3
                || digits.is_empty()
                || digits.len() > 7
                || !digits.chars().all(|ch| ch.is_ascii_digit())
                || digits.starts_with('0')
            {
                bail!("external claim cell '{cell}' is not an A1 cell reference");
            }
            self.cell = Some(c);
        } else {
            let Some(part) = &self.part else {
                bail!("a .docx/.pptx external claim needs a part");
            };
            if self.sheet.is_some() || self.cell.is_some() {
                bail!("a .docx/.pptx external claim takes part (+ object_id), not sheet/cell");
            }
            if part.split('/').any(|s| s == "..") || part.contains('\\') || part.is_empty() {
                bail!("external claim part '{part}' is not a package part name");
            }
            self.part = Some(format!("/{}", part.trim_start_matches('/')));
        }
        self.claim_hash = None;
        Ok(())
    }
}

fn ext_of(file: &str) -> String {
    file.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// The lexical rules for a relative path: segments, with `.` dropped. Refuses anything that is not a
/// plain relative path below the starting directory.
fn lexical_segments(p: &str) -> std::result::Result<Vec<String>, &'static str> {
    if p.is_empty() {
        return Err("empty path");
    }
    if p.starts_with('/') {
        return Err("absolute path");
    }
    if p.contains('\\') {
        return Err("backslash in path");
    }
    if p.contains(':') {
        return Err("drive letter or scheme in path");
    }
    if p.chars().any(|c| c.is_control()) {
        return Err("control character in path");
    }
    let mut out = vec![];
    for seg in p.split('/') {
        match seg {
            "." => {}
            "" => return Err("empty path segment"),
            ".." => return Err("'..' segment"),
            s => out.push(s.to_string()),
        }
    }
    if out.is_empty() {
        return Err("empty path");
    }
    Ok(out)
}

/// Resolve the target to a canonical regular file, or the reason it cannot be used. The file is
/// only `stat`ed here, never opened.
fn resolve(ctx: &ExtCtx, rel: &str) -> std::result::Result<PathBuf, &'static str> {
    let segs = lexical_segments(rel).map_err(|_| EXT_PATH_INVALID)?;
    let base = ctx.base.canonicalize().map_err(|_| EXT_UNREADABLE)?;
    let mut joined = base.clone();
    joined.extend(&segs);
    let canon = match joined.canonicalize() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(EXT_FILE_MISSING),
        Err(_) => return Err(EXT_UNREADABLE),
    };
    if let Some(roots) = &ctx.opts.allowed_roots {
        let inside = roots.iter().any(|r| {
            let r = r.canonicalize().unwrap_or_else(|_| r.clone());
            canon.starts_with(r)
        });
        if !inside {
            return Err(EXT_OUTSIDE_ROOTS);
        }
    }
    if !canon.starts_with(&base) {
        return Err(EXT_PATH_INVALID);
    }
    match std::fs::metadata(&canon) {
        Ok(m) if m.is_file() => Ok(canon),
        Ok(_) => Err(EXT_UNREADABLE),
        Err(_) => Err(EXT_FILE_MISSING),
    }
}

/// Read the claim's current value text, or the reason it cannot be read.
pub(crate) fn read_value(
    ctx: &ExtCtx,
    ec: &ExternalClaim,
) -> std::result::Result<String, &'static str> {
    let target = resolve(ctx, &ec.file)?;
    let mut zip = open(&target).map_err(|_| EXT_UNREADABLE)?;
    let is_xlsx = ext_of(&ec.file) == "xlsx";
    match (is_xlsx, &ec.sheet, &ec.cell, &ec.part) {
        (true, Some(sheet), Some(cell), _) => xlsx_value(&mut zip, sheet, cell),
        (false, _, _, Some(part)) => package_text(&mut zip, part, ec.object_id.as_deref()),
        _ => Err(EXT_OBJECT_MISSING),
    }
}

/// Evaluate a claim against its baseline. `verified` says whether a missing baseline is a problem.
pub(crate) fn evaluate_external(ctx: &ExtCtx, ec: &ExternalClaim, verified: bool) -> ExternalView {
    let mut view = ExternalView {
        file: ec.file.clone(),
        locator: ec.locator(),
        reason: None,
        current_value: None,
        current_hash: None,
    };
    match read_value(ctx, ec) {
        Err(r) => view.reason = Some(r),
        Ok(v) => {
            let h = blake3_tag(v.as_bytes());
            view.reason = match &ec.claim_hash {
                None if verified => Some("no_baseline"),
                Some(b) if *b != h => Some(EXT_CHANGED),
                _ => None,
            };
            view.current_value = Some(clip_chars(&v, MAX_VALUE_VIEW_CHARS));
            view.current_hash = Some(h);
        }
    }
    view
}

// ---------------------------------------------------------------------------------------------
// Value extraction
// ---------------------------------------------------------------------------------------------

fn unreadable<E>(_: E) -> &'static str {
    EXT_UNREADABLE
}

fn local_attr(e: &BytesStart, local: &[u8], prefixed: bool) -> Option<String> {
    for a in e.attributes().flatten() {
        let k = a.key;
        if k.local_name().as_ref() == local && k.prefix().is_some() == prefixed {
            return a.unescape_value().ok().map(|v| v.into_owned());
        }
    }
    None
}

fn read_cap(zip: &mut Archive, name: &str) -> std::result::Result<Option<Vec<u8>>, &'static str> {
    read_bytes(zip, name, MAX_CLAIM_PART_BYTES).map_err(unreadable)
}

/// Normalise a numeric cell so `75`, `75.0` and `7.5E1` hash alike; anything else is kept as is.
fn canonical_number(raw: &str) -> String {
    let t = raw.trim();
    match t.parse::<f64>() {
        Ok(f) if f.is_finite() => format!("{f}"),
        _ => t.to_string(),
    }
}

fn xlsx_value(
    zip: &mut Archive,
    sheet: &str,
    cell: &str,
) -> std::result::Result<String, &'static str> {
    let main = main_part(zip).map_err(unreadable)?;
    let wb = read_cap(zip, &main)?.ok_or(EXT_OBJECT_MISSING)?;
    let rels_name = rels_path(&main);
    let rels = match read_cap(zip, &rels_name)? {
        Some(b) => parse_rels(std::str::from_utf8(&b).map_err(unreadable)?).map_err(unreadable)?,
        None => vec![],
    };
    // sheet name -> relationship id
    let mut rid = None;
    let mut r = Reader::from_reader(wb.as_slice());
    loop {
        match r.read_event().map_err(unreadable)? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"sheet" => {
                if attr(&e, "name")
                    .map_err(unreadable)?
                    .is_some_and(|n| n.eq_ignore_ascii_case(sheet))
                {
                    rid = local_attr(&e, b"id", true);
                    break;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let rid = rid.ok_or(EXT_OBJECT_MISSING)?;
    let rel = rels
        .iter()
        .find(|r| r.id == rid && !r.external)
        .ok_or(EXT_OBJECT_MISSING)?;
    let sheet_part = resolve_target(&main, &rel.target).map_err(unreadable)?;
    let bytes = read_cap(zip, &sheet_part)?.ok_or(EXT_OBJECT_MISSING)?;

    let c = find_cell(&bytes, cell)?.ok_or(EXT_OBJECT_MISSING)?;
    let value = match c.t.as_deref() {
        Some("s") => {
            let idx: usize =
                c.v.as_deref()
                    .and_then(|v| v.trim().parse().ok())
                    .ok_or(EXT_OBJECT_MISSING)?;
            let sst = rels
                .iter()
                .find(|r| r.rel_type.ends_with("/sharedStrings") && !r.external)
                .map(|r| resolve_target(&main, &r.target))
                .transpose()
                .map_err(unreadable)?
                .unwrap_or_else(|| "xl/sharedStrings.xml".into());
            let sb = read_cap(zip, &sst)?.ok_or(EXT_OBJECT_MISSING)?;
            shared_string(&sb, idx)?.ok_or(EXT_OBJECT_MISSING)?
        }
        Some("inlineStr") => c.inline.ok_or(EXT_OBJECT_MISSING)?,
        Some("str") | Some("e") | Some("d") => c.v.ok_or(EXT_OBJECT_MISSING)?,
        Some("b") => match c.v.as_deref().map(str::trim) {
            Some("1") => "TRUE".to_string(),
            Some(_) => "FALSE".to_string(),
            None => return Err(EXT_OBJECT_MISSING),
        },
        _ => canonical_number(&c.v.ok_or(EXT_OBJECT_MISSING)?),
    };
    Ok(value)
}

struct Cell {
    t: Option<String>,
    v: Option<String>,
    inline: Option<String>,
}

fn find_cell(xml: &[u8], cell: &str) -> std::result::Result<Option<Cell>, &'static str> {
    let mut r = Reader::from_reader(xml);
    let mut cur: Option<Cell> = None;
    // 1 = inside <v>, 2 = inside <is>/<t>
    let (mut in_v, mut in_is, mut in_t, mut in_rph) = (false, false, false, false);
    loop {
        match r.read_event().map_err(unreadable)? {
            Event::Empty(e) if e.local_name().as_ref() == b"c" && cur.is_none() => {
                if attr(&e, "r")
                    .map_err(unreadable)?
                    .is_some_and(|n| n.eq_ignore_ascii_case(cell))
                {
                    return Ok(Some(Cell {
                        t: None,
                        v: None,
                        inline: None,
                    }));
                }
            }
            Event::Start(e) => match (e.local_name().as_ref(), cur.is_some()) {
                (b"c", false) => {
                    if attr(&e, "r")
                        .map_err(unreadable)?
                        .is_some_and(|n| n.eq_ignore_ascii_case(cell))
                    {
                        cur = Some(Cell {
                            t: attr(&e, "t").map_err(unreadable)?,
                            v: None,
                            inline: None,
                        });
                    }
                }
                (b"v", true) => in_v = true,
                (b"is", true) => in_is = true,
                (b"rPh", true) if in_is => in_rph = true,
                (b"t", true) if in_is && !in_rph => in_t = true,
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                b"c" if cur.is_some() => return Ok(cur),
                b"v" => in_v = false,
                b"is" => in_is = false,
                b"rPh" => in_rph = false,
                b"t" => in_t = false,
                _ => {}
            },
            Event::Text(t) => {
                if let Some(c) = cur.as_mut() {
                    let s = t.unescape().map_err(unreadable)?;
                    if in_v {
                        c.v.get_or_insert_with(String::new).push_str(&s);
                    } else if in_t {
                        c.inline.get_or_insert_with(String::new).push_str(&s);
                    }
                }
            }
            Event::CData(t) => {
                if let Some(c) = cur.as_mut() {
                    let s = String::from_utf8_lossy(&t).into_owned();
                    if in_v {
                        c.v.get_or_insert_with(String::new).push_str(&s);
                    } else if in_t {
                        c.inline.get_or_insert_with(String::new).push_str(&s);
                    }
                }
            }
            Event::Eof => return Ok(None),
            _ => {}
        }
    }
}

/// The text of the `idx`-th `<si>` (phonetic runs excluded).
fn shared_string(xml: &[u8], idx: usize) -> std::result::Result<Option<String>, &'static str> {
    let mut r = Reader::from_reader(xml);
    let (mut n, mut inside, mut in_t, mut in_rph) = (0usize, false, false, false);
    let mut out = String::new();
    loop {
        match r.read_event().map_err(unreadable)? {
            Event::Start(e) => match e.local_name().as_ref() {
                b"si" => {
                    inside = n == idx;
                    n += 1;
                }
                b"rPh" => in_rph = true,
                b"t" if inside && !in_rph => in_t = true,
                _ => {}
            },
            Event::Empty(e) if e.local_name().as_ref() == b"si" => {
                if n == idx {
                    return Ok(Some(String::new()));
                }
                n += 1;
            }
            Event::End(e) => match e.local_name().as_ref() {
                b"si" if inside => return Ok(Some(out)),
                b"rPh" => in_rph = false,
                b"t" => in_t = false,
                _ => {}
            },
            Event::Text(t) if in_t => out.push_str(&t.unescape().map_err(unreadable)?),
            Event::CData(t) if in_t => out.push_str(&String::from_utf8_lossy(&t)),
            Event::Eof => return Ok(None),
            _ => {}
        }
    }
}

/// The `t` text runs of a WordprocessingML/PresentationML/DrawingML fragment, one line per
/// paragraph, trimmed.
fn runs_text(xml: &[u8]) -> std::result::Result<String, &'static str> {
    let mut r = Reader::from_reader(xml);
    r.config_mut().check_end_names = false;
    let (mut out, mut in_t) = (String::new(), false);
    loop {
        match r.read_event().map_err(unreadable)? {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_t = true,
            Event::End(e) => match e.local_name().as_ref() {
                b"t" => in_t = false,
                b"p" => out.push('\n'),
                _ => {}
            },
            Event::Text(t) if in_t => out.push_str(&t.unescape().map_err(unreadable)?),
            Event::CData(t) if in_t => out.push_str(&String::from_utf8_lossy(&t)),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string())
}

fn package_text(
    zip: &mut Archive,
    part: &str,
    object: Option<&str>,
) -> std::result::Result<String, &'static str> {
    let name = part.trim_start_matches('/');
    let bytes = read_cap(zip, name)?.ok_or(EXT_OBJECT_MISSING)?;
    if let Some(id) = object.filter(|_| name.starts_with("ppt/slides/")) {
        let range = find_object(&bytes, id)
            .map_err(unreadable)?
            .ok_or(EXT_OBJECT_MISSING)?;
        return runs_text(&bytes[range]);
    }
    runs_text(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testpkg::*;

    const SML: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    /// A workbook with a number, a shared string (with a phonetic run), an inline string, a
    /// string formula, a boolean, a formula with no cached value and an empty cell.
    fn workbook(file: &Path, b1: &str) {
        let sheet = format!("<worksheet xmlns=\"{SML}\"><sheetData><row r=\"1\"><c r=\"A1\" t=\"s\"><v>1</v></c><c r=\"B1\"><v>{b1}</v></c><c r=\"C1\" t=\"inlineStr\"><is><t>inline</t></is></c><c r=\"D1\" t=\"str\"><f>A1</f><v>fs</v></c><c r=\"E1\" t=\"b\"><v>1</v></c><c r=\"F1\"><f>B1*2</f></c><c r=\"G1\"/></row></sheetData></worksheet>");
        zip_to(
            file,
            &[
                ("[Content_Types].xml", content_types(&[])),
                ("_rels/.rels", rels(&[("rId1", "officeDocument", "xl/workbook.xml")])),
                ("xl/workbook.xml", format!("<workbook xmlns=\"{SML}\" xmlns:r=\"{R}\"><sheets><sheet name=\"Summary\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>")),
                ("xl/_rels/workbook.xml.rels", rels(&[("rId1", "worksheet", "worksheets/sheet1.xml"), ("rId2", "sharedStrings", "sharedStrings.xml")])),
                ("xl/worksheets/sheet1.xml", sheet),
                ("xl/sharedStrings.xml", format!("<sst xmlns=\"{SML}\"><si><t>zero</t></si><si><r><t>Re</t></r><r><t>venue</t></r><rPh><t>PHON</t></rPh></si></sst>")),
            ],
        );
    }

    fn deck_with(dir: &Path, claim: serde_json::Value) -> PathBuf {
        let deck = dir.join("deck.pptx");
        pptx(&deck);
        let patch: DocPatch = serde_json::from_value(
            serde_json::json!({"sources": [{"id": "x", "status": "verified", "claim_part": "ppt/slides/slide1.xml", "external_claim": claim}]}),
        )
        .unwrap();
        write(&deck, "sources", &patch, false).unwrap();
        deck
    }

    fn cell_claim(file: &str, cell: &str) -> serde_json::Value {
        serde_json::json!({"file": file, "sheet": "Summary", "cell": cell})
    }

    fn view(deck: &Path) -> SourceView {
        sources(deck, None, None).unwrap().sources.remove(0)
    }

    fn val(file: &Path, cell: &str) -> std::result::Result<String, &'static str> {
        let d = file.parent().unwrap().to_path_buf();
        let o = EvalOptions::default();
        let ctx = ExtCtx { base: d, opts: &o };
        read_value(
            &ctx,
            &ExternalClaim {
                file: file.file_name().unwrap().to_str().unwrap().into(),
                sheet: Some("Summary".into()),
                cell: Some(cell.into()),
                ..Default::default()
            },
        )
    }

    #[test]
    fn xlsx_values_resolve_numbers_shared_inline_formula_strings_and_booleans() {
        let d = tmp_dir();
        let wb = d.join("m.xlsx");
        workbook(&wb, "7.50");
        assert_eq!(val(&wb, "B1").unwrap(), "7.5", "numbers are normalised");
        assert_eq!(
            val(&wb, "A1").unwrap(),
            "Revenue",
            "shared string, no phonetic run"
        );
        assert_eq!(val(&wb, "C1").unwrap(), "inline");
        assert_eq!(val(&wb, "D1").unwrap(), "fs");
        assert_eq!(val(&wb, "E1").unwrap(), "TRUE");
        assert_eq!(val(&wb, "F1"), Err(EXT_OBJECT_MISSING), "no cached value");
        assert_eq!(val(&wb, "G1"), Err(EXT_OBJECT_MISSING), "empty cell");
        assert_eq!(val(&wb, "Z9"), Err(EXT_OBJECT_MISSING));
    }

    #[test]
    fn a_verified_external_claim_goes_stale_when_the_value_changes_and_recovers_on_reverify() {
        let d = tmp_dir();
        workbook(&d.join("m.xlsx"), "18.6");
        let deck = deck_with(&d, cell_claim("m.xlsx", "b1"));
        let v = view(&deck);
        assert_eq!(v.effective_status, "verified", "{:?}", v.external_check);
        let ec = v.record.fields.external_claim.as_ref().unwrap();
        assert_eq!(ec.cell.as_deref(), Some("B1"), "cell refs are normalised");
        assert_eq!(ec.claim_hash.as_deref(), Some(blake3_tag(b"18.6").as_str()));
        assert_eq!(
            v.external_check.unwrap().current_value.as_deref(),
            Some("18.6")
        );
        // 18.60 is the same number; 19.1 is not
        workbook(&d.join("m.xlsx"), "18.60");
        assert_eq!(view(&deck).effective_status, "verified");
        workbook(&d.join("m.xlsx"), "19.1");
        let v = view(&deck);
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason),
            ("stale", Some(EXT_CHANGED))
        );
        assert_eq!(
            v.external_check.unwrap().current_value.as_deref(),
            Some("19.1")
        );
        // reconcile persists it, with the reason
        let r = reconcile(&deck, false).unwrap();
        assert_eq!(r.invalidated[0].reason, EXT_CHANGED);
        // re-baseline through a write
        let p: DocPatch =
            serde_json::from_str(r#"{"sources":[{"id":"x","status":"verified"}]}"#).unwrap();
        write(&deck, "sources", &p, false).unwrap();
        let v = view(&deck);
        assert_eq!(v.effective_status, "verified");
        assert_eq!(
            v.record
                .fields
                .external_claim
                .unwrap()
                .claim_hash
                .as_deref(),
            Some(blake3_tag(b"19.1").as_str())
        );
    }

    #[test]
    fn missing_file_and_missing_object_have_their_own_reasons() {
        let d = tmp_dir();
        workbook(&d.join("m.xlsx"), "1");
        let deck = deck_with(&d, cell_claim("m.xlsx", "B1"));
        // object gone: baseline cell is now empty in the workbook
        std::fs::remove_file(d.join("m.xlsx")).unwrap();
        let v = view(&deck);
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason),
            ("stale", Some(EXT_FILE_MISSING))
        );
        let sheet_only = d.join("m.xlsx");
        zip_to(
            &sheet_only,
            &[
                ("[Content_Types].xml", content_types(&[])),
                ("_rels/.rels", rels(&[("rId1", "officeDocument", "xl/workbook.xml")])),
                ("xl/workbook.xml", format!("<workbook xmlns=\"{SML}\" xmlns:r=\"{R}\"><sheets><sheet name=\"Other\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>")),
                ("xl/_rels/workbook.xml.rels", rels(&[("rId1", "worksheet", "worksheets/sheet1.xml")])),
                ("xl/worksheets/sheet1.xml", format!("<worksheet xmlns=\"{SML}\"/>")),
            ],
        );
        assert_eq!(
            view(&deck).stale_reason,
            Some(EXT_OBJECT_MISSING),
            "sheet gone"
        );
        workbook(&d.join("m.xlsx"), "1");
        assert_eq!(view(&deck).effective_status, "verified");
        // not a package at all
        std::fs::write(d.join("m.xlsx"), b"not a zip").unwrap();
        assert_eq!(view(&deck).stale_reason, Some(EXT_UNREADABLE));
    }

    #[test]
    fn a_non_verified_record_reports_the_external_state_but_keeps_its_status() {
        let d = tmp_dir();
        workbook(&d.join("m.xlsx"), "1");
        let deck = deck_with(&d, cell_claim("m.xlsx", "B1"));
        let p: DocPatch =
            serde_json::from_str(r#"{"sources":[{"id":"x","status":"manual"}]}"#).unwrap();
        write(&deck, "sources", &p, false).unwrap();
        workbook(&d.join("m.xlsx"), "2");
        let v = view(&deck);
        assert_eq!(v.effective_status, "manual");
        assert_eq!(v.external_check.unwrap().reason, Some(EXT_CHANGED));
    }

    #[test]
    fn docx_and_pptx_targets_hash_text_not_markup() {
        let d = tmp_dir();
        docx(&d.join("r.docx"));
        pptx(&d.join("p.pptx"));
        let deck = deck_with(
            &d,
            serde_json::json!({"file": "r.docx", "part": "word/document.xml"}),
        );
        assert_eq!(view(&deck).effective_status, "verified");
        let p: DocPatch = serde_json::from_value(serde_json::json!({"sources": [
            {"id": "y", "status": "verified", "claim_part": "ppt/slides/slide1.xml",
             "external_claim": {"file": "p.pptx", "part": "/ppt/slides/slide1.xml", "object_id": "3"}}]}))
        .unwrap();
        write(&deck, "sources", &p, false).unwrap();
        let s = sources(&deck, None, None).unwrap();
        let y = s.sources.iter().find(|v| v.record.id == "y").unwrap();
        assert_eq!(
            y.external_check.as_ref().unwrap().current_value.as_deref(),
            Some("Q3 revenue EUR 18.6M")
        );
        // an edit to the other shape does not matter; a markup-only edit does not matter; text does
        let edits = |find: &str, rep: &str| {
            let mut zip = open(&d.join("p.pptx")).unwrap();
            let s = read_entry(&mut zip, "ppt/slides/slide1.xml")
                .unwrap()
                .unwrap();
            drop(zip);
            rewrite_package(
                &d.join("p.pptx"),
                &[(
                    "ppt/slides/slide1.xml".into(),
                    s.replace(find, rep).into_bytes(),
                )],
            )
            .unwrap();
        };
        let status = |deck: &Path| {
            sources(deck, None, None)
                .unwrap()
                .sources
                .into_iter()
                .find(|v| v.record.id == "y")
                .unwrap()
                .effective_status
        };
        edits("Board update", "Board update v2");
        assert_eq!(status(&deck), "verified");
        edits(
            "<a:r><a:t>Q3 revenue EUR 18.6M",
            "<a:r><a:rPr b=\"1\"/><a:t>Q3 revenue EUR 18.6M",
        );
        assert_eq!(status(&deck), "verified");
        edits("EUR 18.6M", "EUR 19.1M");
        assert_eq!(status(&deck), "stale");
        // shape removed
        edits("id=\"3\"", "id=\"30\"");
        let s = sources(&deck, None, None).unwrap();
        let y = s.sources.iter().find(|v| v.record.id == "y").unwrap();
        assert_eq!(y.stale_reason, Some(EXT_OBJECT_MISSING));
    }

    #[test]
    fn traversal_and_absolute_paths_are_refused_on_write_and_on_read() {
        let d = tmp_dir();
        std::fs::create_dir_all(d.join("sub")).unwrap();
        let outside = d.join("outside.xlsx");
        workbook(&outside, "1");
        let deck_path = d.join("sub").join("deck.pptx");
        pptx(&deck_path);
        for bad in [
            "../outside.xlsx",
            "sub/../../outside.xlsx",
            "/etc/passwd.xlsx",
            outside.to_str().unwrap(),
            "..\\outside.xlsx",
            "C:/x.xlsx",
            "a//b.xlsx",
            "",
            "plain.txt",
        ] {
            let p: DocPatch = serde_json::from_value(serde_json::json!({"sources": [
                {"id": "x", "status": "unverified", "external_claim": {"file": bad, "sheet": "Summary", "cell": "B1"}}]}))
            .unwrap();
            let e = write(&deck_path, "sources", &p, false).unwrap_err();
            assert!(format!("{e:#}").contains("external claim"), "{bad}: {e:#}");
        }
        // a hand-edited part that smuggles one in is stale with a clear reason, never opened
        let xml = serialize_sources(&[SourceRecord {
            id: "x".into(),
            status: "verified".into(),
            fields: SourceFields {
                external_claim: Some(ExternalClaim {
                    file: "../outside.xlsx".into(),
                    sheet: Some("Summary".into()),
                    cell: Some("B1".into()),
                    claim_hash: Some(blake3_tag(b"1")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            claim_hash: None,
        }]);
        let p: DocPatch = serde_json::from_str(r#"{"sources":[{"id":"seed"}]}"#).unwrap();
        write(&deck_path, "sources", &p, false).unwrap();
        let mut zip = open(&deck_path).unwrap();
        let part = load_sources(&mut zip).unwrap().unwrap().0;
        drop(zip);
        rewrite_package(&deck_path, &[(part, xml.into_bytes())]).unwrap();
        let v = sources(&deck_path, None, None).unwrap().sources.remove(0);
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason),
            ("stale", Some(EXT_PATH_INVALID))
        );
        assert_eq!(v.external_check.unwrap().current_value, None);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_that_leaves_the_directory_is_refused() {
        let d = tmp_dir();
        let inner = d.join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        workbook(&d.join("real.xlsx"), "1");
        std::os::unix::fs::symlink(d.join("real.xlsx"), inner.join("link.xlsx")).unwrap();
        std::os::unix::fs::symlink(&d, inner.join("dirlink")).unwrap();
        let deck = inner.join("deck.pptx");
        pptx(&deck);
        for f in ["link.xlsx", "dirlink/real.xlsx"] {
            let p: DocPatch = serde_json::from_value(serde_json::json!({"sources": [
                {"id": "x", "status": "verified", "claim_part": "ppt/slides/slide1.xml",
                 "external_claim": {"file": f, "sheet": "Summary", "cell": "B1"}}]}))
            .unwrap();
            let e = write(&deck, "sources", &p, false).unwrap_err();
            assert!(format!("{e:#}").contains(EXT_PATH_INVALID), "{f}: {e:#}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn allowed_roots_confine_the_target_and_an_outside_file_is_never_opened() {
        let d = tmp_dir();
        let roots = d.join("roots");
        let outside = d.join("outside");
        std::fs::create_dir_all(&roots).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        // the outside target is a valid workbook whose value differs from the baseline: if it were
        // opened the reason would be external_claim_changed
        workbook(&outside.join("real.xlsx"), "99");
        workbook(&roots.join("m.xlsx"), "1");
        let deck = deck_with(&roots, cell_claim("m.xlsx", "B1"));
        std::fs::remove_file(roots.join("m.xlsx")).unwrap();
        std::os::unix::fs::symlink(outside.join("real.xlsx"), roots.join("m.xlsx")).unwrap();
        let opts = EvalOptions {
            allowed_roots: Some(vec![roots.canonicalize().unwrap()]),
        };
        let v = sources_with(&deck, None, None, &opts)
            .unwrap()
            .sources
            .remove(0);
        assert_eq!(v.stale_reason, Some(EXT_OUTSIDE_ROOTS));
        let ext = v.external_check.unwrap();
        assert_eq!(
            (ext.current_value, ext.current_hash),
            (None, None),
            "never opened"
        );
        // without roots the symlink is still refused, by the directory rule
        assert_eq!(view(&deck).stale_reason, Some(EXT_PATH_INVALID));
        // and a roots list that does not contain the source directory refuses a plain file too
        std::fs::remove_file(roots.join("m.xlsx")).unwrap();
        workbook(&roots.join("m.xlsx"), "1");
        let elsewhere = EvalOptions {
            allowed_roots: Some(vec![outside.canonicalize().unwrap()]),
        };
        assert_eq!(
            sources_with(&deck, None, None, &elsewhere).unwrap().sources[0].stale_reason,
            Some(EXT_OUTSIDE_ROOTS)
        );
        assert_eq!(
            sources_with(&deck, None, None, &opts).unwrap().sources[0].effective_status,
            "verified"
        );
    }

    #[test]
    fn verifying_refuses_when_the_external_value_cannot_be_read_and_old_files_still_parse() {
        let d = tmp_dir();
        let deck = d.join("deck.pptx");
        pptx(&deck);
        let p: DocPatch = serde_json::from_value(serde_json::json!({"sources": [
            {"id": "x", "status": "verified", "claim_part": "ppt/slides/slide1.xml",
             "external_claim": {"file": "missing.xlsx", "sheet": "Summary", "cell": "B1"}}]}))
        .unwrap();
        let e = write(&deck, "sources", &p, false).unwrap_err();
        assert!(format!("{e:#}").contains(EXT_FILE_MISSING), "{e:#}");
        // a record with no external claim serialises exactly as before
        let xml = serialize_sources(&[SourceRecord {
            id: "a".into(),
            status: "unverified".into(),
            fields: SourceFields::default(),
            claim_hash: None,
        }]);
        assert!(!xml.contains("externalClaim"));
        // an unknown sibling element and an externalClaim round trip through parse
        let xml = format!("<ao:sources xmlns:ao=\"{AO_NS}\" schemaVersion=\"0.1\"><ao:source id=\"a\" status=\"verified\"><ao:future/><ao:externalClaim file=\"m.xlsx\" sheet=\"S\" cell=\"B2\" claimHash=\"blake3:aa\"/></ao:source></ao:sources>");
        let (_, recs) = parse_sources(&xml).unwrap();
        let ec = recs[0].fields.external_claim.clone().unwrap();
        assert_eq!(
            (
                ec.file.as_str(),
                ec.cell.as_deref(),
                ec.claim_hash.as_deref()
            ),
            ("m.xlsx", Some("B2"), Some("blake3:aa"))
        );
        assert_eq!(parse_sources(&serialize_sources(&recs)).unwrap().1, recs);
    }
}
