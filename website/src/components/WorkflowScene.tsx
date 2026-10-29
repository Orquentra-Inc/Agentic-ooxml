import { useState } from 'react'
import { SectionHead, Badge } from './Chrome'
import { Reveal } from './Reveal'

type ToolState = 'lib' | 'new'
interface Step { id: string; name: string; summary: string; tools: { n: string; s: ToolState }[]; detail: string[]; note?: string }

const STEPS: Step[] = [
  { id: 'inspect', name: 'Inspect', summary: 'Map the package cheaply.', tools: [{ n: 'ao_list', s: 'lib' }, { n: 'ao_inspect', s: 'new' }],
    detail: ['List which Agentic OOXML parts the file carries, with versions and summaries.', 'Report the format, main part and native units (slides, sheets, sections) without loading their content.'] },
  { id: 'interpret', name: 'Interpret', summary: 'Load only the guidance that applies.', tools: [{ n: 'ao_read', s: 'lib' }, { n: 'ao_sources', s: 'lib' }],
    detail: ['Read the intent section for the slide, the design conventions for the chart, and the source record for the shape.', 'Every result is marked trust: document. It is data about the file, not instructions to follow.'] },
  { id: 'plan', name: 'Plan', summary: 'Decide the edit. No tool call.', tools: [],
    detail: ['The agent plans with the user. Order of authority: application policy, the explicit request, trusted skills, then embedded document preferences.', 'A rule inside the file can narrow what an agent does. It can never widen what it is allowed to do.'] },
  { id: 'modify', name: 'Modify', summary: 'Change content, then metadata.', tools: [{ n: 'ao_write', s: 'lib' }],
    detail: ['Slide content is edited with whichever Office editing tool the agent already uses. Agentic OOXML writes only its own parts.', 'Write tools refuse unless AGENTIC_OOXML_ALLOW_WRITE=1, stay inside AGENTIC_OOXML_ROOTS, and by default produce a sibling .ao-proposed copy instead of touching the original.'],
    note: 'Native entries in the package are rewritten byte-identical. That is tested in the crate.' },
  { id: 'validate', name: 'Validate', summary: 'Check the package and the claims.', tools: [{ n: 'ao_validate', s: 'new' }, { n: 'ao_reconcile', s: 'lib' }],
    detail: ['Check ZIP and XML well-formedness, content types, relationships and that every Agentic OOXML part parses.', 'Re-hash claims and mark changed ones stale. A passing check never means a particular Office version will render the file.'] },
  { id: 'review', name: 'Review', summary: 'Show the human a structural diff.', tools: [{ n: 'ao_diff', s: 'new' }],
    detail: ['Compare the proposed copy with the original: which Agentic OOXML parts changed, which native parts changed or stayed identical, which source statuses moved.', 'The person accepts or discards the proposed copy.'] },
]

export function WorkflowScene() {
  const [i, setI] = useState(3)
  const s = STEPS[i]
  return (
    <section id="workflow" className="scene" aria-labelledby="h-flow">
      <div className="wrap">
        <SectionHead n="03" total="05" eyebrow="The agent workflow" id="h-flow" lede="This is the intended workflow, not a recording of one. The tool names are the MCP surface of the server in this repository, which you can install from source or GitHub Releases.">
          Inspect, interpret, plan, modify, validate, review.
        </SectionHead>
        <Reveal>
          <ol className="steps" aria-label="Workflow steps">
            {STEPS.map((st, k) => (
              <li key={st.id}>
                <button className="step" aria-pressed={k === i} aria-controls="step-detail" onClick={() => setI(k)}>
                  <span className="mono small">{String(k + 1).padStart(2, '0')}</span>
                  <span className="step-name">{st.name}</span>
                </button>
              </li>
            ))}
          </ol>
          <div className="panel step-detail" id="step-detail" aria-live="polite">
            <h3>{s.name}: {s.summary}</h3>
            <div className="chips">
              {s.tools.length === 0 && <span className="chip chip-none">no tool: reasoning step</span>}
              {s.tools.map((t) => (
                <span key={t.n} className={`chip mono ${t.s === 'new' ? 'chip-new' : 'chip-lib'}`}>
                  {t.n}<span className="chip-tag">{t.s === 'lib' ? 'library function exists' : 'added with the MCP server'}</span>
                </span>
              ))}
            </div>
            <ul className="does">{s.detail.map((d) => <li key={d}>{d}</li>)}</ul>
            {s.note && <p className="small muted">{s.note}</p>}
          </div>
          <p className="small muted fine">
            <Badge kind="wip">Not verified with an agent</Badge> Every tool above exists in the Rust crates and is tested over stdio by the repository's own checks. The workflow itself is the intended use: it has not been run end to end with a coding agent, and the integrations are not verified.
          </p>
        </Reveal>
      </div>
    </section>
  )
}
