// Scratch experiment (not in the repo): do Agentic-OOXML-style custom parts survive the pptx editor's import -> save?
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import JSZip from 'jszip';
import { importPptxToSlides } from '<MONOREPO>/apps/editors/pptx/extension/webview/src/vendor/pptx/pptx-import';
import { exportSlidesToPptx } from '<MONOREPO>/apps/editors/pptx/extension/webview/src/vendor/pptx/pptx-export';
import { renderCharts, withoutDerived } from '<MONOREPO>/apps/editors/pptx/extension/webview/src/chart/chartLayer';

const NS_R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const AO = 'urn:agentic-office:ooxml:v0.1';
const KINDS = ['agents', 'design', 'intent', 'sources'] as const;
const body = (kind: string) =>
  `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><ao:knowledge xmlns:ao="${AO}" kind="${kind}" schemaVersion="0.1"><ao:summary>${kind} summary</ao:summary><ao:section id="s1" format="text/markdown"><![CDATA[## ${kind}\nPurpose: persuade. Visual plan: photo cover.]]></ao:section></ao:knowledge>`;
const rels = (r: string, id: string, type: string, target: string) => r.replace('</Relationships>', `<Relationship Id="${id}" Type="${type}" Target="${target}"/></Relationships>`);
const ovr = (t: string, e: string) => t.replace('</Types>', `${e}</Types>`);

function baseDeck(): Buffer {
  const file = join(mkdtempSync(join(tmpdir(), 'aoexp-')), 'base.pptx');
  for (const a of [['create', file], ['add', file, '/', '--type', 'slide', '--prop', 'title=Cover'], ['add', file, '/', '--type', 'slide', '--prop', 'title=Body'], ['close', file]]) execFileSync('officecli', a, { stdio: 'pipe' });
  return readFileSync(file);
}

type Variant = { name: string; build: (zip: JSZip) => Promise<string[]> };   // returns the part paths it added
const pres = 'ppt/_rels/presentation.xml.rels';
const addTypes = async (zip: JSZip, entries: string[]) => { let t = await zip.file('[Content_Types].xml')!.async('string'); for (const e of entries) t = ovr(t, e); zip.file('[Content_Types].xml', t); };

const VARIANTS: Variant[] = [
  { name: 'V1 OPC-standard: customXml/itemN.xml + itemProps + customXml rel (what PowerPoint writes)', build: async (zip) => {
    const added: string[] = []; let r = await zip.file(pres)!.async('string'); const ct: string[] = [];
    KINDS.forEach((k, i) => { const n = i + 1;
      zip.file(`customXml/item${n}.xml`, body(k)); added.push(`customXml/item${n}.xml`);
      zip.file(`customXml/itemProps${n}.xml`, `<?xml version="1.0" encoding="UTF-8" standalone="no"?><ds:datastoreItem ds:itemID="{6C4E1F2A-1B2C-4D5E-8F90-12345678900${n}}" xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml"><ds:schemaRefs><ds:schemaRef ds:uri="${AO}"/></ds:schemaRefs></ds:datastoreItem>`);
      zip.file(`customXml/_rels/item${n}.xml.rels`, `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="${NS_R}/customXmlProps" Target="itemProps${n}.xml"/></Relationships>`);
      r = rels(r, `rId90${n}`, `${NS_R}/customXml`, `../customXml/item${n}.xml`);
      ct.push(`<Override PartName="/customXml/itemProps${n}.xml" ContentType="application/vnd.openxmlformats-officedocument.customXmlProperties+xml"/>`); });
    zip.file(pres, r); await addTypes(zip, ct); return added; } },
  { name: 'V2 friendly names (customXml/agents.xml...), customXml rel, no itemProps', build: async (zip) => {
    const added: string[] = []; let r = await zip.file(pres)!.async('string'); let t = await zip.file('[Content_Types].xml')!.async('string');
    if (!/Extension="xml"/i.test(t)) t = t.replace('<Types', '<Types');
    KINDS.forEach((k, i) => { zip.file(`customXml/${k}.xml`, body(k)); added.push(`customXml/${k}.xml`); r = rels(r, `rId91${i}`, `${NS_R}/customXml`, `../customXml/${k}.xml`); });
    zip.file(pres, r); return added; } },
  { name: 'V3 proposal-style: customXml/ao/*.xml, CUSTOM relationship type, content-type override', build: async (zip) => {
    const added: string[] = []; let r = await zip.file(pres)!.async('string'); const ct: string[] = [];
    KINDS.forEach((k, i) => { zip.file(`customXml/ao/${k}.xml`, body(k)); added.push(`customXml/ao/${k}.xml`); r = rels(r, `rId92${i}`, `${AO}/knowledge`, `../customXml/ao/${k}.xml`); ct.push(`<Override PartName="/customXml/ao/${k}.xml" ContentType="application/vnd.agenticoffice.knowledge+xml"/>`); });
    zip.file(pres, r); await addTypes(zip, ct); return added; } },
  { name: 'V4 slide-level: one part related from slide 2 (customXml rel)', build: async (zip) => {
    zip.file('customXml/slide2-intent.xml', body('intent'));
    const p = 'ppt/slides/_rels/slide2.xml.rels';
    zip.file(p, rels((await zip.file(p)?.async('string')) ?? '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>', 'rId93', `${NS_R}/customXml`, '../../customXml/slide2-intent.xml'));
    return ['customXml/slide2-intent.xml']; } },
  { name: 'V5 control: orphan parts, no relationship from anything', build: async (zip) => {
    const added: string[] = []; const ct: string[] = [];
    KINDS.forEach((k) => { zip.file(`customXml/orphan-${k}.xml`, body(k)); added.push(`customXml/orphan-${k}.xml`); ct.push(`<Override PartName="/customXml/orphan-${k}.xml" ContentType="application/xml"/>`); });
    await addTypes(zip, ct); return added; } },
];

