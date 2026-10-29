//! Builders for the native (non-AO) content of the three synthetic examples. Every figure is
//! synthetic (EUR M): Q1 17.2, Q2 17.9, Q3 18.6, Q4 forecast 21.3, annual 75.0.
//!
//! These are small, hand-written OOXML packages. They are checked for structure by this repo's
//! `validate` and by a Python zipfile+XML pass (scripts/check-examples.py). They have NOT been
//! opened in Microsoft Office, so nothing here claims Office renders them.

pub const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const NS_P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL_PKG: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CT_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

pub type Entries = Vec<(String, String)>;

pub const QUARTERS: [(&str, &str, &str); 4] = [
    ("Q1", "17.2", "actual"),
    ("Q2", "17.9", "actual"),
    ("Q3", "18.6", "actual"),
    ("Q4", "21.3", "forecast"),
];
pub const ANNUAL: &str = "75.0";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn rels(items: &[(&str, &str, &str)]) -> String {
    let body: String = items
        .iter()
        .map(|(id, t, target)| {
            format!("<Relationship Id=\"{id}\" Type=\"{NS_R}/{t}\" Target=\"{target}\"/>")
        })
        .collect();
    format!("{XML}<Relationships xmlns=\"{REL_PKG}\">{body}</Relationships>")
}

fn content_types(overrides: &[(&str, &str)]) -> String {
    let o: String = overrides
        .iter()
        .map(|(p, c)| format!("<Override PartName=\"{p}\" ContentType=\"{c}\"/>"))
        .collect();
    format!("{XML}<Types xmlns=\"{CT_NS}\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{o}</Types>")
}

fn core(title: &str) -> String {
    format!("{XML}<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:title>{}</dc:title><dc:creator>Agentic OOXML example generator</dc:creator><dc:description>Synthetic example data. Not real financials.</dc:description><dcterms:created xsi:type=\"dcterms:W3CDTF\">2026-10-29T00:00:00Z</dcterms:created></cp:coreProperties>", esc(title))
}

const CT_CORE: &str = "application/vnd.openxmlformats-package.core-properties+xml";

// ---------------------------------------------------------------------------- pptx

const THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Graphite"><a:themeElements><a:clrScheme name="Graphite"><a:dk1><a:srgbClr val="0B0D10"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1C2026"/></a:dk2><a:lt2><a:srgbClr val="E6E8EB"/></a:lt2><a:accent1><a:srgbClr val="2457F5"/></a:accent1><a:accent2><a:srgbClr val="5B6573"/></a:accent2><a:accent3><a:srgbClr val="8A93A0"/></a:accent3><a:accent4><a:srgbClr val="3D7BFF"/></a:accent4><a:accent5><a:srgbClr val="1A3FB8"/></a:accent5><a:accent6><a:srgbClr val="B8BEC7"/></a:accent6><a:hlink><a:srgbClr val="2457F5"/></a:hlink><a:folHlink><a:srgbClr val="5B6573"/></a:folHlink></a:clrScheme><a:fontScheme name="Graphite"><a:majorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Graphite"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#;

fn sp_tree(shapes: &str) -> String {
    format!("<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>{shapes}</p:spTree>")
}

fn title_ph(id: u32, text: &str) -> String {
    format!("<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Title\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-GB\"/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>", esc(text))
}

/// A text box. Positions are EMU.
#[allow(clippy::too_many_arguments)]
fn textbox(id: u32, name: &str, x: i64, y: i64, w: i64, h: i64, size: u32, text: &str) -> String {
    format!("<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap=\"square\"><a:spAutoFit/></a:bodyPr><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-GB\" sz=\"{size}\"/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>", esc(name), esc(text))
}

fn rect(id: u32, name: &str, x: i64, y: i64, w: i64, h: i64, color: &str) -> String {
    format!("<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{}\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>", esc(name))
}

fn slide(shapes: &str) -> String {
    format!("{XML}<p:sld xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\" xmlns:p=\"{NS_P}\"><p:cSld>{}</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>", sp_tree(shapes))
}

/// Slide part names, in order, for the board deck. Slide 2 carries the Q3 KPI (shape id 5) and
/// slide 3 the annual figure (shape id 3): the shapes the sources claims point at.
pub const Q3_SHAPE_ID: &str = "5";
pub const ANNUAL_SHAPE_ID: &str = "3";

