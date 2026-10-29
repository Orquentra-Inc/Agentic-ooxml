import { blake3 } from '@noble/hashes/blake3'
import { bytesToHex } from '@noble/hashes/utils'

export const REPO = __REPO_URL__
export const SITE = __SITE_URL__
export const BASE = import.meta.env.BASE_URL
export const docsUrl = (path = '') => `${REPO}/blob/main/${path}`
export const treeUrl = (path = '') => `${REPO}/tree/main/${path}`

export const AO_NS = 'urn:agentic-office:ooxml:v0.1'
export const CTX_NS = 'urn:agentic-office:context:v0.2'

// Synthetic example figures (EUR M). Not real company data.
export const FIGURES = [
  { q: 'Q1', v: 17.2, forecast: false },
  { q: 'Q2', v: 17.9, forecast: false },
  { q: 'Q3', v: 18.6, forecast: false },
  { q: 'Q4F', v: 21.3, forecast: true },
] as const
export const TOTAL = 75.0

/** The KPI shape exactly as it would sit in ppt/slides/slide1.xml. The claim hash is taken over these bytes. */
export function kpiShape(value: string): string {
  return (
    `<p:sp><p:nvSpPr><p:cNvPr id="12" name="KPI Q3 revenue"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>` +
    `<p:spPr><a:xfrm><a:off x="6858000" y="1371600"/><a:ext cx="2286000" cy="1143000"/></a:xfrm></p:spPr>` +
    `<p:txBody><a:bodyPr/><a:p><a:r><a:rPr lang="en-GB"/><a:t>${value}</a:t></a:r></a:p></p:txBody></p:sp>`
  )
}

/** Real BLAKE3 over the shape bytes, prefixed the way the Rust crate stores it. */
export function claimHash(shape: string): string {
  return 'blake3:' + bytesToHex(blake3(new TextEncoder().encode(shape)))
}

export const KPI_BASELINE = 'EUR 18.6M'
export const BASELINE_HASH = claimHash(kpiShape(KPI_BASELINE))
export const shortHash = (h: string) => h.slice(0, 13) + '…' + h.slice(-6)

export type LayerId = 'agents' | 'design' | 'intent' | 'sources' | 'spec'
export type Region = 'slide' | 'title' | 'kpi' | 'chart' | 'footer'

export interface Layer {
  id: LayerId
  label: string
  part: string
  status: string
  statusNote: string
  targets: Region[]
  blurb: string
  does: string[]
  xml: string
  caveat?: string
}

