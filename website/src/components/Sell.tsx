import { SectionHead, Badge } from './Chrome'
import { Reveal } from './Reveal'
import { CopyCommand } from './CopyCommand'
import { BASE, CARGO_CMD, INSTALL_CMD, REPO, docsUrl } from '../data'

/* 02 ---------------------------------------------------------------- the cost */

const SCENARIOS = [
  {
    title: 'The quarterly figure drifts from the model.',
    without: 'Finance corrects Q3 in the financial model to 19.1. The board deck still says 18.6. Nothing in either file says the two are linked, so nobody is told.',
    withAo: 'The number on the slide records its source: the workbook and the cell, Summary!B1. The next time the deck is read, that source reports stale (external_claim_changed), although the deck itself was never touched.',
    proof: <>Real output: step 3 of <a href="#run">the recorded session</a>.</>,
  },
  {
    title: 'The forecast label quietly disappears.',
    without: 'An agent tidies a chart and drops the word forecast from the Q4 bar. The slide looks cleaner and now shows a projection like an actual. No one asked for it. The agent never knew the rule.',
    withAo: 'The rule sits beside the slide, in plain text the agent reads before it edits. The example deck says: "Q1 to Q3 actual, Q4 forecast". The file states the rule; it cannot force an agent to follow it.',
    proof: <>Text from <span className="mono">examples/board-deck.pptx</span>. Whether agents follow it is not measured here.</>,
  },
  {
    title: 'An edit quietly invalidates the evidence.',
    without: 'A KPI is reworded or restyled. A footnote still says it was checked against the model. The check no longer applies, and the claim still reads as true.',
    withAo: 'The source record stores a BLAKE3 hash of the claimed shape. Change the shape and the record reports stale (claim_changed) until someone re-verifies. A stored "verified" is never trusted on its own.',
    proof: <>Real output: <span className="mono">stale_reasons</span> in step 6 of <a href={docsUrl('examples/demo-output.txt')}>the transcript</a>.</>,
  },
]

export function CostScene() {
  return (
    <section id="cost" className="scene" aria-labelledby="h-cost">
      <div className="wrap">
        <SectionHead n="02" total="12" eyebrow="The cost of not knowing" id="h-cost" lede="An agent edits fast. It cannot see what the file never said. Three ways that goes wrong in a board pack, and what changes when the file carries the context.">
          Fast edits, no memory.
        </SectionHead>
        <Reveal>
          <ul className="plain scenarios">
            {SCENARIOS.map((s, k) => (
              <li key={s.title} className="scenario">
                <h3><span className="mono small muted">{String(k + 1).padStart(2, '0')}</span> {s.title}</h3>
                <div className="vs">
                  <div className="vs-col vs-without"><p className="label">Without</p><p>{s.without}</p></div>
                  <div className="vs-col vs-with"><p className="label">With Agentic OOXML</p><p>{s.withAo}</p></div>
                </div>
                <p className="small muted">{s.proof}</p>
              </li>
            ))}
          </ul>
          <p className="small muted fine"><Badge kind="warn">Illustrative</Badge> These scenarios show the failure mode. We have not measured how often they happen.</p>
        </Reveal>
      </div>
    </section>
  )
}

/* 03 ------------------------------------------------------------ the questions */

const QUESTIONS = [
  { q: 'Who is this for?', part: 'Intent', body: 'Audience, purpose and constraints, per slide or section. A new session recovers the brief without the chat history.' },
  { q: 'Where did this number come from?', part: 'Sources', body: 'The workbook, the cell and the date it was checked, attached to the shape that shows the number.' },
  { q: 'Is it still true?', part: 'Claim hash', body: 'A hash of the claim as it was verified, recomputed on every read. Move the workbook cell and the deck’s source goes stale.' },
  { q: 'What must not change?', part: 'Agents and design', body: 'Editing rules and visual conventions in plain Markdown. Guidance the agent reads, not a lock: nothing enforces it against an agent that ignores it.' },
  { q: 'Did my edit break anything?', part: 'Validate and diff', body: 'validate checks the ZIP, XML, content types, relationships and stale sources. diff lists which native parts and source statuses changed.' },
]

