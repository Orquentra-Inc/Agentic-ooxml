//! `make-examples`: deterministic generator and small helper tool for the synthetic examples.
//!
//! * `generate`: build board-deck.pptx, financial-model.xlsx and report.docx (native content plus
//!   AO parts written through the `agentic-ooxml` library), byte-reproducibly.
//! * `set-text`: replace exact text inside one package part. A DEMO helper so the walkthrough can
//!   change "Q3" on a slide without Office. It is not a document editor.
//! * `check-links`: compare the workbook cells a deck's sources claims quote against what the
//!   slides show. Example-level tooling: it reads cached cell values and flat shapes; the library
//!   itself does NOT look inside other documents.
//! * `link-patch`: print a `sources` patch that re-verifies links that now agree.
//!
//! Everything is synthetic (EUR M): Q1 17.2, Q2 17.9, Q3 18.6, Q4 forecast 21.3, annual 75.0.
//! Nothing here has been opened in Microsoft Office.

mod native;

use agentic_ooxml as ao;
use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde_json::json;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "make-examples",
    about = "Generate and manipulate the synthetic Agentic OOXML examples"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build the three example files into a directory (default: examples).
    Generate {
        #[arg(long, default_value = "examples")]
        out: PathBuf,
    },
    /// Replace exact text inside one part of a package (demo helper, not an editor).
    SetText {
        #[arg(long)]
        file: PathBuf,
        /// Part name inside the package, e.g. ppt/slides/slide2.xml.
        #[arg(long)]
        part: String,
        /// Text to find (raw XML text). Repeat with --replace, pairwise.
        #[arg(long)]
        find: Vec<String>,
        #[arg(long)]
        replace: Vec<String>,
    },
    /// Compare workbook cells quoted by a deck's sources with what the claimed shapes show.
    /// Exit 1 when any link is not consistent.
    CheckLinks {
        #[arg(long)]
        file: PathBuf,
        /// Directory holding the workbooks named by resource_uri (default: the file's directory).
        #[arg(long)]
        base: Option<PathBuf>,
    },
    /// Print a sources patch that re-verifies every link whose claim already shows the workbook value.
    LinkPatch {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        base: Option<PathBuf>,
        #[arg(long, default_value = "2026-10-29")]
        checked_at: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::Generate { out } => generate(&out),
        Cmd::SetText {
            file,
            part,
            find,
            replace,
        } => set_text(&file, &part, &find, &replace),
        Cmd::CheckLinks { file, base } => {
            let links = check_links(&file, base.as_deref())?;
            serde_json::to_writer_pretty(
                std::io::stdout(),
                &json!({"file": file.display().to_string(), "trust": "document", "links": links}),
            )?;
            println!();
            if links.iter().any(|l| l["verdict"] != "consistent") {
                std::process::exit(1);
            }
            Ok(())
        }
        Cmd::LinkPatch {
            file,
            base,
            checked_at,
        } => {
            let links = check_links(&file, base.as_deref())?;
            let mut patch = vec![];
            for l in &links {
                match l["verdict"].as_str() {
                    Some("needs_reverification") | Some("consistent") => patch.push(json!({
                        "id": l["id"], "status": "verified", "checked_at": checked_at,
                        "source_hash": l["workbook_value_hash"], "source_hash_algorithm": HASH_ALGO,
                    })),
                    _ => eprintln!("skipped {}: {}", l["id"], l["verdict"]),
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"sources": patch}))?
            );
            Ok(())
        }
    }
}

// ------------------------------------------------------------------------------ package io

fn write_zip(file: &Path, entries: &[(String, Vec<u8>)]) -> Result<()> {
    let mut w = zip::ZipWriter::new(std::fs::File::create(file)?);
    // A fixed timestamp and fixed order make the output byte-reproducible.
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (n, b) in entries {
        w.start_file(n.as_str(), opts)?;
        w.write_all(b)?;
    }
    w.finish()?;
    Ok(())
}

fn read_all(file: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut z = zip::ZipArchive::new(
        std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?,
    )?;
    let mut out = vec![];
    for i in 0..z.len() {
        let mut e = z.by_index(i)?;
        if e.is_dir() {
            continue;
        }
        let mut b = vec![];
        e.read_to_end(&mut b)?;
        out.push((e.name().to_string(), b));
    }
    Ok(out)
}

