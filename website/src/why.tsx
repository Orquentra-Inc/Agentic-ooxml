import { Page, SectionHead, Badge } from './components/Chrome'
import { Reveal } from './components/Reveal'
import { BASE, REPO, docsUrl } from './data'
import { mount } from './mount'

const L = {
  ecma: 'https://ecma-international.org/publications-and-standards/standards/ecma-376/',
  iso: 'https://www.iso.org/standard/71691.html',
  sdk: 'https://learn.microsoft.com/en-us/office/open-xml/about-the-open-xml-sdk',
  customPart: 'https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.packaging.presentationpart.addcustomxmlpart',
  slidePart: 'https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.packaging.slidepart',
  pml: 'https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document',
  okf: 'https://github.com/GoogleCloudPlatform/open-knowledge-format',
  blake3: 'https://github.com/BLAKE3-team/BLAKE3',
}

function Card({ title, kind, children }: { title: string; kind: 'impl' | 'wip' | 'prop' | 'warn'; children: React.ReactNode }) {
  return <li className="why-card"><p><Badge kind={kind}>{title}</Badge></p>{children}</li>
}

function Why() {
  return (
    <>
      <section className="scene page-top" aria-labelledby="h-why">
        <div className="wrap narrow">
          <p className="eyebrow"><span className="mono">Why</span><span className="rule" aria-hidden="true" />Agentic OOXML</p>
          <h1 id="h-why">Why Agentic OOXML?</h1>
          <p className="lede">
            Most of what is in this project is not new, and we would rather say so. This page separates what is
            established, what is a standard we build on, what we think is a new combination, and what nobody, including us, has answered yet.
          </p>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-est">
        <div className="wrap narrow">
          <SectionHead n="1" total="" eyebrow="Established" id="h-est">Established techniques we reuse</SectionHead>
          <Reveal>
            <ul className="plain why-list">
              <Card title="Established" kind="impl"><h3>Package parts joined by relationships</h3><p>Office files are ZIP packages of XML parts, typed by <span className="mono">[Content_Types].xml</span> and linked by <span className="mono">.rels</span> files. That is the Open Packaging Conventions, defined in <a href={L.ecma}>ECMA-376</a>.</p></Card>
              <Card title="Established" kind="impl"><h3>Custom XML parts in Office files</h3><p>Office documents have long carried application data in custom XML parts. The Open XML SDK exposes this for presentations through <a href={L.customPart}>PresentationPart.AddCustomXmlPart</a>, and for slides through the <a href={L.slidePart}>SlidePart</a> class.</p></Card>
              <Card title="Established" kind="impl"><h3>Content hashing to detect change</h3><p>Hashing content to notice that it changed is routine. We use <a href={L.blake3}>BLAKE3</a>.</p></Card>
              <Card title="Established" kind="impl"><h3>Selective retrieval of context</h3><p>Listing what exists cheaply, then loading only the relevant section, is a common pattern for agents and for documentation tooling.</p></Card>
            </ul>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-std">
        <div className="wrap narrow">
          <SectionHead n="2" total="" eyebrow="Existing standards" id="h-std">Standards this builds on, and does not replace</SectionHead>
          <Reveal>
            <ul className="plain why-list">
              <Card title="Standard" kind="impl"><h3>ECMA-376 and ISO/IEC 29500</h3><p>Office Open XML is standardised as <a href={L.ecma}>ECMA-376</a> and as <a href={L.iso}>ISO/IEC 29500</a>. The package model (Part 2) is OPC.</p></Card>
              <Card title="Standard" kind="impl"><h3>PresentationML structure</h3><p>Slides, shapes and chart parts keep their native meaning. Agentic OOXML reads them; see Microsoft's <a href={L.pml}>structure of a PresentationML document</a>.</p></Card>
              <Card title="Neighbouring work" kind="wip"><h3>Open Knowledge Format (Google Cloud)</h3><p>OKF describes a directory of Markdown files with YAML frontmatter and provenance, trust and freshness metadata (<a href={L.okf}>repository</a>). It is a useful neighbouring design, not an OOXML packaging spec. Borrowing its conventions does not make our XML an OKF-conformant bundle.</p></Card>
              <Card title="Boundary" kind="warn"><h3>Not part of any standard</h3><p>The <span className="mono">urn:agentic-office:ooxml:v0.1</span> namespace and the meaning of its parts are an application-level convention. Using a recognised custom-part mechanism is not the same as being standardised or universally preserved by other editors.</p></Card>
            </ul>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-nov">
        <div className="wrap narrow">
          <SectionHead n="3" total="" eyebrow="A new combination" id="h-nov">What we think is new: the combination</SectionHead>
          <Reveal>
            <p className="lede">We do not claim any single ingredient is novel, and we have not done an exhaustive prior-art survey. The proposal is that these pieces together are useful, and that is a testable claim, not a measured one.</p>
            <ul className="plain why-list">
              <Card title="Proposed" kind="prop"><h3>Four typed parts with one shared envelope</h3><p>Agents, design, intent and sources share one wrapper and API across .pptx, .docx and .xlsx, so an agent loads only the kind it needs.</p></Card>
              <Card title="Implemented" kind="impl"><h3>Claims tied to native object bytes</h3><p>A source record stores a hash of the claimed shape's own bytes. Its effective status is recomputed on read, so a stored "verified" cannot outlive an edit to the claim.</p></Card>
              <Card title="Implemented" kind="impl"><h3>Document text as data, behind a trusted core</h3><p>Embedded guidance is returned marked as document data. Safety limits and atomic writes live in the Rust code, where a modified part cannot switch them off.</p></Card>
            </ul>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-open">
        <div className="wrap narrow">
          <SectionHead n="4" total="" eyebrow="Open research" id="h-open">What is still open</SectionHead>
          <Reveal>
            <ul className="plain why-list">
              <Card title="Open" kind="warn"><h3>Does it survive other editors?</h3><p>Round-trip retention is <strong>unmeasured</strong> for Microsoft PowerPoint, Word and Excel, LibreOffice, ONLYOFFICE and Google import and export. The only measurement is from one project&rsquo;s own .pptx editor (2026-10-19): presentation-level parts were kept 4 of 4 in each of three encodings, with and without an edit, and a slide-level part was kept but lost its relationship. That says nothing about the applications above. No guarantee should be assumed until there is evidence; <a href={`${BASE}contribute/`}>you can help measure it</a>.</p></Card>
              <Card title="Open" kind="warn"><h3>Does it actually help agents?</h3><p>The proposal's hypotheses (less context lost across sessions, smaller context per task) have not been benchmarked. The 95 percent recall figure in the proposal is a target, not a result.</p></Card>
              <Card title="Open" kind="warn"><h3>Stable locators</h3><p>A shape id is not a durable identity across copy, split and reorder. When a locator is ambiguous, the safe answer is to mark the claim unverified.</p></Card>
              <Card title="Open" kind="warn"><h3>Cross-file edits</h3><p>Noticing that a workbook value changed is implemented (external claims). Proposing the matching slide edit automatically is not.</p></Card>
              <Card title="Open" kind="warn"><h3>Privacy and injection</h3><p>Source records can reveal internal paths and client names, and embedded text can try to instruct an agent. Both need policy, not just code.</p></Card>
            </ul>
            <p className="cta-row"><a className="btn" href={docsUrl('docs/')}>Read the documentation</a><a className="btn btn-quiet" href={`${BASE}contribute/`}>Contribute</a><a className="btn btn-quiet" href={REPO}>Repository</a><a className="btn btn-quiet" href={BASE}>Back to the overview</a></p>
          </Reveal>
        </div>
      </section>
    </>
  )
}

mount(<Page current="why"><Why /></Page>)
