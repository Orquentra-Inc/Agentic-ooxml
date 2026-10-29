import { execFileSync } from "node:child_process";
import fs from "node:fs"; import os from "node:os"; import path from "node:path";
import JSZip from "<MONOREPO>/mcp/office/pptx/node_modules/jszip/lib/index.js";
const { insertSlides } = await import("<MONOREPO>/mcp/office/pptx/build/tools/author/svgSlides.js");
const AO = "urn:agentic-office:ooxml:v0.1", R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const body = (k) => `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><ao:knowledge xmlns:ao="${AO}" kind="${k}" schemaVersion="0.1"><ao:summary>${k}</ao:summary></ao:knowledge>`;
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "aop-"));
const oc = (...a) => execFileSync("officecli", a, { stdio: "pipe", encoding: "utf8" });
const base = path.join(tmp, "base.pptx"); oc("create", base); oc("add", base, "/", "--type", "slide", "--prop", "title=One"); oc("add", base, "/", "--type", "slide", "--prop", "title=Two"); oc("close", base);
// a deck carrying 4 AO parts at presentation level (PowerPoint-standard encoding)
const z = await JSZip.loadAsync(fs.readFileSync(base)); let r = await z.file("ppt/_rels/presentation.xml.rels").async("string");
["agents", "design", "intent", "sources"].forEach((k, i) => { z.file(`customXml/item${i + 1}.xml`, body(k)); r = r.replace("</Relationships>", `<Relationship Id="rId90${i}" Type="${R}/customXml" Target="../customXml/item${i + 1}.xml"/></Relationships>`); });
z.file("ppt/_rels/presentation.xml.rels", r);
const withParts = path.join(tmp, "ao.pptx"); fs.writeFileSync(withParts, await z.generateAsync({ type: "nodebuffer" }));
async function check(file, label) {
  const o = await JSZip.loadAsync(fs.readFileSync(file)); let found = 0, referenced = 0;
  const rel = (await Promise.all(Object.keys(o.files).filter((n) => n.endsWith(".rels")).map((n) => o.file(n).async("string")))).join("\n");
  for (const k of ["agents", "design", "intent", "sources"]) { let at = ""; for (const n of Object.keys(o.files)) { if (o.files[n].dir || !/\.xml$/.test(n)) continue; const t = await o.file(n).async("string"); if (t.includes(`kind="${k}"`) && t.includes(AO)) { at = n; break; } } if (at) { found++; if (new RegExp(`Type="${R}/customXml"[^>]*Target="[^"]*${at.split("/").pop().replace(".", "\\.")}"|Target="[^"]*${at.split("/").pop().replace(".", "\\.")}"[^>]*Type="${R}/customXml"`).test(rel)) referenced++; } }
  console.log(label.padEnd(58), `present ${found}/4, referenced ${referenced}/4`);
}
const copy = (n) => { const f = path.join(tmp, n + ".pptx"); fs.copyFileSync(withParts, f); return f; };
await check(withParts, "input (reference)");
let f = copy("addslide"); oc("add", f, "/", "--type", "slide", "--prop", "title=Three"); oc("close", f); await check(f, "officecli add slide");
f = copy("removeslide"); oc("remove", f, "/slide[2]"); oc("close", f); await check(f, "officecli remove slide");
f = copy("set"); try { oc("set", f, "/slide[1]", "--prop", "background=#112233"); } catch (e) { console.log("  (set background failed: " + String(e.message).slice(0, 80) + ")"); } oc("close", f); await check(f, "officecli set (slide background)");
f = copy("merge"); const mo = path.join(tmp, "merged.pptx"); try { oc("merge", f, mo, "--data", "{}", "--force"); await check(mo, "officecli merge (template fill)"); } catch (e) { console.log("merge: " + String(e.message).slice(0, 100)); }
f = copy("insert"); const svg = path.join(tmp, "s.pptx"); const sz = await JSZip.loadAsync(fs.readFileSync(base)); fs.writeFileSync(svg, await sz.generateAsync({ type: "nodebuffer" }));
await insertSlides(f, svg, path.join(tmp, "inserted.pptx"), 1); await check(path.join(tmp, "inserted.pptx"), "svg_to_deck insertSlides (JSZip package merge)");
f = copy("roundjszip"); const jz = await JSZip.loadAsync(fs.readFileSync(f)); fs.writeFileSync(f, await jz.generateAsync({ type: "nodebuffer" })); await check(f, "plain JSZip rewrite (control)");