fn read_part(file: &Path, part: &str) -> Result<String> {
    let mut z = zip::ZipArchive::new(
        std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?,
    )?;
    let mut s = String::new();
    z.by_name(part)
        .with_context(|| format!("{part} not in {}", file.display()))?
        .read_to_string(&mut s)?;
    Ok(s)
}

/// Rewrite a package with the original entry order but fixed timestamps.
fn freeze(file: &Path) -> Result<()> {
    let entries = read_all(file)?;
    write_zip(file, &entries)
}

fn set_text(file: &Path, part: &str, find: &[String], replace: &[String]) -> Result<()> {
    if find.is_empty() || find.len() != replace.len() {
        bail!("pass the same number of --find and --replace (at least one)");
    }
    let mut text = read_part(file, part)?;
    for (f, r) in find.iter().zip(replace) {
        let n = text.matches(f.as_str()).count();
        if n != 1 {
            bail!("expected exactly one occurrence of {f:?} in {part}, found {n}; nothing was changed");
        }
        text = text.replacen(f.as_str(), r, 1);
    }
    ao::rewrite_package(file, &[(part.to_string(), text.into_bytes())])?;
    println!(
        "{}: {} replacement(s) in {part}",
        file.display(),
        find.len()
    );
    Ok(())
}

// ------------------------------------------------------------------------------ generate

const HASH_ALGO: &str = "blake3-of-cell-value";

fn value_hash(v: &str) -> String {
    format!("blake3:{}", blake3::hash(v.as_bytes()).to_hex())
}

fn put(file: &Path, kind: &str, patch: serde_json::Value) -> Result<()> {
    let patch: ao::DocPatch = serde_json::from_value(patch)?;
    ao::write(file, kind, &patch, false)
        .with_context(|| format!("writing {kind} into {}", file.display()))?;
    Ok(())
}

fn build(file: &Path, entries: native::Entries) -> Result<()> {
    write_zip(
        file,
        &entries
            .into_iter()
            .map(|(n, b)| (n, b.into_bytes()))
            .collect::<Vec<_>>(),
    )
}