export function QuestionsScene() {
  return (
    <section id="questions" className="scene" aria-labelledby="h-questions">
      <div className="wrap">
        <SectionHead n="03" total="12" eyebrow="Five questions" id="h-questions" lede={<>Today the file answers none of them. Agentic OOXML gives each one a place inside the file. We borrowed the framing from the <a href="https://github.com/GoogleCloudPlatform/open-knowledge-format">Open Knowledge Format</a>, which starts from what a consumer needs answered before it defines a schema.</>}>
          Five questions every agent must answer before it touches an Office file.
        </SectionHead>
        <Reveal>
          <ol className="plain qgrid">
            {QUESTIONS.map((x, k) => (
              <li key={x.q} className="qcard">
                <span className="mono small muted">Q{k + 1}</span>
                <h3>{x.q}</h3>
                <p>{x.body}</p>
                <p className="small"><span className="label">The file carries</span><span className="mono qpart">{x.part}</span></p>
              </li>
            ))}
          </ol>
          <p className="cta-row"><a className="btn btn-quiet" href={`${BASE}how-it-works/`}>See the XML behind each answer</a></p>
        </Reveal>
      </div>
    </section>
  )
}

/* 06 -------------------------------------------------------------- highlights */

const TILES = [
  { h: 'Your original is safe by default', p: 'Through MCP, a write produces a sibling .ao-proposed copy and leaves the original alone unless you ask for in-place. Every native package entry is copied byte for byte.' },
  { h: 'Works on the three formats', p: 'Reads and writes .docx, .pptx and .xlsx. Claim hashes cover a single shape in a .pptx and the whole part in .docx and .xlsx.' },
  { h: 'Plugs into the agent you already use', p: 'A stdio MCP server with eight tools, a CLI that prints JSON, and a Claude Code plugin. Codex, Cursor and Pi configs exist but are experimental.' },
  { h: 'Provenance that travels with the file', p: 'Intent, sources and rules live in standard custom XML parts inside the package, so they move when the file is emailed or copied. Found by relationship and namespace, not file name.' },
  { h: 'Validate before you hand it over', p: 'validate checks the ZIP, every XML part, content types, relationships and stale sources. It is a structural check, not proof that Office will open the file.' },
  { h: 'Read-only until you allow writes', p: 'MCP writes are off unless AGENTIC_OOXML_ALLOW_WRITE=1, and paths are confined to AGENTIC_OOXML_ROOTS. Text from a document is always marked trust: "document".' },
  { h: 'Fully local', p: 'The core, CLI and MCP crates depend on no HTTP or network library, and your documents are not sent anywhere by this code. Your agent still talks to its own model provider.' },
  { h: 'Open and MIT', p: 'The code, schema and examples are open to read and question. 76 Rust tests pass with cargo test --workspace at the time of writing.' },
]

export function HighlightsScene() {
  return (
    <section id="highlights" className="scene" aria-labelledby="h-high">
      <div className="wrap">
        <SectionHead n="07" total="12" eyebrow="Highlights" id="h-high" lede="What you get, in plain terms. Each line is backed by code or tests in the repository.">
          Safe, local, and built to be checked.
        </SectionHead>
        <Reveal>
          <ul className="plain tiles">
            {TILES.map((t) => (
              <li key={t.h} className="tile"><h3>{t.h}</h3><p>{t.p}</p></li>
            ))}
          </ul>
        </Reveal>
      </div>
    </section>
  )
}

/* 07 ------------------------------------------------------------------ who */

const WHO = [
  { h: 'Agent builders', p: 'You ship an agent that edits Office files and lose context between sessions.', first: 'Add the MCP server, then call ao_inspect and ao_sources on a deck before your agent edits it.', href: `${BASE}how-it-works/#install`, cta: 'MCP setup' },
  { h: 'Teams automating decks, models and reports', p: 'You want a changed number on a slide to be noticed before it reaches the board.', first: 'Run the recorded demo on the synthetic examples, then record one source claim in your own file.', href: docsUrl('docs/getting-started.md'), cta: 'Getting started' },
  { h: 'Document-tool developers', p: 'You build editors or converters and want to know what this convention asks of them.', first: 'Read the schema, then run validate before and after your tool saves a file.', href: docsUrl('docs/schema.md'), cta: 'Schema reference' },
]

export function AudienceScene() {
  return (
    <section id="who" className="scene" aria-labelledby="h-who">
      <div className="wrap">
        <SectionHead n="08" total="12" eyebrow="Who it is for" id="h-who">
          Three people, three first steps.
        </SectionHead>
        <Reveal>
          <ul className="plain who">
            {WHO.map((w) => (
              <li key={w.h} className="who-card">
                <h3>{w.h}</h3>
                <p>{w.p}</p>
                <p className="small"><span className="label">First thing to do</span>{w.first}</p>
                <a className="btn btn-quiet" href={w.href}>{w.cta}</a>
              </li>
            ))}
          </ul>
        </Reveal>
      </div>
    </section>
  )
}

