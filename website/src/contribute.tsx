import { Page, SectionHead, Badge } from './components/Chrome'
import { Reveal } from './components/Reveal'
import { BASE, REPO, docsUrl } from './data'
import { mount } from './mount'

const issue = (template: string) => `${REPO}/issues/new?template=${template}`
const guide = (anchor = '') => docsUrl(`docs/contributing.md${anchor}`)

function Code({ label, children }: { label: string; children: string }) {
  return <pre className="code" tabIndex={0} aria-label={label}><code>{children}</code></pre>
}

function Way({ title, kind, tag, children }: { title: string; kind: 'impl' | 'wip' | 'prop' | 'warn'; tag: string; children: React.ReactNode }) {
  return <li className="why-card"><p><Badge kind={kind}>{tag}</Badge></p><h3>{title}</h3>{children}</li>
}

const ROUND_TRIP = `# 1. Work on a copy of a synthetic example. Never use a real document.
mkdir -p rt && cp examples/board-deck.pptx rt/before.pptx

# 2. Record the starting state.
agentic-ooxml list     --file rt/before.pptx > rt/list-before.json
agentic-ooxml validate --file rt/before.pptx

# 3. In the application: open rt/before.pptx, change one word, Save As
#    rt/after.pptx in the SAME format. Repeat once with no edit at all.

# 4. Record the result.
agentic-ooxml list     --file rt/after.pptx > rt/list-after.json
agentic-ooxml validate --file rt/after.pptx
agentic-ooxml diff --before rt/before.pptx --after rt/after.pptx`

const SETUP = `git clone https://github.com/Orquentra-Inc/Agentic-ooxml
cd Agentic-ooxml
cargo build --workspace
cargo test --workspace`

const CHECKS = `cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 docs/check-links.py          # links and anchors in every .md
python3 scripts/check-examples.py    # independent check of the examples
integrations/verify.sh               # SKIP means "not tested"

# regenerate the examples (output is byte-reproducible)
cargo run -q -p agentic-ooxml-examples --bin make-examples -- generate

# the website
cd website && npm ci && npm run dev
npm run build && npm run lint && npm run a11y`

const TASKS: { task: string; needs: string }[] = [
  { task: 'Run cargo test --workspace on Windows and report', needs: 'Windows, Rust' },
  { task: 'Audit dependency licences and put the table in the compatibility page', needs: 'Rust' },
  { task: 'Add a dedicated test for .xlsx v0.2 context reads', needs: 'Rust' },
  { task: 'Open the three examples in LibreOffice and another app, and report', needs: 'an Office app' },
  { task: 'Verify the Codex, Cursor or Pi integration against the real agent', needs: 'that agent' },
  { task: 'Write the design for the Streamable HTTP transport (planned, not built)', needs: 'Rust' },
  { task: 'Draft a worked proposal for typed design tokens', needs: 'schema thinking' },
  { task: 'Propose a stable claim locator under copy, move and reorder (RQ3)', needs: 'OOXML knowledge' },
  { task: 'Start a conformance corpus with expected validate and list output', needs: 'Python or Rust' },
]

