import { useState } from 'react'
import { SectionHead } from './Chrome'
import { Reveal } from './Reveal'
import { CopyCommand } from './CopyCommand'
import { docsUrl } from '../data'

interface RunStep { id: string; name: string; what: string; cmd: string; out: string; note?: string }

const SETUP = `git clone https://github.com/Orquentra-Inc/Agentic-ooxml && cd Agentic-ooxml
cargo build -q -p agentic-ooxml-cli -p agentic-ooxml-examples
export PATH="$PWD/target/debug:$PATH"
mkdir -p /tmp/ao-try && cp examples/board-deck.pptx examples/financial-model.xlsx examples/report.docx /tmp/ao-try/ && cd /tmp/ao-try`

const STEPS: RunStep[] = [
  {
    id: 'inspect', name: 'Inspect',
    what: 'Open the board deck and see what it carries: four slides, four Agentic OOXML parts, and three source records. The deck says where Q3 revenue comes from.',
    cmd: `agentic-ooxml inspect --file board-deck.pptx
agentic-ooxml sources --file board-deck.pptx`,
    out: `# inspect prints JSON; the demo filters it to these lines
format: pptx | main part: ppt/presentation.xml | entries: 30
  slide 1 id 256 | FY Board Update
  slide 2 id 257 | Revenue by quarter
  slide 3 id 258 | Full-year outlook
  slide 4 id 259 | Risks and asks
  AO part: intent customXml/ao-intent.xml | sections: ['purpose', 'audience', 'slide-2'] | sources: None
  AO part: sources customXml/ao-sources.xml | sections: [] | sources: 3
  ...

# sources, trimmed
{
  "sources": [
    {
      "id": "q3-kpi",
      "status": "verified",
      "resource_uri": "financial-model.xlsx",
      "location": "Summary!B1",
      "claim_part": "/ppt/slides/slide2.xml",
      "claim_object_id": "5",
      ...
      "effective_status": "verified",
      "external_check": {
        "current_value": "18.6",
        ...
      }
    },
    ...`,
  },
  {
    id: 'edit', name: 'Edit',
    what: 'Someone corrects Q3 in the workbook: 18.6 becomes 19.1. The deck is not opened.',
    cmd: `mkdir before && cp board-deck.pptx financial-model.xlsx before/
make-examples set-text --file financial-model.xlsx --part xl/worksheets/sheet1.xml --find "<v>18.6</v>" --replace "<v>19.1</v>"
make-examples set-text --file financial-model.xlsx --part xl/worksheets/sheet2.xml --find "<v>18.6</v>" --replace "<v>19.1</v>"`,
    out: `financial-model.xlsx: 1 replacement(s) in xl/worksheets/sheet1.xml
financial-model.xlsx: 1 replacement(s) in xl/worksheets/sheet2.xml

# the deck was not touched; slide 2 still says:
   Q3 revenue EUR 18.6M`,
    note: 'The library does not edit cells. make-examples set-text is a demo helper that replaces text inside one package part.',
  },
  {
    id: 'stale', name: 'Stale',
    what: 'Read the deck again. Its Q3 source is stale because the workbook cell moved. validate still exits 0: a stale source is a warning, not corruption.',
    cmd: `agentic-ooxml sources --file board-deck.pptx
agentic-ooxml validate --file board-deck.pptx`,
    out: `# sources, trimmed
      "id": "q3-kpi",
      "status": "verified",
      ...
      "effective_status": "stale",
      "stale_reason": "external_claim_changed",
      ...
      "external_check": {
        "reason": "external_claim_changed",
        "current_value": "19.1",
        ...
      }

# validate, trimmed
{
  "ok": true,
  "errors": [],
  "warnings": [
    {
      "code": "sources.stale",
      "part": "q3-kpi",
      "message": "source 'q3-kpi': verified, but now stale (external_claim_changed)"
    }
  ],
  ...
}`,
  },
  {
    id: 'diff', name: 'Diff',
    what: 'The agent updates slide 2, then compares the new deck with the old one. The report lists the part whose bytes changed and the source whose status moved.',
    cmd: `make-examples set-text --file board-deck.pptx --part ppt/slides/slide2.xml --find "EUR 18.6M" --replace "EUR 19.1M"
agentic-ooxml diff --before before/board-deck.pptx --after board-deck.pptx`,
    out: `# diff, trimmed
{
  "identical": false,
  "sources": [
    {
      "id": "q3-kpi",
      "change": "changed",
      "before": { "effective_status": "verified", ... },
      "after":  { "effective_status": "stale",
                  "stale_reason": "external_claim_changed", ... }
    }
  ],
  "native_parts": {
    "added": [],
    "removed": [],
    "changed": [
      {
        "part": "ppt/slides/slide2.xml",
        "before": "blake3:3a47555e...",
        "after": "blake3:af06780a..."
      }
    ]
  },
  "package_plumbing": { "added": [], "removed": [], "changed": [] }
}`,
    note: 'In the recorded output the before and after objects are expanded over many lines; they are condensed here.',
  },
  {
    id: 'verify', name: 'Verify',
    what: 'Record the finding, then re-verify. The write re-reads the workbook cell and takes 19.1 as the new baseline. The final validate is clean.',
    cmd: `agentic-ooxml reconcile --file board-deck.pptx --dry-run
agentic-ooxml reconcile --file board-deck.pptx
echo '{"sources":[{"id":"q3-kpi","status":"verified"}]}' > verify.json
agentic-ooxml write --file board-deck.pptx --kind sources --input verify.json
agentic-ooxml validate --file board-deck.pptx`,
    out: `# reconcile --dry-run
{
  "file": "board-deck.pptx",
  "invalidated": [ { "id": "q3-kpi", "reason": "external_claim_changed" } ],
  "unchanged": 2,
  "written": false
}
# reconcile: the same, with "written": true

# write
{
  "part": "customXml/ao-sources.xml",
  "created": false,
  ...
}

# validate, trimmed
{
  "ok": true,
  "errors": [],
  "warnings": [],
  ...
}`,
    note: 'The recorded transcript prints the reconcile object over several lines. validate checks structure, not that Word, PowerPoint or Excel will open the file.',
  },
]