pub fn board_deck() -> Entries {
    let pml = "application/vnd.openxmlformats-officedocument.presentationml";
    let s1 = slide(&format!(
        "{}{}",
        title_ph(2, "FY Board Update"),
        textbox(
            3,
            "Subtitle",
            838_200,
            3_000_000,
            10_000_000,
            600_000,
            2000,
            "Synthetic example data (EUR M). Not real financials."
        )
    ));
    // Slide 2: four quarter figures plus four bars scaled to the values.
    let mut s2 = title_ph(2, "Revenue by quarter");
    let labels = [
        ("Q1 revenue EUR 17.2M", 17.2f64),
        ("Q2 revenue EUR 17.9M", 17.9),
        ("Q3 revenue EUR 18.6M", 18.6),
        ("Q4 forecast EUR 21.3M", 21.3),
    ];
    for (i, (text, v)) in labels.iter().enumerate() {
        let y = 1_700_000 + (i as i64) * 650_000;
        s2.push_str(&textbox(
            3 + i as u32,
            &format!("{} KPI", &text[..2]),
            838_200,
            y,
            4_800_000,
            500_000,
            2000,
            text,
        ));
        let bar_w = (*v / 21.3 * 5_000_000.0) as i64;
        s2.push_str(&rect(
            7 + i as u32,
            &format!("{} bar", &text[..2]),
            5_900_000,
            y + 60_000,
            bar_w,
            380_000,
            if i == 3 { "8A93A0" } else { "2457F5" },
        ));
    }
    let s2 = slide(&s2);
    let s3 = slide(&format!(
        "{}{}{}",
        title_ph(2, "Full-year outlook"),
        textbox(
            3,
            "Annual KPI",
            838_200,
            1_900_000,
            10_000_000,
            700_000,
            3200,
            &format!("FY revenue EUR {ANNUAL}M")
        ),
        textbox(
            4,
            "Basis",
            838_200,
            2_900_000,
            10_000_000,
            900_000,
            1800,
            "Q1 to Q3 actual plus Q4 forecast of EUR 21.3M."
        )
    ));
    let s4 = slide(&format!(
        "{}{}",
        title_ph(2, "Risks and asks"),
        textbox(
            3,
            "Risks",
            838_200,
            1_800_000,
            10_000_000,
            2_000_000,
            2000,
            "Q4 forecast depends on two renewals; board asked to confirm the hiring plan."
        )
    ));

    let master = format!("{XML}<p:sldMaster xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\" xmlns:p=\"{NS_P}\"><p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef></p:bg>{}</p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/><p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle><a:lvl1pPr algn=\"l\"><a:defRPr sz=\"4000\" b=\"1\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mj-lt\"/></a:defRPr></a:lvl1pPr></p:titleStyle><p:bodyStyle><a:lvl1pPr><a:defRPr sz=\"2000\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/></a:defRPr></a:lvl1pPr></p:bodyStyle><p:otherStyle><a:lvl1pPr><a:defRPr sz=\"1800\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/></a:defRPr></a:lvl1pPr></p:otherStyle></p:txStyles></p:sldMaster>",
        sp_tree("<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title Placeholder\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"838200\" y=\"365125\"/><a:ext cx=\"10515600\" cy=\"1100000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr anchor=\"ctr\"/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-GB\"/><a:t>Title</a:t></a:r></a:p></p:txBody></p:sp>"));
    let layout = format!("{XML}<p:sldLayout xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\" xmlns:p=\"{NS_P}\" type=\"titleOnly\" preserve=\"1\"><p:cSld name=\"Title Only\">{}</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>",
        sp_tree("<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-GB\"/><a:t>Title</a:t></a:r></a:p></p:txBody></p:sp>"));
    let pres = format!("{XML}<p:presentation xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\" xmlns:p=\"{NS_P}\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId2\"/><p:sldId id=\"257\" r:id=\"rId3\"/><p:sldId id=\"258\" r:id=\"rId4\"/><p:sldId id=\"259\" r:id=\"rId5\"/></p:sldIdLst><p:sldSz cx=\"12192000\" cy=\"6858000\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>");
    vec![
        ("[Content_Types].xml".into(), content_types(&[
            ("/ppt/presentation.xml", &format!("{pml}.presentation.main+xml")),
            ("/ppt/slideMasters/slideMaster1.xml", &format!("{pml}.slideMaster+xml")),
            ("/ppt/slideLayouts/slideLayout1.xml", &format!("{pml}.slideLayout+xml")),
            ("/ppt/theme/theme1.xml", "application/vnd.openxmlformats-officedocument.theme+xml"),
            ("/ppt/slides/slide1.xml", &format!("{pml}.slide+xml")), ("/ppt/slides/slide2.xml", &format!("{pml}.slide+xml")),
            ("/ppt/slides/slide3.xml", &format!("{pml}.slide+xml")), ("/ppt/slides/slide4.xml", &format!("{pml}.slide+xml")),
            ("/docProps/core.xml", CT_CORE),
        ])),
        ("_rels/.rels".into(), format!("{XML}<Relationships xmlns=\"{REL_PKG}\"><Relationship Id=\"rId1\" Type=\"{NS_R}/officeDocument\" Target=\"ppt/presentation.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>")),
        ("docProps/core.xml".into(), core("FY Board Update (synthetic)")),
        ("ppt/presentation.xml".into(), pres),
        ("ppt/_rels/presentation.xml.rels".into(), rels(&[("rId1", "slideMaster", "slideMasters/slideMaster1.xml"), ("rId2", "slide", "slides/slide1.xml"), ("rId3", "slide", "slides/slide2.xml"), ("rId4", "slide", "slides/slide3.xml"), ("rId5", "slide", "slides/slide4.xml"), ("rId6", "theme", "theme/theme1.xml")])),
        ("ppt/slideMasters/slideMaster1.xml".into(), master),
        ("ppt/slideMasters/_rels/slideMaster1.xml.rels".into(), rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml"), ("rId2", "theme", "../theme/theme1.xml")])),
        ("ppt/slideLayouts/slideLayout1.xml".into(), layout),
        ("ppt/slideLayouts/_rels/slideLayout1.xml.rels".into(), rels(&[("rId1", "slideMaster", "../slideMasters/slideMaster1.xml")])),
        ("ppt/theme/theme1.xml".into(), THEME.to_string()),
        ("ppt/slides/slide1.xml".into(), s1), ("ppt/slides/slide2.xml".into(), s2), ("ppt/slides/slide3.xml".into(), s3), ("ppt/slides/slide4.xml".into(), s4),
        ("ppt/slides/_rels/slide1.xml.rels".into(), rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml")])),
        ("ppt/slides/_rels/slide2.xml.rels".into(), rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml")])),
        ("ppt/slides/_rels/slide3.xml.rels".into(), rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml")])),
        ("ppt/slides/_rels/slide4.xml.rels".into(), rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml")])),
    ]
}

// ---------------------------------------------------------------------------- xlsx

fn istr(r: &str, text: &str) -> String {
    format!(
        "<c r=\"{r}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
        esc(text)
    )
}
fn num(r: &str, v: &str) -> String {
    format!("<c r=\"{r}\"><v>{v}</v></c>")
}
fn formula(r: &str, f: &str, cached: &str) -> String {
    format!("<c r=\"{r}\"><f>{}</f><v>{cached}</v></c>", esc(f))
}
fn sheet(rows: &[Vec<String>]) -> String {
    let body: String = rows
        .iter()
        .enumerate()
        .map(|(i, cells)| format!("<row r=\"{}\">{}</row>", i + 1, cells.concat()))
        .collect();
    format!("{XML}<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>{body}</sheetData></worksheet>")
}

/// Summary!B1 is the Q3 figure the deck quotes (a formula over Quarterly!B4); Summary!B2 is the annual total.
pub const Q3_CELL: &str = "Summary!B1";
pub const ANNUAL_CELL: &str = "Summary!B2";

pub fn financial_model() -> Entries {
    let sml = "application/vnd.openxmlformats-officedocument.spreadsheetml";
    let summary = sheet(&[
        vec![
            istr("A1", "Q3 revenue (EUR M)"),
            formula("B1", "Quarterly!B4", "18.6"),
        ],
        vec![
            istr("A2", "Annual revenue (EUR M)"),
            formula("B2", "Quarterly!B6", "75"),
        ],
        vec![istr(
            "A3",
            "Q4 is a forecast; Q1 to Q3 are actuals. Synthetic data.",
        )],
    ]);
    let quarterly = sheet(&[
        vec![
            istr("A1", "Quarter"),
            istr("B1", "Revenue (EUR M)"),
            istr("C1", "Basis"),
        ],
        vec![istr("A2", "Q1"), num("B2", "17.2"), istr("C2", "actual")],
        vec![istr("A3", "Q2"), num("B3", "17.9"), istr("C3", "actual")],
        vec![istr("A4", "Q3"), num("B4", "18.6"), istr("C4", "actual")],
        vec![istr("A5", "Q4"), num("B5", "21.3"), istr("C5", "forecast")],
        vec![istr("A6", "Annual"), formula("B6", "SUM(B2:B5)", "75")],
    ]);
    let styles = format!("{XML}<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"1\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/></cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>");
    vec![
        ("[Content_Types].xml".into(), content_types(&[("/xl/workbook.xml", &format!("{sml}.sheet.main+xml")), ("/xl/worksheets/sheet1.xml", &format!("{sml}.worksheet+xml")), ("/xl/worksheets/sheet2.xml", &format!("{sml}.worksheet+xml")), ("/xl/styles.xml", &format!("{sml}.styles+xml")), ("/docProps/core.xml", CT_CORE)])),
        ("_rels/.rels".into(), format!("{XML}<Relationships xmlns=\"{REL_PKG}\"><Relationship Id=\"rId1\" Type=\"{NS_R}/officeDocument\" Target=\"xl/workbook.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>")),
        ("docProps/core.xml".into(), core("Financial model (synthetic)")),
        ("xl/workbook.xml".into(), format!("{XML}<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"{NS_R}\"><sheets><sheet name=\"Summary\" sheetId=\"1\" r:id=\"rId1\"/><sheet name=\"Quarterly\" sheetId=\"2\" r:id=\"rId2\"/></sheets><calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/></workbook>")),
        ("xl/_rels/workbook.xml.rels".into(), rels(&[("rId1", "worksheet", "worksheets/sheet1.xml"), ("rId2", "worksheet", "worksheets/sheet2.xml"), ("rId3", "styles", "styles.xml")])),
        ("xl/styles.xml".into(), styles),
        ("xl/worksheets/sheet1.xml".into(), summary),
        ("xl/worksheets/sheet2.xml".into(), quarterly),
    ]
}

// ---------------------------------------------------------------------------- docx

fn para(style: Option<&str>, text: &str) -> String {
    let ppr = style
        .map(|s| format!("<w:pPr><w:pStyle w:val=\"{s}\"/></w:pPr>"))
        .unwrap_or_default();
    format!(
        "<w:p>{ppr}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        esc(text)
    )
}

pub fn report() -> Entries {
    let wml = "application/vnd.openxmlformats-officedocument.wordprocessingml";
    let cell = |t: &str| {
        format!(
            "<w:tc><w:tcPr><w:tcW w:w=\"2400\" w:type=\"dxa\"/></w:tcPr>{}</w:tc>",
            para(None, t)
        )
    };
    let mut rows = format!(
        "<w:tr>{}{}{}</w:tr>",
        cell("Quarter"),
        cell("Revenue (EUR M)"),
        cell("Basis")
    );
    for (q, v, b) in QUARTERS {
        rows.push_str(&format!("<w:tr>{}{}{}</w:tr>", cell(q), cell(v), cell(b)));
    }
    let table = format!("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/><w:tblBorders><w:top w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"8A93A0\"/><w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"8A93A0\"/><w:insideH w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"B8BEC7\"/></w:tblBorders></w:tblPr><w:tblGrid><w:gridCol w:w=\"2400\"/><w:gridCol w:w=\"2400\"/><w:gridCol w:w=\"2400\"/></w:tblGrid>{rows}</w:tbl>");
    let body = format!(
        "{}{}{}{}{}{}{}{}{}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr>",
        para(Some("Title"), "Annual revenue report"),
        para(Some("Heading1"), "Summary"),
        para(None, &format!("Revenue for the year is EUR {ANNUAL}M: Q1 to Q3 are actuals and Q4 is a forecast. All figures in this document are synthetic.")),
        para(Some("Heading1"), "Quarterly revenue"),
        table,
        para(Some("Heading2"), "Q3"),
        para(None, "Q3 revenue was EUR 18.6M."),
        para(Some("Heading1"), "Outlook"),
        para(None, "The Q4 forecast of EUR 21.3M depends on two renewals."),
    );
    let styles = format!("{XML}<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\"/><w:sz w:val=\"22\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"160\" w:line=\"264\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style><w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:qFormat/><w:rPr><w:b/><w:sz w:val=\"52\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before=\"360\" w:after=\"120\"/><w:outlineLvl w:val=\"0\"/></w:pPr><w:rPr><w:b/><w:color w:val=\"2457F5\"/><w:sz w:val=\"32\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before=\"240\" w:after=\"80\"/><w:outlineLvl w:val=\"1\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"26\"/></w:rPr></w:style></w:styles>");
    vec![
        ("[Content_Types].xml".into(), content_types(&[("/word/document.xml", &format!("{wml}.document.main+xml")), ("/word/styles.xml", &format!("{wml}.styles+xml")), ("/docProps/core.xml", CT_CORE)])),
        ("_rels/.rels".into(), format!("{XML}<Relationships xmlns=\"{REL_PKG}\"><Relationship Id=\"rId1\" Type=\"{NS_R}/officeDocument\" Target=\"word/document.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>")),
        ("docProps/core.xml".into(), core("Annual revenue report (synthetic)")),
        ("word/document.xml".into(), format!("{XML}<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}</w:body></w:document>")),
        ("word/_rels/document.xml.rels".into(), rels(&[("rId1", "styles", "styles.xml")])),
        ("word/styles.xml".into(), styles),
    ]
}
