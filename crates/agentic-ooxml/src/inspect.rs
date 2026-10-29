//! `inspect`: a read-only inventory of a package. Format, main part, every package part (size,
//! content type, hash, role), the AO parts found, and the native units (slides, sheets, headings).
//!
//! Limits, stated honestly: the native-unit view is a shallow structural read (slide order and the
//! first text run, sheet names with cell and formula counts, paragraph headings). It is not a
//! renderer and says nothing about how Office would display the file. Text it returns comes from
//! the document and is untrusted data (`"trust": "document"`).

use super::*;
use std::collections::HashMap;

/// Cap on native units listed (the same bound the v0.2 context reader uses).
const MAX_UNITS: usize = CTX_MAX_UNITS;
/// Longest text preview returned per unit, in characters.
const PREVIEW_CHARS: usize = 120;

/// `[Content_Types].xml`: extension defaults and per-part overrides.
#[derive(Debug, Default, Clone)]
pub(crate) struct ContentTypes {
    defaults: HashMap<String, String>,
    overrides: HashMap<String, String>,
}

impl ContentTypes {
    pub(crate) fn parse(xml: &str) -> Result<ContentTypes> {
        let mut r = Reader::from_str(xml.trim_start_matches('\u{feff}'));
        let mut out = ContentTypes::default();
        loop {
            match r.read_event()? {
                Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                    b"Default" => {
                        if let (Some(ext), Some(ct)) =
                            (attr(&e, "Extension")?, attr(&e, "ContentType")?)
                        {
                            out.defaults.insert(ext.to_ascii_lowercase(), ct);
                        }
                    }
                    b"Override" => {
                        if let (Some(p), Some(ct)) =
                            (attr(&e, "PartName")?, attr(&e, "ContentType")?)
                        {
                            out.overrides
                                .insert(p.trim_start_matches('/').to_ascii_lowercase(), ct);
                        }
                    }
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(out)
    }

    /// The content type of a part name (no leading `/`): an override wins over the extension
    /// default. Matching is case-insensitive, as OPC part names are.
    pub(crate) fn of(&self, part: &str) -> Option<&str> {
        let lower = part.to_ascii_lowercase();
        if let Some(ct) = self.overrides.get(&lower) {
            return Some(ct);
        }
        let ext = lower.rsplit_once('.')?.1;
        self.defaults.get(ext).map(String::as_str)
    }

    /// Part names (lowercased, no leading `/`) that have an explicit override.
    pub(crate) fn override_parts(&self) -> impl Iterator<Item = &str> {
        self.overrides.keys().map(String::as_str)
    }
}

pub(crate) fn load_content_types(zip: &mut Archive) -> Option<ContentTypes> {
    let xml = read_entry(zip, "[Content_Types].xml").ok()??;
    ContentTypes::parse(&xml).ok()
}

/// One package part (zip entry that is not a directory).
#[derive(Debug, Clone, Serialize)]
pub struct PartInfo {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    /// From `[Content_Types].xml`; `None` when the package declares none for this part.
    pub content_type: Option<String>,
    /// `main`, `native`, `relationships`, `content-types`, `ao`, `custom-xml` or `other`.
    pub role: &'static str,
    /// `blake3:<hex>` of the uncompressed bytes; absent when the part is over the read cap.
    pub hash: Option<String>,
}

/// One native unit: a slide, a sheet, or a heading paragraph.
#[derive(Debug, Clone, Serialize)]
pub struct NativeUnit {
    /// 1-based position in document order.
    pub n: usize,
    /// The package's own identifier: slide id, sheet name; empty for headings.
    pub id: String,
    /// The part that holds it (absent for docx headings, which live in the main part).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// First text on a slide, the heading text, or nothing for a sheet. Document data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cells: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formulas: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct InspectResult {
    pub file: String,
    pub format: String,
    pub main_part: String,
    pub trust: &'static str,
    pub entry_count: usize,
    pub uncompressed_bytes: u64,
    /// `slides`, `sheets` or `headings`.
    pub unit_kind: &'static str,
    pub units: Vec<NativeUnit>,
    pub ao_parts: Vec<DiscoveredPart>,
    pub parts: Vec<PartInfo>,
}

fn clip(s: &str) -> String {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    clip_chars(&t, PREVIEW_CHARS)
}

/// First `<a:t>` text on a slide.
fn slide_text(xml: &str) -> Option<String> {
    let mut r = Reader::from_str(xml);
    let mut in_t = false;
    let mut acc = String::new();
    loop {
        match r.read_event().ok()? {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_t = true,
            Event::Text(t) if in_t => acc.push_str(&t.unescape().ok()?),
            Event::End(e) if e.local_name().as_ref() == b"t" => {
                in_t = false;
                if !acc.trim().is_empty() {
                    return Some(clip(&acc));
                }
                acc.clear();
            }
            Event::Eof => return None,
            _ => {}
        }
    }
}

/// `(cells, formulas)` in a worksheet part.
fn sheet_counts(xml: &str) -> (usize, usize) {
    let mut r = Reader::from_str(xml);
    let (mut c, mut f) = (0, 0);
    while let Ok(ev) = r.read_event() {
        match ev {
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                b"c" => c += 1,
                b"f" => f += 1,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    (c, f)
}

/// Heading paragraphs of a document part: a `w:p` whose `w:pStyle` is `Title` or `Heading<N>`.
fn docx_headings(xml: &str) -> Vec<String> {
    let mut r = Reader::from_str(xml);
    let mut out = vec![];
    let mut in_p = false;
    let mut heading = false;
    let mut in_t = false;
    let mut acc = String::new();
    while let Ok(ev) = r.read_event() {
        match ev {
            Event::Start(e) if e.local_name().as_ref() == b"t" && in_p => in_t = true,
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                b"p" => {
                    in_p = true;
                    heading = false;
                    acc.clear();
                }
                b"pStyle" if in_p => {
                    let val = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.local_name().as_ref() == b"val")
                        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()));
                    if let Some(v) = val {
                        let l = v.to_ascii_lowercase().replace(' ', "");
                        heading = l == "title"
                            || l.strip_prefix("heading")
                                .is_some_and(|n| n.parse::<u8>().is_ok());
                    }
                }
                _ => {}
            },
            Event::Text(t) if in_t => {
                if let Ok(s) = t.unescape() {
                    acc.push_str(&s);
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                b"t" => in_t = false,
                b"p" => {
                    if heading && out.len() < MAX_UNITS {
                        out.push(clip(&acc));
                    }
                    in_p = false;
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    out
}

fn native_unit_list(
    zip: &mut Archive,
    main: &str,
    format: &str,
) -> (&'static str, Vec<NativeUnit>) {
    match format {
        "pptx" | "xlsx" => {
            let natives = native_units(zip, main, format)
                .ok()
                .flatten()
                .unwrap_or_default();
            let mut out = vec![];
            for (i, (id, part)) in natives.into_iter().enumerate() {
                let xml = read_bytes(zip, &part, MAX_CLAIM_PART_BYTES)
                    .ok()
                    .flatten()
                    .and_then(|b| String::from_utf8(b).ok());
                let mut u = NativeUnit {
                    n: i + 1,
                    id,
                    part: Some(part),
                    text: None,
                    cells: None,
                    formulas: None,
                };
                if let Some(xml) = xml {
                    if format == "pptx" {
                        u.text = slide_text(&xml);
                    } else {
                        let (c, f) = sheet_counts(&xml);
                        u.cells = Some(c);
                        u.formulas = Some(f);
                    }
                }
                out.push(u);
            }
            (if format == "pptx" { "slides" } else { "sheets" }, out)
        }
        "docx" => {
            let xml = read_bytes(zip, main, MAX_CLAIM_PART_BYTES)
                .ok()
                .flatten()
                .and_then(|b| String::from_utf8(b).ok());
            let heads = xml.map(|x| docx_headings(&x)).unwrap_or_default();
            let units = heads
                .into_iter()
                .enumerate()
                .map(|(i, t)| NativeUnit {
                    n: i + 1,
                    id: String::new(),
                    part: None,
                    text: Some(t),
                    cells: None,
                    formulas: None,
                })
                .collect();
            ("headings", units)
        }
        _ => ("none", vec![]),
    }
}

/// Inventory a package. Fails only when the file is not a readable package with an
/// `officeDocument` relationship; a broken AO part is listed with its `error`, not fatal.
pub fn inspect(file: &Path) -> Result<InspectResult> {
    let mut zip = open(file)?;
    let main = main_part(&mut zip)?;
    let format = format_of(&main).to_string();
    let (_, found) = scan(&mut zip)?;
    let ctx = load_context(&mut zip, &main, &found)?;
    let mut ao_parts: Vec<DiscoveredPart> = found
        .iter()
        .filter(|f| !matches!(f.parsed, Ok(Parsed::Doc(_)) | Ok(Parsed::Units(_))))
        .map(to_discovered)
        .collect();
    ao_parts.extend(ctx.discovered());

    // Parts that are AO payloads: the parts the scan recognised, plus the v0.2 unit parts.
    let mut ao_names: std::collections::HashSet<String> = found
        .iter()
        .filter(|f| f.kind != "unknown")
        .map(|f| f.part.clone())
        .collect();
    ao_names.extend(ctx.unit_parts.iter().cloned());
    let ct = load_content_types(&mut zip);

    let mut parts = vec![];
    let mut total = 0u64;
    for i in 0..zip.len() {
        let (name, size, csize, is_dir) = {
            let e = zip.by_index_raw(i)?;
            (
                e.name().to_string(),
                e.size(),
                e.compressed_size(),
                e.is_dir(),
            )
        };
        if is_dir {
            continue;
        }
        total += size;
        let role = if name == main {
            "main"
        } else if name == "[Content_Types].xml" {
            "content-types"
        } else if name.ends_with(".rels") {
            "relationships"
        } else if ao_names.contains(&name) {
            "ao"
        } else if name.starts_with("customXml/") {
            "custom-xml"
        } else {
            "native"
        };
        let hash = read_bytes(&mut zip, &name, MAX_CLAIM_PART_BYTES)
            .ok()
            .flatten()
            .map(|b| blake3_tag(&b));
        let content_type = ct.as_ref().and_then(|c| c.of(&name)).map(str::to_string);
        parts.push(PartInfo {
            name,
            size,
            compressed_size: csize,
            content_type,
            role,
            hash,
        });
    }
    let (unit_kind, units) = native_unit_list(&mut zip, &main, &format);
    Ok(InspectResult {
        file: file.display().to_string(),
        format,
        main_part: main,
        trust: "document",
        entry_count: parts.len(),
        uncompressed_bytes: total,
        unit_kind,
        units,
        ao_parts,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testpkg::*;

    #[test]
    fn inspect_pptx_lists_slides_parts_and_ao_parts() {
        let f = tmp_dir().join("a.pptx");
        pptx(&f);
        write(
            &f,
            "design",
            &serde_json::from_str(r#"{"summary":"s","sections":[{"id":"type","text":"Inter"}]}"#)
                .unwrap(),
            false,
        )
        .unwrap();
        let r = inspect(&f).unwrap();
        assert_eq!(
            (
                r.format.as_str(),
                r.main_part.as_str(),
                r.unit_kind,
                r.trust
            ),
            ("pptx", "ppt/presentation.xml", "slides", "document")
        );
        assert_eq!(r.units.len(), 2);
        assert_eq!(
            (r.units[0].id.as_str(), r.units[0].text.as_deref()),
            ("256", Some("Board update"))
        );
        assert_eq!(r.units[1].part.as_deref(), Some("ppt/slides/slide2.xml"));
        let role = |n: &str| r.parts.iter().find(|p| p.name == n).unwrap().role;
        assert_eq!(role("ppt/presentation.xml"), "main");
        assert_eq!(role("ppt/slides/slide1.xml"), "native");
        assert_eq!(role("customXml/ao-design.xml"), "ao");
        assert_eq!(role("[Content_Types].xml"), "content-types");
        assert_eq!(
            r.parts
                .iter()
                .find(|p| p.name == "ppt/slides/slide1.xml")
                .unwrap()
                .content_type
                .as_deref(),
            Some("application/vnd.openxmlformats-officedocument.presentationml.slide+xml")
        );
        assert!(r
            .parts
            .iter()
            .all(|p| p.hash.as_deref().is_some_and(|h| h.starts_with("blake3:"))));
        assert_eq!(r.ao_parts.len(), 1);
        assert_eq!(r.ao_parts[0].kind, "design");
        assert_eq!(r.entry_count, r.parts.len());
    }

    #[test]
    fn inspect_docx_headings_and_xlsx_sheets() {
        let d = tmp_dir();
        let (w, x) = (d.join("a.docx"), d.join("a.xlsx"));
        docx(&w);
        xlsx(&x);
        let r = inspect(&w).unwrap();
        assert_eq!((r.format.as_str(), r.unit_kind), ("docx", "headings"));
        assert_eq!(
            r.units
                .iter()
                .map(|u| u.text.clone().unwrap())
                .collect::<Vec<_>>(),
            vec!["Summary", "Risks"]
        );
        let r = inspect(&x).unwrap();
        assert_eq!((r.format.as_str(), r.unit_kind), ("xlsx", "sheets"));
        assert_eq!(
            (
                r.units[0].id.as_str(),
                r.units[0].cells,
                r.units[0].formulas
            ),
            ("Summary", Some(3), Some(1))
        );
    }

    #[test]
    fn inspect_refuses_a_non_package() {
        let f = tmp_dir().join("x.docx");
        std::fs::write(&f, b"not a zip").unwrap();
        assert!(inspect(&f).is_err());
    }
}
