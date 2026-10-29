import { SectionHead } from './Chrome'
import { Reveal } from './Reveal'
import { BASE, REPO, docsUrl } from '../data'

export function CloseScene() {
  return (
    <section id="open" className="scene scene-close" aria-labelledby="h-open">
      <div className="wrap">
        <SectionHead n="08" eyebrow="Open source" id="h-open" lede="MIT licensed. The schema, the Rust crate and the examples are meant to be read, questioned and improved. The proposal's claims are hypotheses until the benchmark exists, and it says so.">
          Built to be inspected.
        </SectionHead>
        <Reveal>
          <ul className="close-links">
            <li><a href={REPO}><span>Repository</span><span className="muted small">Source, issues, discussions</span></a></li>
            <li><a href={docsUrl('docs/')}><span>Documentation</span><span className="muted small">Specification, guides, schema reference</span></a></li>
            <li><a href={`${BASE}contribute/`}><span>Contribute</span><span className="muted small">Round-trip results from your Office tools are the most useful gift</span></a></li>
            <li><a href={`${BASE}why/`}><span>Why this exists</span><span className="muted small">What is established, what is new, what is open</span></a></li>
          </ul>
        </Reveal>
      </div>
    </section>
  )
}