export const LAYERS: Layer[] = [
  {
    id: 'agents',
    label: 'Agents',
    part: 'customXml/agents.xml',
    status: 'Implemented, v0.1',
    statusNote: 'Free-text editing policy. A typed agent registry is proposed, not built.',
    targets: ['slide'],
    blurb: 'Document-specific rules for whoever edits this file next.',
    does: [
      'Lists the part and its sections cheaply, then loads only the policy that applies to the task.',
      'Treats the text as document data, never as authority: it cannot grant tools, network access or override the user.',
      'Applies rules such as "show a reviewable diff before committing" when planning an edit.',
    ],
    xml: `<ao:knowledge xmlns:ao="${AO_NS}" kind="agents" schemaVersion="0.1">
  <ao:summary>Guidance for editing financial slides</ao:summary>
  <ao:section id="edit-policy" format="text/markdown"><![CDATA[
# Editing policy
- Prefer changing existing editable PowerPoint objects.
- Do not replace a native chart with an image.
- Preserve historical figures unless the user authorizes a change.
- Show a reviewable diff before committing substantive edits.
  ]]></ao:section>
</ao:knowledge>`,
  },
  {
    id: 'design',
    label: 'Design',
    part: 'customXml/design.xml',
    status: 'Implemented, v0.1',
    statusNote: 'Free-text Markdown sections. Typed design tokens are proposed, not built.',
    targets: ['title', 'chart'],
    blurb: 'How this document should look, beyond what the theme already says.',
    does: [
      'Reads the native theme and layouts first; the design part only adds conventions the theme cannot express.',
      'Styles a new chart so a forecast is never drawn like an actual.',
      'Flags a restyle that contradicts a stated convention instead of silently applying it.',
    ],
    xml: `<ao:knowledge xmlns:ao="${AO_NS}" kind="design" schemaVersion="0.1">
  <ao:summary>Board deck: restrained, one accent</ao:summary>
  <ao:section id="charts" format="text/markdown"><![CDATA[
- Solid bars for actuals; outlined bars for forecast (Q4F).
- Currency as EUR M with one decimal.
- One accent colour per slide.
  ]]></ao:section>
</ao:knowledge>`,
  },
  {
    id: 'intent',
    label: 'Intent',
    part: 'customXml/intent.xml',
    status: 'Implemented, v0.1',
    statusNote: 'Free-text sections addressed by id, for example one per slide.',
    targets: ['title', 'kpi', 'footer'],
    blurb: 'Why this slide exists, and what must survive an edit.',
    does: [
      'Recovers audience and purpose in a new session, with no conversation history.',
      'Reads the constraint on this slide before changing any number on it.',
      'Asks the user, rather than guessing, when a change would alter the narrative.',
    ],
    xml: `<ao:knowledge xmlns:ao="${AO_NS}" kind="intent" schemaVersion="0.1">
  <ao:summary>Board narrative and constraints</ao:summary>
  <ao:section id="slide-1" format="text/markdown"><![CDATA[
## Slide 1
Purpose: show acceleration in revenue.
Constraint: label forecasts; do not alter audited actuals.
  ]]></ao:section>
</ao:knowledge>`,
  },
  {
    id: 'sources',
    label: 'Sources',
    part: 'customXml/sources.xml',
    status: 'Implemented, v0.1',
    statusNote: 'Claim hashes, effective status and cross-file value claims are implemented. Automatic slide edits are proposed.',
    targets: ['kpi'],
    blurb: 'Where a claim came from, and a hash of the claim as it was verified.',
    does: [
      'Finds the source record for the object it is about to edit, by part and object id.',
      'Computes the effective status: a stored "verified" holds only while the claim still hashes to its baseline.',
      'Marks a changed claim stale on reconcile, and keeps the old hash as the baseline.',
    ],
    xml: `<ao:sources xmlns:ao="${AO_NS}" schemaVersion="0.1">
  <ao:source id="src-q3-revenue" status="verified"
             sourceKind="workbook" checkedAt="2026-10-01">
    <ao:resource uri="relative:Financial_Model.xlsx"/>
    <ao:location>Revenue!C4</ao:location>
    <ao:claimLocator part="/ppt/slides/slide1.xml"
                     objectLocalId="12"
                     claimHash="${BASELINE_HASH}"/>
  </ao:source>
</ao:sources>`,
  },
  {
    id: 'spec',
    label: 'Spec (v0.2)',
    part: 'customXml/spec.xml',
    status: 'v0.2, read-only here',
    statusNote: 'The Rust crate reads it. The writer lives in TypeScript tools that are not part of this repository yet.',
    targets: ['slide'],
    blurb: 'The requirement contract: what was asked, assumed, left open and must be proved.',
    does: [
      'Checks the finished work against the criteria, not against its own memory of the request.',
      'Surfaces unconfirmed assumptions and open questions to the user before acting on them.',
      'Reports how many criteria are met, as a one-line summary, via the v0.2 context reader.',
    ],
    xml: `<ao:spec xmlns:ao="${CTX_NS}" schemaVersion="0.2" rev="4"
         mode="deep" format="pptx">
  <ao:request>Board slide on Q3 revenue</ao:request>
  <ao:requirement id="r1" source="file">Use the Q3 actuals</ao:requirement>
  <ao:assumption id="a1" status="unconfirmed">Show EUR, not USD</ao:assumption>
  <ao:question id="q1" status="open"><ao:text>Show Q4F?</ao:text></ao:question>
  <ao:criterion id="c1" check="tool" status="met">No overlaps</ao:criterion>
  <ao:grounding>Q3 revenue (actual): 18.6m (Financial_Model.xlsx)</ao:grounding>
</ao:spec>`,
    caveat: 'Different namespace from v0.1: urn:agentic-office:context:v0.2.',
  },
]

