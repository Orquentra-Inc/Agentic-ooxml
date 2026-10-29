import { useState } from 'react'
import { Slide } from './Slide'
import { XmlCode } from './XmlCode'
import { SectionHead, Badge } from './Chrome'
import { Reveal } from './Reveal'
import { PARTS } from '../data'

export function PackageScene() {
  const [sel, setSel] = useState('ppt/slides/slide1.xml')
  const part = PARTS.find((p) => p.path === sel)!
  const native = PARTS.filter((p) => p.group === 'native')
  const ao = PARTS.filter((p) => p.group === 'ao')
  const Item = ({ p }: { p: (typeof PARTS)[number] }) => (
    <li>
      <button className={`part ${p.group}`} aria-pressed={sel === p.path} onClick={() => setSel(p.path)}>
        <span className="mono">{p.path}</span>
      </button>
    </li>
  )
  return (
    <section id="package" className="scene" aria-labelledby="h-package">
      <div className="wrap">
        <SectionHead n="01" total="05" eyebrow="The package" id="h-package" lede="A slide you see is the output of a ZIP archive of XML parts, tied together by relationships. That structure is a published standard, and it already has a place for custom data.">
          A rendered slide is a package of parts.
        </SectionHead>
        <Reveal>
          <ol className="flow" aria-label="From slide to parts">
            <li>Rendered slide</li><li aria-hidden="true" className="flow-arrow">→</li>
            <li>Open Packaging Conventions ZIP (.pptx)</li><li aria-hidden="true" className="flow-arrow">→</li>
            <li>XML parts and relationships</li>
          </ol>
          <div className="package-grid">
            <figure className="panel">
              <Slide highlight={part.region ? [part.region] : []} title="The rendered slide, with the region that the selected part controls highlighted" />
              <figcaption className="small muted">
                {part.region ? 'Highlighted: the region the selected part drives or describes.' : 'The selected part has no visible region; it wires the package together.'}
              </figcaption>
            </figure>
            <div className="tree panel">
              <h3 className="label">Native parts (ECMA-376, OPC)</h3>
              <ul className="plain">{native.map((p) => <Item key={p.path} p={p} />)}
                <li className="omitted mono small muted">… slideLayouts, slideMasters, docProps, embedded workbook</li></ul>
              <h3 className="label ao-label">Agentic OOXML parts <Badge kind="impl">customXml</Badge></h3>
              <ul className="plain">{ao.map((p) => <Item key={p.path} p={p} />)}</ul>
              <p className="small muted tree-note">
                File names under customXml/ are conventions. A reader discovers these parts through the customXml
                relationship plus the Agentic Office namespace, so a renamed part is still found.
              </p>
            </div>
            <div className="panel codepane">
              <p className="mono small path">{part.path} {part.group === 'ao' && <Badge kind="impl">added by Agentic OOXML</Badge>}</p>
              <p className="small">{part.role}</p>
              <XmlCode code={part.snippet} label={`Excerpt of ${part.path}`} />
              <p className="small muted">Excerpts are abridged and synthetic. Relationship Type values are shortened where marked with "...".</p>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  )
}