function Contribute() {
  return (
    <>
      <section className="scene page-top" aria-labelledby="h-contribute">
        <div className="wrap narrow">
          <p className="eyebrow"><span className="mono">Contribute</span><span className="rule" aria-hidden="true" />Agentic OOXML</p>
          <h1 id="h-contribute">How to help</h1>
          <p className="lede">
            You do not need to be an OOXML expert, and for the most useful contributions you do not need Rust. The one rule:
            every public claim must match code that runs. If it is not built, we say &ldquo;proposed&rdquo;.
          </p>
          <p className="cta-row">
            <a className="btn" href={issue('round_trip.yml')}>Report a round trip</a>
            <a className="btn btn-quiet" href={guide()}>Read the full guide</a>
            <a className="btn btn-quiet" href={REPO}>Repository</a>
          </p>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-ways">
        <div className="wrap narrow">
          <SectionHead n="1" total="" eyebrow="Ways to help" id="h-ways">Ranked by impact</SectionHead>
          <Reveal>
            <ol className="plain why-list">
              <Way title="Report a round trip" kind="warn" tag="Most useful">
                <p>
                  The central open question is whether Office applications keep the parts when they save a file. PowerPoint, Word,
                  Excel, LibreOffice, ONLYOFFICE and Google are all <strong>unmeasured</strong>. A negative result is worth as much as a
                  positive one. The method is in the next section; send it with the{' '}
                  <a href={issue('round_trip.yml')}>round-trip report template</a>.
                </p>
              </Way>
              <Way title="Add example documents" kind="impl" tag="High">
                <p>Synthetic files only, with invented numbers. A <span className="mono">.docx</span> with comments, an <span className="mono">.xlsx</span> with named ranges, a deck with a chart part: anything the current three do not exercise. Open an issue first.</p>
              </Way>
              <Way title="Propose a schema change" kind="impl" tag="High">
                <p>Typed design tokens, a <span className="mono">validation</span> part and an agent registry are proposed, not built. They need a worked proposal before code. See <a href="#h-schema">how to propose one</a>.</p>
              </Way>
              <Way title="Integrations for other agents" kind="wip" tag="Medium">
                <p>The Claude Code, Codex, Cursor and Pi integrations are not verified end to end. Running one against the real agent and reporting what happened, or adding another agent, is high value. Use the <a href={issue('integration.yml')}>integration template</a>.</p>
              </Way>
              <Way title="Fix the docs" kind="wip" tag="Medium">
                <p>Run the commands in a page. If the output differs, or a claim goes further than the code, fix it or open a <a href={issue('docs.yml')}>documentation issue</a>.</p>
              </Way>
            </ol>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-method">
        <div className="wrap narrow">
          <SectionHead n="2" total="" eyebrow="The method" id="h-method">Reporting a round trip</SectionHead>
          <Reveal>
            <p className="lede">
              Install the binary (see <a href={`${BASE}#install`}>Install</a>), then work on a copy of a synthetic example.
              An empty <span className="mono">parts</span> list after a save means the context was lost, not that there was none.
            </p>
            <Code label="Round-trip method">{ROUND_TRIP}</Code>
            <p>Open an issue with the round-trip template and include:</p>
            <ul>
              <li>the application and its exact version, and your operating system;</li>
              <li>the starting file and exactly what you did in the application;</li>
              <li>both <span className="mono">list</span> outputs, and the <span className="mono">diff</span> and <span className="mono">validate</span> output for the saved file;</li>
              <li>anything the application said or changed, such as a repair prompt.</li>
            </ul>
            <p className="small muted">The only measurements that exist so far are from one project&rsquo;s own editor, and they say nothing about the applications above. See <a href={docsUrl('docs/compatibility.md')}>compatibility</a> and the <a href={docsUrl('docs/evidence/round-trip-2026-10-19/README.md')}>raw evidence</a>.</p>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-tasks">
        <div className="wrap narrow">
          <SectionHead n="3" total="" eyebrow="First tasks" id="h-tasks">Concrete things to pick up</SectionHead>
          <Reveal>
            <p className="lede">From the <a href={docsUrl('ROADMAP.md')}>roadmap</a> and the known gaps. They are not filed as issues yet: open one saying you are taking it, so that work is not duplicated.</p>
            <ul className="plain why-list">
              {TASKS.map((t) => (
                <li key={t.task} className="why-card"><h3>{t.task}</h3><p className="small muted">Needs: {t.needs}</p></li>
              ))}
            </ul>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-setup">
        <div className="wrap narrow">
          <SectionHead n="4" total="" eyebrow="Set up" id="h-setup">Development setup</SectionHead>
          <Reveal>
            <p className="lede">A stable Rust toolchain, Python 3 for the doc helpers, and Node 22 for the website.</p>
            <Code label="Clone, build and test">{SETUP}</Code>
            <p>Before you open a pull request, run what CI runs, plus the checks for what you touched:</p>
            <Code label="Pre-pull-request checks">{CHECKS}</Code>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-pr">
        <div className="wrap narrow">
          <SectionHead n="5" total="" eyebrow="Pull requests" id="h-pr">The flow, and what review looks like</SectionHead>
          <Reveal>
            <ol>
              <li>For anything larger than a typo, open an issue first so we can agree on the direction.</li>
              <li>Fork, branch, and keep the change small and focused.</li>
              <li>A change in behaviour needs a test. Tests build their packages in memory. Examples and fixtures use synthetic data only.</li>
              <li>Run the checks, and add a line to <span className="mono">CHANGELOG.md</span> under Unreleased for user-visible changes.</li>
              <li>Open the pull request using the template. By contributing you agree your work is licensed under the MIT licence.</li>
            </ol>
            <p><strong>In review:</strong> CI must pass (format, clippy with warnings as errors, tests on Linux and macOS, the docs link check). Reviewers check claims first: expect a request to soften a claim or add the command that shows it. Anything that makes the library act on embedded document text needs a design discussion before code. Small changes are usually answered within a few days; a polite ping after a week is fine.</p>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-schema">
        <div className="wrap narrow">
          <SectionHead n="6" total="" eyebrow="Schema" id="h-schema">How to propose a schema change</SectionHead>
          <Reveal>
            <p>Open an issue with the <a href={issue('schema_proposal.yml')}>schema proposal template</a> before writing code. State:</p>
            <ol>
              <li>the part, element or field;</li>
              <li>why native OOXML cannot already say it;</li>
              <li>how an agent reads it, and what an older reader does when it does not know it (new fields are optional and ignorable);</li>
              <li>what happens when its text is hostile (embedded text is data, never authority);</li>
              <li>the evidence it is needed.</li>
            </ol>
            <p>An accepted proposal is marked <strong>proposed</strong> in the schema reference until code and tests exist.</p>
          </Reveal>
        </div>
      </section>

      <section className="scene" aria-labelledby="h-ask">
        <div className="wrap narrow">
          <SectionHead n="7" total="" eyebrow="Questions" id="h-ask">Where to ask</SectionHead>
          <Reveal>
            <ul>
              <li>Questions and ideas: <a href={`${REPO}/discussions`}>GitHub Discussions</a> if enabled, otherwise <a href={`${REPO}/issues`}>an issue</a>.</li>
              <li>Bugs, round-trip reports and docs problems: issues, using the templates.</li>
              <li>Conduct concerns and security reports <strong>only</strong>: <a href="mailto:agenticoffcie70@gmail.com">agenticoffcie70@gmail.com</a>. Security problems go through <a href={docsUrl('SECURITY.md')}>SECURITY.md</a>, never a public issue. Not for general support.</li>
            </ul>
            <p>Participation is covered by the <a href={docsUrl('CODE_OF_CONDUCT.md')}>Code of Conduct</a>.</p>
            <p className="cta-row"><a className="btn" href={guide()}>Full contribution guide</a><a className="btn btn-quiet" href={`${BASE}why/`}>Why this exists</a><a className="btn btn-quiet" href={BASE}>Back to the overview</a></p>
          </Reveal>
        </div>
      </section>
    </>
  )
}

mount(<Page current="contribute"><Contribute /></Page>)