fn generate(out: &Path) -> Result<()> {
    std::fs::create_dir_all(out)?;
    let (deck, model, report) = (
        out.join("board-deck.pptx"),
        out.join("financial-model.xlsx"),
        out.join("report.docx"),
    );
    build(&deck, native::board_deck())?;
    build(&model, native::financial_model())?;
    build(&report, native::report())?;
    let day = "2026-10-29";

    // ---- the deck
    put(
        &deck,
        "agents",
        json!({"summary": "Editing policy for this deck", "sections": [
        {"id": "figures", "text": "Every revenue figure on slides 2 and 3 is quoted from financial-model.xlsx and has a sources record. Change a figure only together with its source, and re-verify the record afterwards."},
        {"id": "style", "text": "Keep the graphite and cobalt look: one accent colour, thin rules, no decorative imagery."}]}),
    )?;
    put(
        &deck,
        "design",
        json!({"summary": "Graphite ground, electric cobalt accent, editorial type", "sections": [
        {"id": "palette", "text": "Ground near-black or white; text graphite; one accent, cobalt 2457F5; forecast bars in grey 8A93A0."},
        {"id": "layout", "text": "Title top-left; figures left-aligned in a single column with proportional bars to the right."}]}),
    )?;
    put(
        &deck,
        "intent",
        json!({"summary": "Quarterly board update on revenue", "sections": [
        {"id": "purpose", "text": "Show that FY revenue lands at EUR 75.0M and what the Q4 forecast depends on."},
        {"id": "audience", "text": "Board members who have read last quarter's deck; they want numbers and the ask, not background."},
        {"id": "slide-2", "text": "Revenue by quarter: Q1 to Q3 actual, Q4 forecast, bars scaled to value."}]}),
    )?;
    put(
        &deck,
        "sources",
        json!({"sources": [
        {"id": "q3-kpi", "status": "verified", "source_kind": "spreadsheet", "resource_uri": "financial-model.xlsx", "location": native::Q3_CELL,
         "claim_part": "ppt/slides/slide2.xml", "claim_object_id": native::Q3_SHAPE_ID, "checked_at": day,
         "source_hash": value_hash("18.6"), "source_hash_algorithm": HASH_ALGO,
         "external_claim": {"file": "financial-model.xlsx", "sheet": "Summary", "cell": "B1"}},
        {"id": "annual", "status": "verified", "source_kind": "spreadsheet", "resource_uri": "financial-model.xlsx", "location": native::ANNUAL_CELL,
         "claim_part": "ppt/slides/slide3.xml", "claim_object_id": native::ANNUAL_SHAPE_ID, "checked_at": day,
         "source_hash": value_hash("75"), "source_hash_algorithm": HASH_ALGO},
        {"id": "q4-forecast", "status": "manual", "source_kind": "spreadsheet", "resource_uri": "financial-model.xlsx", "location": "Quarterly!B5",
         "claim_part": "ppt/slides/slide2.xml", "claim_object_id": "6", "checked_at": day}]}),
    )?;

    // ---- the workbook
    put(
        &model,
        "agents",
        json!({"summary": "Editing policy for this model", "sections": [
        {"id": "inputs", "text": "Quarterly!B2:B5 are inputs; Summary and the annual total are formulas. Cached values are not recalculated by tools: open in a spreadsheet application to recalculate."}]}),
    )?;
    put(
        &model,
        "design",
        json!({"summary": "Plain model layout", "sections": [{"id": "layout", "text": "Inputs on Quarterly, outputs on Summary. Units are EUR M in column headers."}]}),
    )?;
    put(
        &model,
        "intent",
        json!({"summary": "Source of truth for the FY revenue figures quoted in board-deck.pptx and report.docx", "sections": [
        {"id": "consumers", "text": "Summary!B1 (Q3) and Summary!B2 (annual) are quoted by the deck and the report."}]}),
    )?;
    put(
        &model,
        "sources",
        json!({"sources": [
        {"id": "forecast-basis", "status": "manual", "source_kind": "memo", "resource_uri": "synthetic:forecast-memo", "claim_part": "xl/worksheets/sheet2.xml", "checked_at": day}]}),
    )?;

    // ---- the report
    put(
        &report,
        "agents",
        json!({"summary": "Editing policy for this report", "sections": [
        {"id": "figures", "text": "Figures come from financial-model.xlsx. Update the model first, then the text, then re-verify the source."}]}),
    )?;
    put(
        &report,
        "design",
        json!({"summary": "Report styling", "sections": [{"id": "headings", "text": "Title, Heading 1 for sections, Heading 2 for sub-sections; table with thin horizontal rules."}]}),
    )?;
    put(
        &report,
        "intent",
        json!({"summary": "Written companion to the board deck", "sections": [
        {"id": "purpose", "text": "Give the quarterly revenue table and the outlook in prose for readers who skip the deck."}]}),
    )?;
    put(
        &report,
        "sources",
        json!({"sources": [
        {"id": "annual-total", "status": "verified", "source_kind": "spreadsheet", "resource_uri": "financial-model.xlsx", "location": native::ANNUAL_CELL,
         "claim_part": "word/document.xml", "checked_at": day, "source_hash": value_hash("75"), "source_hash_algorithm": HASH_ALGO}]}),
    )?;

    for f in [&deck, &model, &report] {
        freeze(f)?;
        println!("wrote {}", f.display());
    }
    Ok(())
}

// ------------------------------------------------------------------------------ links

/// All text inside `<..:t>` elements of an XML fragment.
fn texts(xml: &str) -> String {
    let mut r = Reader::from_str(xml);
    let (mut in_t, mut out) = (false, String::new());
    while let Ok(ev) = r.read_event() {
        match ev {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_t = true,
            Event::End(e) if e.local_name().as_ref() == b"t" => {
                in_t = false;
                out.push(' ');
            }
            Event::Text(t) if in_t => out.push_str(&t.unescape().unwrap_or_default()),
            Event::Eof => break,
            _ => {}
        }
    }
    out.trim().to_string()
}

/// The text of one flat PresentationML shape (`<p:sp>` that holds `<p:cNvPr id="ID"`). Example
/// tooling: it relies on the generator's flat shapes and does not handle nested groups.
fn shape_text(slide: &str, id: &str) -> Option<String> {
    let at = slide.find(&format!("<p:cNvPr id=\"{id}\""))?;
    let start = slide[..at].rfind("<p:sp>")?;
    let end = at + slide[at..].find("</p:sp>")?;
    Some(texts(&slide[start..end]))
}

