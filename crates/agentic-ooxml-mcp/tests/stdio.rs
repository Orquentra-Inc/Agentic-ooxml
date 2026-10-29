//! Drives the real `agentic-ooxml-mcp` binary over stdio, one JSON-RPC message per line.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Client {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    next: i64,
}

impl Client {
    fn start(roots: &Path, allow_write: bool) -> Client {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_agentic-ooxml-mcp"));
        cmd.env("AGENTIC_OOXML_ROOTS", roots)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if allow_write {
            cmd.env("AGENTIC_OOXML_ALLOW_WRITE", "1");
        } else {
            cmd.env_remove("AGENTIC_OOXML_ALLOW_WRITE");
        }
        let mut child = cmd.spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut c = Client {
            child,
            stdin,
            out,
            next: 1,
        };
        let r = c.rpc("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}));
        assert_eq!(r["result"]["serverInfo"]["name"], "agentic-ooxml-mcp");
        c.notify("notifications/initialized");
        c
    }

    fn send_raw(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn read(&mut self) -> Value {
        let mut l = String::new();
        self.out.read_line(&mut l).unwrap();
        serde_json::from_str(&l).unwrap_or_else(|e| panic!("not JSON ({e}): {l:?}"))
    }

    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send_raw(
            &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string(),
        );
        let r = self.read();
        assert_eq!(r["id"], id);
        r
    }

    fn notify(&mut self, method: &str) {
        self.send_raw(&json!({"jsonrpc": "2.0", "method": method}).to_string());
    }

    /// Returns (is_error, structured result).
    fn tool(&mut self, name: &str, args: Value) -> (bool, Value) {
        let r = self.rpc("tools/call", json!({"name": name, "arguments": args}));
        let res = &r["result"];
        assert!(res.is_object(), "{r}");
        let text: Value =
            serde_json::from_str(res["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(&text, &res["structuredContent"]);
        assert_eq!(
            text["trust"], "document",
            "every tool result carries trust: document: {text}"
        );
        (res["isError"].as_bool().unwrap(), text)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("ao-mcp-{}-{}", std::process::id(), uniq()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(&d).unwrap()
}

const SLIDE: &str = r#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="3" name="KPI"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>Q3 18.6</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

fn pptx(file: &Path) {
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let mut w = zip::ZipWriter::new(std::fs::File::create(file).unwrap());
    let o = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (n, b) in [
        ("[Content_Types].xml", "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>".to_string()),
        ("_rels/.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"{rel}/officeDocument\" Target=\"ppt/presentation.xml\"/></Relationships>")),
        ("ppt/presentation.xml", "<p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId2\"/></p:sldIdLst></p:presentation>".to_string()),
        ("ppt/_rels/presentation.xml.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId2\" Type=\"{rel}/slide\" Target=\"slides/slide1.xml\"/></Relationships>")),
        ("ppt/slides/slide1.xml", SLIDE.to_string()),
    ] {
        w.start_file(n, o).unwrap();
        w.write_all(b.as_bytes()).unwrap();
    }
    w.finish().unwrap();
}

fn code(v: &Value) -> &str {
    v["error"]["code"].as_str().unwrap_or("")
}

#[test]
fn protocol_handshake_listing_and_errors() {
    let root = tmp();
    let mut c = Client::start(&root, false);
    let tools = c.rpc("tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "ao_inspect",
            "ao_list",
            "ao_read",
            "ao_sources",
            "ao_validate",
            "ao_diff",
            "ao_write",
            "ao_reconcile"
        ]
    );
    for t in tools["result"]["tools"].as_array().unwrap() {
        assert_eq!(t["inputSchema"]["type"], "object");
        let read_only =
            !t["name"].as_str().unwrap().starts_with("ao_write") && t["name"] != "ao_reconcile";
        assert_eq!(t["annotations"]["readOnlyHint"], read_only, "{}", t["name"]);
    }
    assert_eq!(c.rpc("ping", json!({}))["result"], json!({}));
    assert_eq!(c.rpc("nope/method", json!({}))["error"]["code"], -32601);
    assert_eq!(
        c.rpc("tools/call", json!({"name": "ao_missing"}))["error"]["code"],
        -32602
    );
    assert_eq!(
        c.rpc("tools/call", json!({"arguments": {}}))["error"]["code"],
        -32602
    );
    c.send_raw("{ not json");
    assert_eq!(c.read()["error"]["code"], -32700);
    c.send_raw("[1,2]");
    assert_eq!(c.read()["error"]["code"], -32600);
    // an unknown protocol version is answered with the latest the server speaks
    let r = c.rpc("initialize", json!({"protocolVersion": "1999-01-01"}));
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
    // parameter validation is a tool error, not a protocol error
    let (err, v) = c.tool("ao_list", json!({}));
    assert!(err);
    assert_eq!(code(&v), "invalid_params");
    let (err, v) = c.tool("ao_list", json!({"file": 7}));
    assert!(err);
    assert_eq!(code(&v), "invalid_params");
}

#[test]
fn read_tools_work_and_mark_results_as_document_data() {
    let root = tmp();
    let f = root.join("deck.pptx");
    pptx(&f);
    let p = f.to_str().unwrap();
    let mut c = Client::start(&root, false);

    let (err, v) = c.tool("ao_inspect", json!({"file": p}));
    assert!(!err, "{v}");
    assert_eq!(v["format"], "pptx");
    assert_eq!(v["units"][0]["text"], "Q3 18.6");
    let (err, v) = c.tool("ao_list", json!({"file": p}));
    assert!(!err);
    assert_eq!(v["parts"], json!([]));
    let (err, v) = c.tool("ao_validate", json!({"file": p}));
    assert!(!err);
    assert_eq!(v["ok"], true);
    let (err, v) = c.tool("ao_sources", json!({"file": p}));
    assert!(!err);
    assert_eq!(v["sources"], json!([]));
    let (err, v) = c.tool("ao_read", json!({"file": p, "kind": "design"}));
    assert!(err);
    assert_eq!(code(&v), "library_error");
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .contains("no 'design' knowledge part"));
    let (err, v) = c.tool("ao_diff", json!({"before": p, "after": p}));
    assert!(!err);
    assert_eq!(v["identical"], true);
}

#[test]
fn paths_outside_the_roots_are_refused_for_every_tool() {
    let root = tmp();
    let outside = tmp();
    let f_out = outside.join("secret.pptx");
    pptx(&f_out);
    let f_in = root.join("deck.pptx");
    pptx(&f_in);
    let mut c = Client::start(&root, true);

    // absolute path outside, traversal out of the root, and a missing file outside
    let traversal = format!(
        "{}/../{}/secret.pptx",
        root.display(),
        outside.file_name().unwrap().to_str().unwrap()
    );
    for bad in [
        f_out.to_str().unwrap().to_string(),
        traversal,
        outside.join("missing.pptx").display().to_string(),
    ] {
        for tool in ["ao_inspect", "ao_list", "ao_validate", "ao_sources"] {
            let (err, v) = c.tool(tool, json!({"file": bad}));
            assert!(err && code(&v) == "path_outside_roots", "{tool} {bad}: {v}");
        }
        let (_, v) = c.tool("ao_read", json!({"file": bad, "kind": "design"}));
        assert_eq!(code(&v), "path_outside_roots");
        let (_, v) = c.tool(
            "ao_diff",
            json!({"before": f_in.to_str().unwrap(), "after": bad}),
        );
        assert_eq!(code(&v), "path_outside_roots");
        let (_, v) = c.tool("ao_write", json!({"file": bad, "kind": "design", "patch": {"sections": [{"id": "x", "text": "y"}]}}));
        assert_eq!(code(&v), "path_outside_roots");
        let (_, v) = c.tool("ao_reconcile", json!({"file": bad}));
        assert_eq!(code(&v), "path_outside_roots");
    }
    // the outside file is untouched, and no proposal appeared next to it
    assert!(!outside.join("secret.ao-proposed.pptx").exists());
}

#[cfg(unix)]
#[test]
fn a_symlink_that_points_outside_the_roots_is_refused() {
    let root = tmp();
    let outside = tmp();
    pptx(&outside.join("target.pptx"));
    std::os::unix::fs::symlink(outside.join("target.pptx"), root.join("link.pptx")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("dirlink")).unwrap();
    let mut c = Client::start(&root, true);
    let (err, v) = c.tool(
        "ao_inspect",
        json!({"file": root.join("link.pptx").to_str().unwrap()}),
    );
    assert!(err && code(&v) == "path_outside_roots", "{v}");
    let (_, v) = c.tool(
        "ao_inspect",
        json!({"file": root.join("dirlink/target.pptx").to_str().unwrap()}),
    );
    assert_eq!(code(&v), "path_outside_roots");
    let (_, v) = c.tool("ao_write", json!({"file": root.join("link.pptx").to_str().unwrap(), "kind": "design", "patch": {"sections": [{"id": "x", "text": "y"}]}, "in_place": true}));
    assert_eq!(code(&v), "path_outside_roots");
    // the target is unchanged (still a valid, AO-free package)
    let (_, v) = {
        let mut c2 = Client::start(&outside, false);
        c2.tool(
            "ao_list",
            json!({"file": outside.join("target.pptx").to_str().unwrap()}),
        )
    };
    assert_eq!(v["parts"], json!([]));
}

#[test]
fn unsupported_files_and_directories_are_refused() {
    let root = tmp();
    std::fs::write(root.join("notes.txt"), b"hi").unwrap();
    std::fs::create_dir(root.join("folder.docx")).unwrap();
    let mut c = Client::start(&root, false);
    let (_, v) = c.tool(
        "ao_inspect",
        json!({"file": root.join("notes.txt").to_str().unwrap()}),
    );
    assert_eq!(code(&v), "unsupported_extension");
    let (_, v) = c.tool(
        "ao_inspect",
        json!({"file": root.join("folder.docx").to_str().unwrap()}),
    );
    assert_eq!(code(&v), "not_a_file");
    let (_, v) = c.tool(
        "ao_inspect",
        json!({"file": root.join("absent.docx").to_str().unwrap()}),
    );
    assert_eq!(code(&v), "path_not_found");
}

#[test]
fn write_tools_refuse_unless_enabled_and_change_nothing() {
    let root = tmp();
    let f = root.join("deck.pptx");
    pptx(&f);
    let before = std::fs::read(&f).unwrap();
    let mut c = Client::start(&root, false);
    let patch = json!({"sections": [{"id": "type", "text": "Inter"}]});
    let (err, v) = c.tool(
        "ao_write",
        json!({"file": f.to_str().unwrap(), "kind": "design", "patch": patch.clone()}),
    );
    assert!(err);
    assert_eq!(code(&v), "write_disabled");
    let (_, v) = c.tool(
        "ao_write",
        json!({"file": f.to_str().unwrap(), "kind": "design", "patch": patch, "in_place": true}),
    );
    assert_eq!(code(&v), "write_disabled");
    let (_, v) = c.tool(
        "ao_reconcile",
        json!({"file": f.to_str().unwrap(), "dry_run": true}),
    );
    assert_eq!(code(&v), "write_disabled");
    assert_eq!(std::fs::read(&f).unwrap(), before);
    assert_eq!(
        std::fs::read_dir(&root).unwrap().count(),
        1,
        "no proposal or temp file was created"
    );
}

#[test]
fn writes_go_to_a_proposed_copy_by_default_and_in_place_only_when_asked() {
    let root = tmp();
    let f = root.join("deck.pptx");
    pptx(&f);
    let original = std::fs::read(&f).unwrap();
    let p = f.to_str().unwrap();
    let proposed = root.join("deck.ao-proposed.pptx");
    let mut c = Client::start(&root, true);

    let patch = json!({"sections": [{"id": "type", "text": "Inter, 12pt"}]});
    let (err, v) = c.tool(
        "ao_write",
        json!({"file": p, "kind": "design", "patch": patch}),
    );
    assert!(!err, "{v}");
    assert_eq!(v["mode"], "proposed");
    assert_eq!(v["output"], proposed.to_str().unwrap());
    assert_eq!(v["result"]["created"], true);
    assert_eq!(
        std::fs::read(&f).unwrap(),
        original,
        "the original must be untouched"
    );
    assert!(proposed.exists());
    let (_, v) = c.tool("ao_list", json!({"file": p}));
    assert_eq!(v["parts"], json!([]));
    let (_, v) = c.tool(
        "ao_read",
        json!({"file": proposed.to_str().unwrap(), "kind": "design"}),
    );
    assert_eq!(v["sections"][0]["text"], "Inter, 12pt");
    assert_eq!(v["trust"], "document");
    let (_, v) = c.tool("ao_validate", json!({"file": proposed.to_str().unwrap()}));
    assert_eq!(v["ok"], true);

    // a second write whose input is the proposal edits that proposal in place (edits accumulate)
    let (err, v) = c.tool("ao_write", json!({"file": proposed.to_str().unwrap(), "kind": "intent", "patch": {"sections": [{"id": "goal", "text": "Inform the board"}]}}));
    assert!(!err, "{v}");
    assert_eq!(v["output"], proposed.to_str().unwrap());
    let (_, v) = c.tool("ao_list", json!({"file": proposed.to_str().unwrap()}));
    assert_eq!(v["parts"].as_array().unwrap().len(), 2);

    // a failing write leaves no proposal behind and does not disturb an existing one
    let before = std::fs::read(&proposed).unwrap();
    let (err, v) = c.tool("ao_write", json!({"file": p, "kind": "bogus", "patch": {}}));
    assert!(err);
    assert_eq!(code(&v), "library_error");
    assert_eq!(std::fs::read(&proposed).unwrap(), before);
    assert!(std::fs::read_dir(&root).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".tmp")));

    // in_place edits the file itself
    let (err, v) = c.tool("ao_write", json!({"file": p, "kind": "agents", "patch": {"sections": [{"id": "policy", "text": "Ask first."}]}, "in_place": true}));
    assert!(!err, "{v}");
    assert_eq!(v["mode"], "in_place");
    assert_ne!(std::fs::read(&f).unwrap(), original);
    let (_, v) = c.tool("ao_list", json!({"file": p}));
    assert_eq!(v["parts"][0]["kind"], "agents");
}