const ab = (b: Buffer) => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) as ArrayBuffer;
async function save(bytes: Buffer, edit?: (deck: any) => void): Promise<Buffer> {
  const deck = await importPptxToSlides(ab(bytes), 'deck.pptx'); renderCharts(deck); edit?.(deck);
  return Buffer.from(await (await exportSlidesToPptx(withoutDerived(deck))).arrayBuffer());
}
function validates(bytes: Buffer): string[] {
  const f = join(mkdtempSync(join(tmpdir(), 'aoexp-v-')), 'd.pptx'); writeFileSync(f, bytes); let rep = '';
  try { rep = execFileSync('officecli', ['validate', f], { encoding: 'utf8', stdio: 'pipe' }); } catch (e) { rep = `${(e as any).stdout ?? ''}${(e as any).stderr ?? e}`; }
  return rep.split('\n').filter((l) => /^\s*\[/.test(l) && !/notesMasterIdLst/.test(l));
}

const base = baseDeck();
const results: string[] = [];
for (const v of VARIANTS) for (const edit of ['no edit', 'reorder slides'] as const) {
  test(`${v.name} | ${edit}`, async () => {
    const zip = await JSZip.loadAsync(base); const added = await v.build(zip);
    const input = await zip.generateAsync({ type: 'nodebuffer' });
    const startErr = validates(input);
    const out = await JSZip.loadAsync(await save(input, edit === 'reorder slides' ? (deck) => { const o = deck.body.pageOrder; deck.body.pageOrder = [o[1], ...o.filter((_: unknown, i: number) => i !== 1)]; } : undefined));
    // kept = found by content; then the three things that make it USABLE: identical bytes, still related (same rel type) from the
    // presentation or a slide, and its content type intact. Paths may be renamed by a writer, so match on content, then on target.
    const kept: string[] = []; const lost: string[] = []; const notes: string[] = [];
    const relTypeWanted = v.name.startsWith('V3') ? `${AO}/knowledge` : `${NS_R}/customXml`;
    const relFiles = Object.keys(out.files).filter((n) => n.endsWith('.rels'));
    const relsText = (await Promise.all(relFiles.map(async (n) => ({ n, t: await out.file(n)!.async('string') }))));
    const ct = await out.file('[Content_Types].xml')!.async('string');
    for (const p of added) {
      const orig = await zip.file(p)!.async('string'); const k = /kind="(\w+)"/.exec(orig)![1]; let at = '';
      for (const n of Object.keys(out.files)) { if (out.files[n].dir) continue; const t = await out.file(n)!.async('string').catch(() => ''); if (t.includes(`kind="${k}"`) && t.includes(AO)) { at = n; break; } }
      if (!at) { lost.push(k); continue; }
      const same = (await out.file(at)!.async('string')) === orig;
      const base = at.split('/').pop()!;
      const referenced = relsText.some((r) => new RegExp(`Type="${relTypeWanted.replace(/[/.]/g, '\\$&')}"[^>]*Target="[^"]*${base.replace('.', '\\.')}"|Target="[^"]*${base.replace('.', '\\.')}"[^>]*Type="${relTypeWanted.replace(/[/.]/g, '\\$&')}"`).test(r.t));
      const ctOk = v.name.startsWith('V3') ? ct.includes(`PartName="/${at}"`) : true;
      kept.push(k); if (!same) notes.push(`${k}:bytes changed`); if (!referenced) notes.push(`${k}:NOT referenced`); if (!ctOk) notes.push(`${k}:content type lost`);
    }
    const endErr = validates(await out.generateAsync({ type: 'nodebuffer' }));
    results.push(`${v.name.slice(0, 2)} ${edit.padEnd(14)} present ${kept.length}/${added.length}${lost.length ? ' (lost: ' + lost.join(',') + ')' : ''}${notes.length ? ' | PROBLEMS: ' + [...new Set(notes)].join('; ') : ' | all intact, referenced'} | validator ${startErr.length}->${endErr.length}`);
    assert.ok(true);
  });
}
test('summary', () => { console.log('\n=== AO-OOXML parts through the pptx editor save ===\n' + results.join('\n')); });