/* 08 ------------------------------------------------------------- works with */

type St = 'tested' | 'exp' | 'planned'
const WORKS: { n: string; s: St; label: string; note: string }[] = [
  { n: 'MCP server (stdio)', s: 'tested', label: 'Tested', note: 'integrations/verify.sh drives the real binary over stdio: handshake, all eight tools listed, writes refused by default, paths outside the roots refused.' },
  { n: 'CLI', s: 'tested', label: 'Tested', note: 'Every command prints JSON; covered by the crate tests.' },
  { n: 'Claude Code plugin', s: 'tested', label: 'Tested (install)', note: 'claude plugin validate passes, and adding the marketplace and installing the plugin from the public repo succeeds. Running the skills in a live session has not been verified end to end.' },
  { n: 'Agent Skills', s: 'tested', label: 'Tested (format)', note: 'Four skills pass the repository’s own spec checker. Agent behaviour with them is experimental.' },
  { n: 'Codex CLI', s: 'exp', label: 'Experimental', note: 'A config exists. Not run.' },
  { n: 'Cursor', s: 'exp', label: 'Experimental', note: 'An MCP config exists. Not run; Cursor was not installed when it was written.' },
  { n: 'Pi', s: 'exp', label: 'Experimental', note: 'Skills and CLI. An MCP route and a package are planned.' },
]

export function WorksWithScene() {
  return (
    <section id="works-with" className="scene" aria-labelledby="h-works">
      <div className="wrap">
        <SectionHead n="09" total="12" eyebrow="Works with" id="h-works" lede={<>Status from running <span className="mono">integrations/verify.sh</span> and from <a href={docsUrl('integrations/README.md')}>integrations/README.md</a>. We would rather show Experimental than imply support.</>}>
          The agents you already use, honestly labelled.
        </SectionHead>
        <Reveal>
          <ul className="plain works">
            {WORKS.map((w) => (
              <li key={w.n} className="work">
                <div className="work-head"><h3>{w.n}</h3><span className={`status status-${w.s}`}>{w.label}</span></div>
                <p className="small muted">{w.note}</p>
              </li>
            ))}
          </ul>
        </Reveal>
      </div>
    </section>
  )
}

/* 09 ------------------------------------------------------------------ limits */

const LIMITS = [
  'It is not a Word-style editor. It does not edit slides, cells or paragraphs. Use the editing tool you already have.',
  'It does not check that your numbers are right. It notices that a claimed number changed.',
  'Rules in the file are guidance an agent reads. Nothing forces an agent to obey them.',
  'Round trips through Microsoft Office, LibreOffice, ONLYOFFICE and Google are unmeasured. The examples have never been opened in Office.',
  'It does not recalculate formulas. It compares the cached value stored in the workbook.',
  'It does not propose the matching slide edit for you when a workbook value changes. Detection is built; the edit is proposed.',
  'It is not on crates.io or npm. Install from a GitHub release, from source, or through the Claude Code marketplace.',
  'The schema is version 0.1 and early. It is a project convention, not part of ECMA-376 or ISO/IEC 29500.',
]

export function LimitsScene() {
  return (
    <section id="limits" className="scene" aria-labelledby="h-limits">
      <div className="wrap">
        <SectionHead n="10" total="12" eyebrow="Limits" id="h-limits" lede="We would rather you trust the claims above than be surprised by the gaps below.">
          What it does not do (yet).
        </SectionHead>
        <Reveal>
          <ul className="plain limits">
            {LIMITS.map((l) => <li key={l}>{l}</li>)}
          </ul>
          <p className="cta-row"><a className="btn btn-quiet" href={`${BASE}why/`}>Established, new and open: the full list</a></p>
        </Reveal>
      </div>
    </section>
  )
}

/* 10 -------------------------------------------------------------------- faq */