export function RunScene() {
  const [i, setI] = useState(2)
  const s = STEPS[i]
  return (
    <section id="run" className="scene" aria-labelledby="h-run">
      <div className="wrap">
        <SectionHead n="06" total="12" eyebrow="See it run" id="h-run" lede={<>This is a recorded session on the synthetic examples, trimmed for length. Nothing here is simulated: the commands and output come from <a href={docsUrl('examples/demo-output.txt')}>examples/demo-output.txt</a>, produced by <a href={docsUrl('scripts/demo.sh')}>scripts/demo.sh</a>. The examples have not been opened in Office.</>}>
          Five steps, from inspect to verified.
        </SectionHead>
        <Reveal>
          <ol className="steps run-steps" aria-label="Demo steps">
            {STEPS.map((st, k) => (
              <li key={st.id}>
                <button type="button" className="step" aria-pressed={k === i} aria-controls="run-detail" onClick={() => setI(k)}>
                  <span className="mono small">{String(k + 1).padStart(2, '0')}</span>
                  <span className="step-name">{st.name}</span>
                </button>
              </li>
            ))}
          </ol>
          <div className="panel run-detail" id="run-detail">
            <h3>{i + 1}. {s.name}</h3>
            <p>{s.what}</p>
            <p className="label">Commands</p>
            <CopyCommand text={s.cmd} label={`commands for ${s.name}`} />
            <p className="label">Output</p>
            <pre className="code run-out" tabIndex={0} aria-label={`Output of ${s.name}, trimmed`}><code>{s.out}</code></pre>
            {s.note && <p className="small muted">{s.note}</p>}
          </div>
          <details className="panel setup">
            <summary>Run it yourself: setup (needs a Rust toolchain)</summary>
            <p className="small muted">Builds the CLI and the demo helper from a checkout, then works on a copy so the committed examples stay untouched. Or run <span className="mono">scripts/demo.sh</span> for the whole session at once.</p>
            <CopyCommand text={SETUP} label="setup commands" />
            <p className="small muted">Each step assumes you are in <span className="mono">/tmp/ao-try</span> with both binaries on your PATH.</p>
          </details>
        </Reveal>
      </div>
    </section>
  )
}