/// The cached value of `Sheet!Cell` in a workbook (numeric `<v>` or inline string).
fn cell_value(workbook: &Path, location: &str) -> Result<String> {
    let (sheet, cell) = location
        .split_once('!')
        .ok_or_else(|| anyhow!("location {location:?} is not Sheet!Cell"))?;
    let wb = read_part(workbook, "xl/workbook.xml")?;
    let rels = read_part(workbook, "xl/_rels/workbook.xml.rels")?;
    let rid = {
        let at = wb
            .find(&format!("name=\"{sheet}\""))
            .ok_or_else(|| anyhow!("no sheet {sheet}"))?;
        let tag = &wb[at..at + wb[at..].find("/>").unwrap_or(0)];
        let k = tag
            .find("r:id=\"")
            .ok_or_else(|| anyhow!("sheet {sheet} has no r:id"))?
            + 6;
        tag[k..k + tag[k..].find('"').unwrap_or(0)].to_string()
    };
    let target = {
        let at = rels
            .find(&format!("Id=\"{rid}\""))
            .ok_or_else(|| anyhow!("no relationship {rid}"))?;
        let from = rels[..at].rfind("<Relationship ").unwrap_or(0);
        let tag = &rels[from..at + rels[at..].find("/>").unwrap_or(0)];
        let k = tag.find("Target=\"").ok_or_else(|| anyhow!("no Target"))? + 8;
        tag[k..k + tag[k..].find('"').unwrap_or(0)].to_string()
    };
    let xml = read_part(workbook, &format!("xl/{target}"))?;
    let mut r = Reader::from_str(&xml);
    let (mut in_cell, mut grab, mut val) = (false, false, None::<String>);
    while let Ok(ev) = r.read_event() {
        match ev {
            Event::Start(e) if e.local_name().as_ref() == b"c" => {
                in_cell = e
                    .attributes()
                    .flatten()
                    .any(|a| a.key.as_ref() == b"r" && a.value.as_ref() == cell.as_bytes());
            }
            Event::End(e) if e.local_name().as_ref() == b"c" => in_cell = false,
            Event::Start(e) if in_cell && matches!(e.local_name().as_ref(), b"v" | b"t") => {
                grab = true
            }
            Event::End(e) if matches!(e.local_name().as_ref(), b"v" | b"t") => grab = false,
            Event::Text(t) if in_cell && grab => val = Some(t.unescape()?.into_owned()),
            Event::Eof => break,
            _ => {}
        }
    }
    val.ok_or_else(|| anyhow!("{location} has no value in {}", workbook.display()))
}

fn numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter_map(|t| t.parse::<f64>().ok())
        .collect()
}

