import { useState } from 'react'
import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { Slide, regionLabel } from './Slide'
import { XmlCode } from './XmlCode'
import { Tabs } from './Tabs'
import { SectionHead, Badge } from './Chrome'
import { Reveal } from './Reveal'
import { LAYERS, type LayerId } from '../data'

export function LayersScene() {
  const [id, setId] = useState<LayerId>('intent')
  const reduce = useReducedMotion()
  const layer = LAYERS.find((l) => l.id === id)!
  return (
    <section id="inspector" className="scene" aria-labelledby="h-layers">
      <div className="wrap">
        <SectionHead n="02" total="05" eyebrow="The layers" id="h-layers" lede="Four small XML parts, each answering a different question for the next agent to open the file. A fifth, the requirement contract, arrives in the v0.2 context schema.">
          Agents, design, intent, sources.
        </SectionHead>
        <Reveal>
          <p className="sim-note"><Badge kind="warn">Simulation</Badge> Browser-only educational simulation. It shows a synthetic slide and the XML these parts hold; it does not open or parse any file, and nothing leaves this page.</p>
          <Tabs
            tabs={LAYERS.map((l) => ({ id: l.id, label: l.label }))}
            active={id}
            onChange={(v) => setId(v as LayerId)}
            prefix="layers"
            label="Agentic OOXML layers"
          >
            <div className="inspector">
              <figure className="panel">
                <Slide highlight={layer.targets} title="Synthetic board-deck slide; highlighted region is what the selected layer concerns" />
                <figcaption className="small muted">Highlighted: {layer.targets.map(regionLabel).join(', ').toLowerCase()}.</figcaption>
              </figure>
              <AnimatePresence mode="wait" initial={false}>
                <motion.div
                  key={layer.id}
                  className="inspector-detail"
                  initial={reduce ? false : { opacity: 0, x: 8 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={reduce ? undefined : { opacity: 0 }}
                  transition={{ duration: 0.18 }}
                >
                  <p className="status-line">
                    <Badge kind={layer.id === 'spec' ? 'wip' : 'impl'}>{layer.status}</Badge>
                    <span className="small muted">{layer.statusNote}</span>
                  </p>
                  <h3>{layer.blurb}</h3>
                  <p className="mono small path">{layer.part}</p>
                  <XmlCode code={layer.xml} label={`${layer.label} XML fragment`} mark={layer.id === 'sources' ? 'claimHash' : undefined} />
                  {layer.caveat && <p className="small muted">{layer.caveat}</p>}
                  <h4 className="label">What an agent does with it</h4>
                  <ul className="does">{layer.does.map((d) => <li key={d}>{d}</li>)}</ul>
                </motion.div>
              </AnimatePresence>
            </div>
          </Tabs>
          <p className="small muted fine">
            The claim hash shown in the Sources tab is a real BLAKE3 digest of the KPI shape's XML as listed in the package
            scene, computed in your browser. Embedded text is document data, not instructions.
          </p>
        </Reveal>
      </div>
    </section>
  )
}