const FAQ: { q: string; a: React.ReactNode }[] = [
  {
    q: 'Will Word and PowerPoint still open my file?',
    a: <>We expect so, but we have not measured it. The added parts are standard OPC custom XML parts, a mechanism Office files already allow for application data, and the package stays an ordinary .docx, .pptx or .xlsx. The example files have never been opened in Office. The only retention measurement is from one project&rsquo;s own .pptx editor; see the <a href={docsUrl('docs/evidence/round-trip-2026-10-19/README.md')}>evidence</a> and <a href={docsUrl('docs/compatibility.md')}>compatibility</a> pages. Round-trip reports from Office are the most useful contribution you can make.</>,
  },
  {
    q: 'Does it send my documents anywhere?',
    a: <>Not from this code. The core, CLI and MCP crates have no HTTP or network library in their dependencies, and a search of the source finds no socket use. The one cross-file read, a workbook next to a deck, is a local file read. The agent you connect is separate: it may send what it reads to its own model provider. Treat that as you would any agent.</>,
  },
  {
    q: 'What if another editor strips the parts?',
    a: <>The file still opens, and the context is gone: a read then reports no Agentic OOXML parts, and nothing pretends otherwise. In the one editor measured, presentation-level parts survived and a slide-level relationship was dropped, which is why the parts are linked from the main part. Anything beyond that editor is unmeasured.</>,
  },
  {
    q: 'Is it only for Claude?',
    a: <>No. The MCP server speaks stdio to any MCP client, and the CLI prints JSON that any agent or script can read. The Claude Code plugin is the packaged route and its manifest is validated. Codex, Cursor and Pi configs exist and are experimental.</>,
  },
  {
    q: 'How is this different from python-docx, templates or other Office MCP tools?',
    a: <>Those edit content: they open a file and change a paragraph, a cell or a shape. Agentic OOXML does not edit content. It stores the context around the content: audience, sources, rules, and a hash to notice change. It is meant to sit beside your editing tool. The comparison table is on the <a href={`${BASE}why/`}>Why page</a>.</>,
  },
  {
    q: 'Does it check that my numbers are correct?',
    a: <>No. It notices that a number moved since someone verified it. Whether either number is right is still a human or upstream-system question.</>,
  },
  {
    q: 'Is the schema stable?',
    a: <>No. It is version 0.1, in the namespace <span className="mono">urn:agentic-office:ooxml:v0.1</span>, and may change. Typed design tokens and an agent registry are proposed, not built. Pin a release if you depend on it.</>,
  },
]

export function FaqScene() {
  return (
    <section id="faq" className="scene" aria-labelledby="h-faq">
      <div className="wrap narrow">
        <SectionHead n="11" total="12" eyebrow="Questions" id="h-faq">
          Questions people ask first.
        </SectionHead>
        <Reveal>
          <div className="faq">
            {FAQ.map((f) => (
              <details key={f.q}>
                <summary>{f.q}</summary>
                <p>{f.a}</p>
              </details>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  )
}

/* 11 ------------------------------------------------------------------- final */

export function FinalCta() {
  return (
    <section id="install" className="scene scene-close" aria-labelledby="h-install">
      <div className="wrap">
        <SectionHead n="12" total="12" eyebrow="Install" id="h-install" lede="One command installs the CLI and the MCP server. Then run the recorded demo on a synthetic deck and watch a number go stale.">
          Give your agent a file it can trust.
        </SectionHead>
        <Reveal>
          <CopyCommand wrap text={INSTALL_CMD} label="install command for Linux and macOS" />
          <p className="small muted fine">Installs <span className="mono">agentic-ooxml</span> and <span className="mono">agentic-ooxml-mcp</span> v0.1.0 into <span className="mono">~/.local/bin</span> after checking the release checksum. Linux x86_64 and macOS (arm64, x86_64). The binaries are not signed.</p>
          <div className="alt-install">
            <div>
              <p className="label">Have Rust?</p>
              <CopyCommand wrap text={CARGO_CMD} label="cargo install command" />
            </div>
            <div>
              <p className="label">Claude Code</p>
              <CopyCommand text={'claude plugin marketplace add Orquentra-Inc/Agentic-ooxml\nclaude plugin install agentic-ooxml@agentic-ooxml'} label="Claude Code plugin commands" />
            </div>
          </div>
          <p className="small muted fine">Windows: download the .zip from <a href={`${REPO}/releases`}>GitHub Releases</a>. The plugin does not bundle the binary, so install it first. Not on crates.io or npm. More routes and MCP setup on <a href={`${BASE}how-it-works/#install`}>How it works</a>.</p>
          <p className="cta-row">
            <a className="btn" href={REPO}>Star or fork on GitHub</a>
            <a className="btn btn-quiet" href={`${BASE}contribute/`}>Contribute</a>
            <a className="btn btn-quiet" href={docsUrl('docs/')}>Read the docs</a>
          </p>
        </Reveal>
      </div>
    </section>
  )
}