export interface PartInfo {
  path: string
  group: 'native' | 'ao'
  indent: number
  region?: Region
  role: string
  snippet: string
}

export const PARTS: PartInfo[] = [
  {
    path: '[Content_Types].xml', group: 'native', indent: 0,
    role: 'Declares the content type of every part. A custom part needs an entry here or the package is not well-formed OPC.',
    snippet: `<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml"
    ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml"
    ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>`,
  },
  {
    path: '_rels/.rels', group: 'native', indent: 0,
    role: 'Package-level relationships. The entry point to the main document part.',
    snippet: `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
    Target="ppt/presentation.xml"/>
</Relationships>`,
  },
  {
    path: 'ppt/presentation.xml', group: 'native', indent: 0, region: 'slide',
    role: 'The presentation: slide list, slide size and master references. Authoritative for structure.',
    snippet: `<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="5143500"/>
</p:presentation>`,
  },
  {
    path: 'ppt/_rels/presentation.xml.rels', group: 'native', indent: 0,
    role: 'Relationships from the presentation part. Custom XML parts are attached here, using the standard customXml relationship type.',
    snippet: `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type=".../relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId9"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml"
    Target="../customXml/intent.xml"/>
</Relationships>`,
  },
  {
    path: 'ppt/slides/slide1.xml', group: 'native', indent: 0, region: 'kpi',
    role: 'The slide itself. Shapes carry a local numeric id; the Q3 KPI here is shape 12. This is where a claim lives.',
    snippet: kpiShape(KPI_BASELINE).replace('<p:sp>', '<p:spTree> …\n<p:sp>').replace('</p:sp>', '</p:sp>\n… </p:spTree>'),
  },
  {
    path: 'ppt/slides/_rels/slide1.xml.rels', group: 'native', indent: 0,
    role: 'Relationships from the slide to its layout and chart.',
    snippet: `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type=".../relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/>
  <Relationship Id="rId2" Type=".../relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>`,
  },
  {
    path: 'ppt/charts/chart1.xml', group: 'native', indent: 0, region: 'chart',
    role: 'The native chart, with its cached values. The chart is editable data, not a picture.',
    snippet: `<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:plotArea><c:barChart>
    <c:ser><c:val><c:numRef>
      <c:numCache>
        <c:pt idx="0"><c:v>17.2</c:v></c:pt>  <c:pt idx="1"><c:v>17.9</c:v></c:pt>
        <c:pt idx="2"><c:v>18.6</c:v></c:pt>  <c:pt idx="3"><c:v>21.3</c:v></c:pt>
      </c:numCache>
    </c:numRef></c:val></c:ser>
  </c:barChart></c:plotArea></c:chart>
</c:chartSpace>`,
  },
  {
    path: 'ppt/theme/theme1.xml', group: 'native', indent: 0, region: 'title',
    role: 'The native design source: colour scheme and fonts. Agentic OOXML reads this rather than copying it.',
    snippet: `<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Board">
  <a:themeElements>
    <a:clrScheme name="Board"> <a:accent1><a:srgbClr val="2F5BFF"/></a:accent1> … </a:clrScheme>
    <a:fontScheme name="Board"> … </a:fontScheme>
  </a:themeElements>
</a:theme>`,
  },
  {
    path: 'customXml/agents.xml', group: 'ao', indent: 0, region: 'slide',
    role: 'Agentic OOXML part. Editing policy for this document.',
    snippet: LAYERS[0].xml,
  },
  {
    path: 'customXml/design.xml', group: 'ao', indent: 0, region: 'chart',
    role: 'Agentic OOXML part. Design conventions beyond the theme.',
    snippet: LAYERS[1].xml,
  },
  {
    path: 'customXml/intent.xml', group: 'ao', indent: 0, region: 'title',
    role: 'Agentic OOXML part. Audience, purpose, constraints.',
    snippet: LAYERS[2].xml,
  },
  {
    path: 'customXml/sources.xml', group: 'ao', indent: 0, region: 'kpi',
    role: 'Agentic OOXML part. Optional. Evidence records with claim hashes.',
    snippet: LAYERS[3].xml,
  },
]