fn check_links(deck: &Path, base: Option<&Path>) -> Result<Vec<serde_json::Value>> {
    let base = base
        .map(Path::to_path_buf)
        .unwrap_or_else(|| deck.parent().map(Path::to_path_buf).unwrap_or_default());
    let mut out = vec![];
    for v in ao::sources(deck, None, None)?.sources {
        let f = &v.record.fields;
        let (Some(uri), Some(loc), Some(_)) = (
            f.resource_uri.as_deref(),
            f.location.as_deref(),
            f.source_hash.as_deref(),
        ) else {
            continue;
        };
        if !uri.ends_with(".xlsx") || uri.contains('/') || uri.contains("..") {
            continue; // only workbooks next to the deck, by plain file name
        }
        let mut row = json!({"id": v.record.id, "workbook": uri, "location": loc, "effective_status": v.effective_status});
        let verdict = (|| -> Result<(String, serde_json::Value)> {
            let value = cell_value(&base.join(uri), loc)?;
            let claim = match (f.claim_part.as_deref(), f.claim_object_id.as_deref()) {
                (Some(p), Some(o)) => shape_text(&read_part(deck, p.trim_start_matches('/'))?, o)
                    .ok_or_else(|| anyhow!("shape {o} not found in {p}"))?,
                (Some(p), None) => texts(&read_part(deck, p.trim_start_matches('/'))?),
                _ => bail!("the source has no claim_part"),
            };
            let want: f64 = value
                .parse()
                .map_err(|_| anyhow!("workbook value {value:?} is not numeric"))?;
            let shows = numbers(&claim).iter().any(|n| (*n - want).abs() < 1e-9);
            let hash = value_hash(&value);
            let verdict = if !shows {
                "claim_differs_from_workbook"
            } else if f.source_hash.as_deref() != Some(hash.as_str())
                || v.effective_status != "verified"
            {
                "needs_reverification"
            } else {
                "consistent"
            };
            Ok((
                verdict.to_string(),
                json!({"workbook_value": value, "workbook_value_hash": hash, "claim_text": claim, "claim_shows_workbook_value": shows}),
            ))
        })();
        match verdict {
            Ok((verdict, extra)) => {
                row["verdict"] = verdict.into();
                if let (Some(r), Some(e)) = (row.as_object_mut(), extra.as_object()) {
                    r.extend(e.clone());
                }
            }
            Err(e) => {
                row["verdict"] = "unreadable".into();
                row["error"] = format!("{e:#}").into();
            }
        }
        out.push(row);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    static UNIQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "ao-gen-{}-{}",
            std::process::id(),
            UNIQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn generation_is_byte_reproducible() {
        let (a, b) = (tmp(), tmp());
        generate(&a).unwrap();
        generate(&b).unwrap();
        for f in ["board-deck.pptx", "financial-model.xlsx", "report.docx"] {
            assert_eq!(
                std::fs::read(a.join(f)).unwrap(),
                std::fs::read(b.join(f)).unwrap(),
                "{f} differs between runs"
            );
        }
    }

    #[test]
    fn every_example_round_trips_through_the_library_and_validates_clean() {
        let d = tmp();
        generate(&d).unwrap();
        for (f, fmt) in [
            ("board-deck.pptx", "pptx"),
            ("financial-model.xlsx", "xlsx"),
            ("report.docx", "docx"),
        ] {
            let p = d.join(f);
            let r = ao::validate(&p).unwrap();
            assert!(
                r.ok && r.warnings.is_empty(),
                "{f}: {:?} {:?}",
                r.errors,
                r.warnings
            );
            let l = ao::list(&p).unwrap();
            assert_eq!(l.format, fmt);
            let mut kinds: Vec<_> = l.parts.iter().map(|x| x.kind.as_str()).collect();
            kinds.sort();
            assert_eq!(kinds, ["agents", "design", "intent", "sources"], "{f}");
            assert!(l.parts.iter().all(|x| x.error.is_none()));
            for k in ["agents", "design", "intent"] {
                assert!(!ao::read(&p, k, None).unwrap().sections.is_empty());
            }
            // a library write then read gives back what was written (round trip)
            let patch: ao::DocPatch =
                serde_json::from_str(r#"{"sections":[{"id":"rt","text":"round trip"}]}"#).unwrap();
            ao::write(&p, "intent", &patch, false).unwrap();
            assert_eq!(
                ao::read(&p, "intent", Some("rt")).unwrap().sections[0].text,
                "round trip"
            );
            assert!(ao::validate(&p).unwrap().ok);
        }
        let i = ao::inspect(&d.join("board-deck.pptx")).unwrap();
        assert_eq!(i.units.len(), 4);
        assert_eq!(i.units[1].text.as_deref(), Some("Revenue by quarter"));
        let m = ao::inspect(&d.join("financial-model.xlsx")).unwrap();
        assert_eq!(
            m.units.iter().map(|u| u.id.as_str()).collect::<Vec<_>>(),
            ["Summary", "Quarterly"]
        );
        assert_eq!(m.units[1].formulas, Some(1));
        let w = ao::inspect(&d.join("report.docx")).unwrap();
        assert!(w.units.len() >= 4 && w.units[0].text.as_deref() == Some("Annual revenue report"));
    }

    #[test]
    fn the_figures_match_the_contract() {
        let sum: f64 = native::QUARTERS
            .iter()
            .map(|q| q.1.parse::<f64>().unwrap())
            .sum();
        assert!((sum - native::ANNUAL.parse::<f64>().unwrap()).abs() < 1e-9);
    }

    #[test]
    fn editing_q3_on_the_slide_makes_the_source_stale_and_the_link_check_catches_the_mismatch() {
        let d = tmp();
        generate(&d).unwrap();
        let deck = d.join("board-deck.pptx");
        assert!(check_links(&deck, None)
            .unwrap()
            .iter()
            .all(|l| l["verdict"] == "consistent"));
        set_text(
            &deck,
            "ppt/slides/slide2.xml",
            &["EUR 18.6M".into()],
            &["EUR 19.1M".into()],
        )
        .unwrap();
        let s = ao::sources(&deck, None, None).unwrap();
        let q3 = s.sources.iter().find(|v| v.record.id == "q3-kpi").unwrap();
        assert_eq!(
            (q3.effective_status.as_str(), q3.stale_reason),
            ("stale", Some("claim_changed"))
        );
        assert_eq!(
            s.sources
                .iter()
                .find(|v| v.record.id == "annual")
                .unwrap()
                .effective_status,
            "verified",
            "an unrelated claim stays fresh"
        );
        let links = check_links(&deck, None).unwrap();
        assert_eq!(
            links.iter().find(|l| l["id"] == "q3-kpi").unwrap()["verdict"],
            "claim_differs_from_workbook"
        );
        // a failed replacement changes nothing
        assert!(set_text(
            &deck,
            "ppt/slides/slide2.xml",
            &["no such text".into()],
            &["x".into()]
        )
        .is_err());
        // make the workbook agree, then re-verify through a generated patch
        let wb = d.join("financial-model.xlsx");
        for part in ["xl/worksheets/sheet1.xml", "xl/worksheets/sheet2.xml"] {
            set_text(&wb, part, &["<v>18.6</v>".into()], &["<v>19.1</v>".into()]).unwrap();
        }
        let links = check_links(&deck, None).unwrap();
        assert_eq!(
            links.iter().find(|l| l["id"] == "q3-kpi").unwrap()["verdict"],
            "needs_reverification"
        );
        let patch = serde_json::json!({"sources": [{"id": "q3-kpi", "status": "verified", "source_hash": links[0]["workbook_value_hash"], "source_hash_algorithm": HASH_ALGO}]});
        ao::write(
            &deck,
            "sources",
            &serde_json::from_value(patch).unwrap(),
            false,
        )
        .unwrap();
        assert!(check_links(&deck, None)
            .unwrap()
            .iter()
            .all(|l| l["verdict"] == "consistent"));
        assert!(ao::validate(&deck).unwrap().warnings.is_empty());
    }

    #[test]
    fn editing_the_workbook_makes_the_deck_source_stale_until_the_slide_is_updated_and_reverified()
    {
        let d = tmp();
        generate(&d).unwrap();
        let (deck, wb) = (d.join("board-deck.pptx"), d.join("financial-model.xlsx"));
        let q3 = |deck: &Path| {
            ao::sources(deck, None, None)
                .unwrap()
                .sources
                .into_iter()
                .find(|v| v.record.id == "q3-kpi")
                .unwrap()
        };
        let v = q3(&deck);
        assert_eq!(v.effective_status, "verified");
        assert_eq!(
            v.external_check.as_ref().unwrap().current_value.as_deref(),
            Some("18.6")
        );
        for part in ["xl/worksheets/sheet1.xml", "xl/worksheets/sheet2.xml"] {
            set_text(&wb, part, &["<v>18.6</v>".into()], &["<v>19.1</v>".into()]).unwrap();
        }
        let v = q3(&deck);
        assert_eq!(
            (v.effective_status.as_str(), v.stale_reason),
            ("stale", Some("external_claim_changed"))
        );
        // the agent updates the slide; the claim is still stale until it is re-verified
        set_text(
            &deck,
            "ppt/slides/slide2.xml",
            &["EUR 18.6M".into()],
            &["EUR 19.1M".into()],
        )
        .unwrap();
        assert_eq!(
            q3(&deck).stale_reasons,
            ["external_claim_changed", "claim_changed"]
        );
        let patch = serde_json::json!({"sources": [{"id": "q3-kpi", "status": "verified"}]});
        ao::write(
            &deck,
            "sources",
            &serde_json::from_value(patch).unwrap(),
            false,
        )
        .unwrap();
        let v = q3(&deck);
        assert_eq!(v.effective_status, "verified");
        assert_eq!(
            v.record.fields.external_claim.unwrap().claim_hash.unwrap(),
            value_hash("19.1")
        );
    }
}
