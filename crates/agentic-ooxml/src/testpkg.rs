//! Test-only package builders: small but structurally complete .pptx/.docx/.xlsx files (content
//! types for every part, resolvable relationships, real shapes/cells/headings).

use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

pub(crate) fn tmp_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("ao-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

pub(crate) fn zip_to(file: &Path, entries: &[(&str, String)]) {
    let mut w = zip::ZipWriter::new(std::fs::File::create(file).unwrap());
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (n, b) in entries {
        w.start_file(*n, opts).unwrap();
        w.write_all(b.as_bytes()).unwrap();
    }
    w.finish().unwrap();
}

pub(crate) fn rels(items: &[(&str, &str, &str)]) -> String {
    let body: String = items
        .iter()
        .map(|(id, t, target)| {
            format!("<Relationship Id=\"{id}\" Type=\"{R}/{t}\" Target=\"{target}\"/>")
        })
        .collect();
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{body}</Relationships>")
}

pub(crate) fn content_types(overrides: &[(&str, &str)]) -> String {
    let o: String = overrides
        .iter()
        .map(|(p, c)| format!("<Override PartName=\"{p}\" ContentType=\"{c}\"/>"))
        .collect();
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{o}</Types>")
}

pub(crate) const SLIDE1: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>Board update</a:t></a:r></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Q3 KPI"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>Q3 revenue EUR 18.6M</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;

pub(crate) fn pptx(file: &Path) {
    let pml = "application/vnd.openxmlformats-officedocument.presentationml";
    zip_to(
        file,
        &[
            ("[Content_Types].xml", content_types(&[("/ppt/presentation.xml", &format!("{pml}.presentation.main+xml")), ("/ppt/slides/slide1.xml", &format!("{pml}.slide+xml")), ("/ppt/slides/slide2.xml", &format!("{pml}.slide+xml"))])),
            ("_rels/.rels", rels(&[("rId1", "officeDocument", "ppt/presentation.xml")])),
            ("ppt/presentation.xml", "<p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId2\"/><p:sldId id=\"257\" r:id=\"rId3\"/></p:sldIdLst></p:presentation>".into()),
            ("ppt/_rels/presentation.xml.rels", rels(&[("rId2", "slide", "slides/slide1.xml"), ("rId3", "slide", "slides/slide2.xml")])),
            ("ppt/slides/slide1.xml", SLIDE1.into()),
            ("ppt/slides/slide2.xml", SLIDE1.replace("Board update", "Appendix").replace("18.6", "17.9")),
        ],
    );
}

pub(crate) fn docx(file: &Path) {
    let wml = "application/vnd.openxmlformats-officedocument.wordprocessingml";
    zip_to(
        file,
        &[
            ("[Content_Types].xml", content_types(&[("/word/document.xml", &format!("{wml}.document.main+xml"))])),
            ("_rels/.rels", rels(&[("rId1", "officeDocument", "word/document.xml")])),
            ("word/document.xml", "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body><w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Summary</w:t></w:r></w:p><w:p><w:r><w:t>Body text.</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val=\"Heading2\"/></w:pPr><w:r><w:t>Risks</w:t></w:r></w:p></w:body></w:document>".into()),
        ],
    );
}

pub(crate) fn xlsx(file: &Path) {
    let sml = "application/vnd.openxmlformats-officedocument.spreadsheetml";
    zip_to(
        file,
        &[
            ("[Content_Types].xml", content_types(&[("/xl/workbook.xml", &format!("{sml}.sheet.main+xml")), ("/xl/worksheets/sheet1.xml", &format!("{sml}.worksheet+xml"))])),
            ("_rels/.rels", rels(&[("rId1", "officeDocument", "xl/workbook.xml")])),
            ("xl/workbook.xml", "<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Summary\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>".into()),
            ("xl/_rels/workbook.xml.rels", rels(&[("rId1", "worksheet", "worksheets/sheet1.xml")])),
            ("xl/worksheets/sheet1.xml", "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData><row r=\"1\"><c r=\"A1\"><v>17.2</v></c><c r=\"B1\"><v>17.9</v></c><c r=\"C1\"><f>A1+B1</f><v>35.1</v></c></row></sheetData></worksheet>".into()),
        ],
    );
}
