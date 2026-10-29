import { SectionHead } from './Chrome'
import { Reveal } from './Reveal'

type K = 'impl' | 'wip' | 'prop'
const Node = ({ k, title, children }: { k: K; title: string; children?: string }) => (
  <li className={`node node-${k}`}>
    <span className="node-title">{title}</span>
    {children && <span className="node-body small">{children}</span>}
    <span className="node-state small mono">{k === 'impl' ? 'implemented' : k === 'wip' ? 'built, not verified end to end' : 'proposed'}</span>
  </li>
)

export function ArchScene() {
  return (
    <section id="architecture" className="scene" aria-labelledby="h-arch">
      <div className="wrap">
        <SectionHead n="04" total="05" eyebrow="Architecture" id="h-arch" lede="Native OOXML stays authoritative. A small trusted core reads and writes the extra parts; everything agent-facing is a thin layer over it.">
          What exists, what is coming, what is only proposed.
        </SectionHead>
        <Reveal>
          <ul className="legend" aria-label="Legend">
            <li><i className="sw sw-impl" />Implemented and tested in the Rust crate</li>
            <li><i className="sw sw-wip" />Built, but not verified end to end</li>
            <li><i className="sw sw-prop" />Proposed in the spec, not built</li>
          </ul>
          <div className="arch">
            <div className="arch-col">
              <h3 className="label">Agent hosts</h3>
              <ul className="plain nodes">
                <Node k="wip" title="Claude Code plugin and skills">Skills only for what the code backs.</Node>
                <Node k="impl" title="MCP server, stdio">Read tools, plus write tools that refuse by default.</Node>
                <Node k="prop" title="Other agent adapters">No other host is implemented.</Node>
              </ul>
              <div className="conn" aria-hidden="true" />
              <h3 className="label">Interfaces</h3>
              <ul className="plain nodes">
                <Node k="impl" title="CLI: list, read, write, sources, reconcile">JSON on stdout.</Node>
                <Node k="impl" title="CLI: inspect, validate, diff">Structural reports.</Node>
              </ul>
              <div className="conn" aria-hidden="true" />
              <h3 className="label">Rust library: agentic-ooxml</h3>
              <ul className="plain nodes">
                <Node k="impl" title="Discovery">By customXml relationship and namespace, not by file name.</Node>
                <Node k="impl" title="Knowledge read, write, merge">agents, design, intent, sources.</Node>
                <Node k="impl" title="Claim hash and reconcile">Effective status, stale on change.</Node>
                <Node k="impl" title="Safety and atomic rewrite">Path, size, DOCTYPE and CDATA limits; native entries byte-identical.</Node>
                <Node k="impl" title="v0.2 context reader">doc, units, spec. Read-only here.</Node>
              </ul>
              <div className="conn" aria-hidden="true" />
              <h3 className="label">The file</h3>
              <ul className="plain nodes">
                <Node k="impl" title=".pptx, .docx, .xlsx package">Native parts plus customXml parts. Ordinary Office content untouched.</Node>
              </ul>
            </div>
            <aside className="arch-col arch-prop" aria-label="Proposed components">
              <h3 className="label">Proposed, beside the core</h3>
              <ul className="plain nodes">
                <Node k="prop" title="Derived SQLite and FTS index">Disposable, rebuilt from the file.</Node>
                <Node k="prop" title="Semantic diff classifier">Visual, text, data, formula, structural.</Node>
                <Node k="prop" title="Automatic cross-file edits">Proposing the slide edit when a workbook value changes. Detection is implemented; the edit is not.</Node>
                <Node k="prop" title="Typed design tokens, agent registry">Today both are free text.</Node>
                <Node k="prop" title="Round-trip retention matrix">PowerPoint, LibreOffice, ONLYOFFICE, Google. Untested.</Node>
              </ul>
            </aside>
          </div>
        </Reveal>
      </div>
    </section>
  )
}