#[test]
fn a_changed_claim_goes_stale_and_reconcile_writes_a_proposal() {
    let root = tmp();
    let f = root.join("deck.pptx");
    pptx(&f);
    let p = f.to_str().unwrap();
    let mut c = Client::start(&root, true);
    let claim = json!({"sources": [{"id": "q3", "status": "verified", "claim_part": "ppt/slides/slide1.xml", "claim_object_id": "3", "resource_uri": "financial-model.xlsx", "location": "Summary!C4"}]});
    let (err, v) = c.tool(
        "ao_write",
        json!({"file": p, "kind": "sources", "patch": claim, "in_place": true}),
    );
    assert!(!err, "{v}");
    let (_, v) = c.tool("ao_sources", json!({"file": p}));
    assert_eq!(v["sources"][0]["effective_status"], "verified");

    // someone edits the slide outside the server (here: rewrite the file with the number changed)
    let edited = root.join("edited.pptx");
    {
        use std::io::Read;
        let mut z = zip::ZipArchive::new(std::fs::File::open(&f).unwrap()).unwrap();
        let mut w = zip::ZipWriter::new(std::fs::File::create(&edited).unwrap());
        for i in 0..z.len() {
            let mut e = z.by_index(i).unwrap();
            let mut b = vec![];
            e.read_to_end(&mut b).unwrap();
            if e.name() == "ppt/slides/slide1.xml" {
                b = String::from_utf8(b)
                    .unwrap()
                    .replace("18.6", "19.1")
                    .into_bytes();
            }
            w.start_file(e.name(), zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(&b).unwrap();
        }
        w.finish().unwrap();
    }
    std::fs::rename(&edited, &f).unwrap();

    let (_, v) = c.tool("ao_sources", json!({"file": p}));
    assert_eq!(v["sources"][0]["effective_status"], "stale");
    assert_eq!(v["sources"][0]["stale_reason"], "claim_changed");
    let (_, v) = c.tool("ao_validate", json!({"file": p}));
    assert_eq!(v["ok"], true);
    assert_eq!(v["warnings"][0]["code"], "sources.stale");

    let (_, v) = c.tool("ao_reconcile", json!({"file": p, "dry_run": true}));
    assert_eq!(v["written"], false);
    assert_eq!(v["invalidated"][0]["id"], "q3");
    assert!(
        !root.join("deck.ao-proposed.pptx").exists(),
        "a dry run writes nothing"
    );

    let (err, v) = c.tool("ao_reconcile", json!({"file": p}));
    assert!(!err, "{v}");
    assert_eq!(v["mode"], "proposed");
    assert_eq!(v["result"]["written"], true);
    let proposed = root.join("deck.ao-proposed.pptx");
    let (_, v) = c.tool(
        "ao_diff",
        json!({"before": p, "after": proposed.to_str().unwrap()}),
    );
    assert_eq!(v["sources"][0]["before"]["status"], "verified");
    assert_eq!(v["sources"][0]["after"]["status"], "stale");
}

#[test]
fn a_notification_gets_no_response_and_requests_still_answer_in_order() {
    let root = tmp();
    let mut c = Client::start(&root, false);
    c.notify("notifications/initialized");
    c.notify("notifications/cancelled");
    assert_eq!(c.rpc("ping", json!({}))["result"], json!({}));
}

/// A process-unique counter: pid plus nanoseconds is not unique across parallel test threads.
fn uniq() -> u64 {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

fn xlsx(file: &Path, b1: &str) {
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let sml = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let mut w = zip::ZipWriter::new(std::fs::File::create(file).unwrap());
    let o = zip::write::SimpleFileOptions::default();
    for (n, b) in [
        ("[Content_Types].xml", "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>".to_string()),
        ("_rels/.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"{rel}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>")),
        ("xl/workbook.xml", format!("<workbook xmlns=\"{sml}\" xmlns:r=\"{rel}\"><sheets><sheet name=\"Summary\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>")),
        ("xl/_rels/workbook.xml.rels", format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"{rel}/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>")),
        ("xl/worksheets/sheet1.xml", format!("<worksheet xmlns=\"{sml}\"><sheetData><row r=\"1\"><c r=\"B1\"><v>{b1}</v></c></row></sheetData></worksheet>")),
    ] {
        w.start_file(n, o).unwrap();
        w.write_all(b.as_bytes()).unwrap();
    }
    w.finish().unwrap();
}

#[test]
fn an_external_claim_goes_stale_through_the_tools_and_can_be_rebaselined() {
    let root = tmp();
    let f = root.join("deck.pptx");
    pptx(&f);
    xlsx(&root.join("model.xlsx"), "18.6");
    let p = f.to_str().unwrap();
    let mut c = Client::start(&root, true);
    let claim = json!({"sources": [{"id": "q3", "status": "verified", "claim_part": "ppt/slides/slide1.xml", "claim_object_id": "3",
        "external_claim": {"file": "model.xlsx", "sheet": "Summary", "cell": "B1"}}]});
    let (err, v) = c.tool(
        "ao_write",
        json!({"file": p, "kind": "sources", "patch": claim, "in_place": true}),
    );
    assert!(!err, "{v}");
    let (_, v) = c.tool("ao_sources", json!({"file": p}));
    assert_eq!(v["sources"][0]["effective_status"], "verified");
    assert_eq!(v["sources"][0]["external_check"]["current_value"], "18.6");

    xlsx(&root.join("model.xlsx"), "19.1");
    let (_, v) = c.tool("ao_sources", json!({"file": p}));
    assert_eq!(v["sources"][0]["stale_reason"], "external_claim_changed");
    let (_, v) = c.tool("ao_validate", json!({"file": p}));
    assert!(
        v["warnings"][0]["message"]
            .as_str()
            .unwrap()
            .contains("external_claim_changed"),
        "{v}"
    );
    let (_, v) = c.tool("ao_reconcile", json!({"file": p, "dry_run": true}));
    assert_eq!(v["invalidated"][0]["reason"], "external_claim_changed");

    let (err, v) = c.tool("ao_write", json!({"file": p, "kind": "sources", "patch": {"sources": [{"id": "q3", "status": "verified"}]}, "in_place": true}));
    assert!(!err, "{v}");
    let (_, v) = c.tool("ao_sources", json!({"file": p}));
    assert_eq!(v["sources"][0]["effective_status"], "verified");
}

#[cfg(unix)]
#[test]
fn an_external_claim_outside_the_roots_is_reported_and_never_opened() {
    let base = tmp();
    let root = base.join("roots");
    let outside = base.join("outside");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let f = root.join("deck.pptx");
    pptx(&f);
    xlsx(&root.join("model.xlsx"), "18.6");
    let p = f.to_str().unwrap();
    let mut c = Client::start(&root, true);
    let claim = json!({"sources": [{"id": "q3", "status": "verified", "claim_part": "ppt/slides/slide1.xml",
        "external_claim": {"file": "model.xlsx", "sheet": "Summary", "cell": "B1"}}]});
    let (err, v) = c.tool(
        "ao_write",
        json!({"file": p, "kind": "sources", "patch": claim, "in_place": true}),
    );
    assert!(!err, "{v}");
    // swap the workbook for a symlink to a different, valid workbook outside the roots
    xlsx(&outside.join("real.xlsx"), "99");
    std::fs::remove_file(root.join("model.xlsx")).unwrap();
    std::os::unix::fs::symlink(outside.join("real.xlsx"), root.join("model.xlsx")).unwrap();
    for tool in ["ao_sources", "ao_validate"] {
        let (_, v) = c.tool(tool, json!({"file": p}));
        let s = v.to_string();
        assert!(s.contains("external_outside_roots"), "{tool}: {s}");
        assert!(
            !s.contains("external_claim_changed") && !s.contains("\"99\""),
            "{tool} must not have opened it: {s}"
        );
    }
    let (_, v) = c.tool("ao_sources", json!({"file": p}));
    assert_eq!(v["sources"][0]["stale_reason"], "external_outside_roots");
    assert!(v["sources"][0]["external_check"]["current_value"].is_null());
    // re-baselining refuses too
    let (err, v) = c.tool("ao_write", json!({"file": p, "kind": "sources", "patch": {"sources": [{"id": "q3", "status": "verified"}]}, "in_place": true}));
    assert!(err, "{v}");
    assert!(v.to_string().contains("external_outside_roots"), "{v}");
    // and an absolute path or traversal cannot even be recorded
    for bad in [
        "../outside/real.xlsx",
        outside.join("real.xlsx").to_str().unwrap(),
    ] {
        let patch = json!({"sources": [{"id": "z", "external_claim": {"file": bad, "sheet": "Summary", "cell": "B1"}}]});
        let (err, _) = c.tool(
            "ao_write",
            json!({"file": p, "kind": "sources", "patch": patch, "in_place": true}),
        );
        assert!(err, "{bad}");
    }
}
