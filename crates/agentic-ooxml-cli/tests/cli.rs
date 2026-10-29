//! End-to-end tests of the `agentic-ooxml` binary: real processes, real files, JSON on stdout.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_agentic-ooxml"))
}

fn tmp() -> PathBuf {
    let n = format!("ao-cli-{}-{}", std::process::id(), uniq());
    let d = std::env::temp_dir().join(n);
    std::fs::create_dir_all(&d).unwrap();
    d
}

const SLIDE: &str = r#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="3" name="KPI"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>Q3 18.6</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

fn make_zip(file: &Path, entries: &[(&str, String)]) {
    let mut w = zip::ZipWriter::new(std::fs::File::create(file).unwrap());
    let o = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (n, b) in entries {
        w.start_file(*n, o).unwrap();
        w.write_all(b.as_bytes()).unwrap();
    }
    w.finish().unwrap();
}

fn pptx(file: &Path, slide: &str) {
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    make_zip(
        file,
        &[
            ("[Content_Types].xml", "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>".into()),
            ("_rels/.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"{rel}/officeDocument\" Target=\"ppt/presentation.xml\"/></Relationships>")),
            ("ppt/presentation.xml", "<p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId2\"/></p:sldIdLst></p:presentation>".into()),
            ("ppt/_rels/presentation.xml.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId2\" Type=\"{rel}/slide\" Target=\"slides/slide1.xml\"/></Relationships>")),
            ("ppt/slides/slide1.xml", slide.into()),
        ],
    );
}

fn run(args: &[&str]) -> Output {
    bin().args(args).output().unwrap()
}

fn json(o: &Output) -> serde_json::Value {
    serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}",
            String::from_utf8_lossy(&o.stdout)
        )
    })
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn help_lists_every_subcommand_and_states_the_limits() {
    let o = run(&["--help"]);
    assert!(o.status.success());
    let h = String::from_utf8_lossy(&o.stdout);
    for c in [
        "inspect",
        "validate",
        "diff",
        "list",
        "read",
        "write",
        "sources",
        "reconcile",
    ] {
        assert!(h.contains(c), "--help lacks {c}:\n{h}");
    }
    assert!(h.contains("untrusted"));
    assert!(h.contains("does not edit slides"));
    let v = run(&["validate", "--help"]);
    assert!(String::from_utf8_lossy(&v.stdout).contains("NOT a claim"));
}

#[test]
fn inspect_prints_json_with_units_and_trust() {
    let f = tmp().join("d.pptx");
    pptx(&f, SLIDE);
    let o = run(&["inspect", "--file", s(&f)]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let j = json(&o);
    assert_eq!(j["format"], "pptx");
    assert_eq!(j["trust"], "document");
    assert_eq!(j["unit_kind"], "slides");
    assert_eq!(j["units"][0]["text"], "Q3 18.6");
}

#[test]
fn validate_exits_zero_when_clean_and_one_on_errors() {
    let d = tmp();
    let good = d.join("good.pptx");
    pptx(&good, SLIDE);
    let o = run(&["validate", "--file", s(&good)]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stdout)
    );
    assert_eq!(json(&o)["ok"], true);

    let bad = d.join("bad.pptx");
    pptx(&bad, "<p:sld><p:cSld></p:sld>");
    let o = run(&["validate", "--file", s(&bad)]);
    assert_eq!(o.status.code(), Some(1));
    let j = json(&o);
    assert_eq!(j["ok"], false);
    assert_eq!(j["errors"][0]["code"], "xml.malformed");

    let junk = d.join("junk.docx");
    std::fs::write(&junk, b"nope").unwrap();
    let o = run(&["validate", "--file", s(&junk)]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(json(&o)["errors"][0]["code"], "zip.invalid");

    // a missing file is also exit 1, with the reason on stderr
    let o = run(&["validate", "--file", s(&d.join("absent.docx"))]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("open"));
}

#[test]
fn write_then_stale_then_reconcile_then_diff() {
    let d = tmp();
    let f = d.join("deck.pptx");
    pptx(&f, SLIDE);
    let patch = d.join("sources.json");
    std::fs::write(&patch, r#"{"sources":[{"id":"q3","status":"verified","claim_part":"ppt/slides/slide1.xml","claim_object_id":"3","resource_uri":"model.xlsx","location":"Summary!C4"}]}"#).unwrap();
    assert!(run(&[
        "write",
        "--file",
        s(&f),
        "--kind",
        "sources",
        "--input",
        s(&patch)
    ])
    .status
    .success());
    let fresh = json(&run(&["sources", "--file", s(&f)]));
    assert_eq!(fresh["sources"][0]["effective_status"], "verified");

    let before = d.join("before.pptx");
    std::fs::copy(&f, &before).unwrap();
    // `after` = the same package with the claimed number changed in the slide part
    let after = d.join("after.pptx");
    std::fs::copy(&f, &after).unwrap();
    let mut z = zip::ZipArchive::new(std::fs::File::open(&after).unwrap()).unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = vec![];
    for i in 0..z.len() {
        let mut e = z.by_index(i).unwrap();
        let mut b = vec![];
        std::io::Read::read_to_end(&mut e, &mut b).unwrap();
        let b = if e.name() == "ppt/slides/slide1.xml" {
            String::from_utf8(b)
                .unwrap()
                .replace("18.6", "19.1")
                .into_bytes()
        } else {
            b
        };
        entries.push((e.name().to_string(), b));
    }
    drop(z);
    let mut w = zip::ZipWriter::new(std::fs::File::create(&after).unwrap());
    for (n, b) in &entries {
        w.start_file(n.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(b).unwrap();
    }
    w.finish().unwrap();

    let stale = json(&run(&["sources", "--file", s(&after)]));
    assert_eq!(stale["sources"][0]["effective_status"], "stale");
    assert_eq!(stale["sources"][0]["stale_reason"], "claim_changed");
    // validate still exits 0: stale is a warning
    let v = run(&["validate", "--file", s(&after)]);
    assert_eq!(v.status.code(), Some(0));
    assert_eq!(json(&v)["warnings"][0]["code"], "sources.stale");

    let dr = json(&run(&["reconcile", "--file", s(&after), "--dry-run"]));
    assert_eq!(dr["written"], false);
    assert_eq!(dr["invalidated"][0]["id"], "q3");
    let rc = json(&run(&["reconcile", "--file", s(&after)]));
    assert_eq!(rc["written"], true);

    let df = json(&run(&[
        "diff",
        "--before",
        s(&before),
        "--after",
        s(&after),
    ]));
    assert_eq!(df["identical"], false);
    assert_eq!(
        df["native_parts"]["changed"][0]["part"],
        "ppt/slides/slide1.xml"
    );
    assert_eq!(df["sources"][0]["before"]["status"], "verified");
    assert_eq!(df["sources"][0]["after"]["status"], "stale");
    let same = json(&run(&[
        "diff",
        "--before",
        s(&before),
        "--after",
        s(&before),
    ]));
    assert_eq!(same["identical"], true);
}

#[test]
fn an_external_claim_is_recorded_checked_and_refused_when_it_escapes() {
    let d = tmp();
    let deck = d.join("deck.pptx");
    pptx(&deck, SLIDE);
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let sml = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let wb = |v: &str| {
        make_zip(
            &d.join("m.xlsx"),
            &[
                ("[Content_Types].xml", "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>".into()),
                ("_rels/.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"{rel}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>")),
                ("xl/workbook.xml", format!("<workbook xmlns=\"{sml}\" xmlns:r=\"{rel}\"><sheets><sheet name=\"Summary\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>")),
                ("xl/_rels/workbook.xml.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"{rel}/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>")),
                ("xl/worksheets/sheet1.xml", format!("<worksheet xmlns=\"{sml}\"><sheetData><row r=\"1\"><c r=\"B1\"><v>{v}</v></c></row></sheetData></worksheet>")),
            ],
        );
    };
    wb("18.6");
    let patch = d.join("p.json");
    std::fs::write(&patch, r#"{"sources":[{"id":"q3","status":"verified","claim_part":"ppt/slides/slide1.xml","claim_object_id":"3","external_claim":{"file":"m.xlsx","sheet":"Summary","cell":"B1"}}]}"#).unwrap();
    let o = run(&[
        "write",
        "--file",
        s(&deck),
        "--kind",
        "sources",
        "--input",
        s(&patch),
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v = json(&run(&["sources", "--file", s(&deck)]));
    assert_eq!(v["sources"][0]["effective_status"], "verified");
    wb("19.1");
    let v = json(&run(&["sources", "--file", s(&deck)]));
    assert_eq!(v["sources"][0]["stale_reason"], "external_claim_changed");
    let v = json(&run(&["reconcile", "--file", s(&deck), "--dry-run"]));
    assert_eq!(v["invalidated"][0]["reason"], "external_claim_changed");
    let o = run(&["validate", "--file", s(&deck)]);
    assert!(o.status.success(), "stale is a warning");
    // a path that leaves the directory is refused at write time
    std::fs::write(&patch, r#"{"sources":[{"id":"bad","external_claim":{"file":"../m.xlsx","sheet":"Summary","cell":"B1"}}]}"#).unwrap();
    let o = run(&[
        "write",
        "--file",
        s(&deck),
        "--kind",
        "sources",
        "--input",
        s(&patch),
    ]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("external claim"));
}

#[test]
fn read_and_list_report_an_error_for_a_missing_part() {
    let f = tmp().join("d.pptx");
    pptx(&f, SLIDE);
    let l = json(&run(&["list", "--file", s(&f)]));
    assert_eq!(l["parts"].as_array().unwrap().len(), 0);
    let o = run(&["read", "--file", s(&f), "--kind", "design"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("no 'design' knowledge part"));
}

/// A process-unique counter: pid plus nanoseconds is not unique across parallel test threads.
fn uniq() -> u64 {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
