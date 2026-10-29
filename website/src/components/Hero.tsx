import { motion, useReducedMotion } from 'motion/react'
import { Slide } from './Slide'
import { Badge } from './Chrome'
import { BASE, REPO } from '../data'

const QUESTIONS = [
  { x: '6%', y: '4%', t: 'Who is this for?' },
  { x: 'auto', r: '3%', y: '5%', t: 'Where does 18.6 come from?' },
  { x: '38%', y: '86%', t: 'What must not change?' },
]

export function Hero() {
  const reduce = useReducedMotion()
  return (
    <section className="hero" aria-labelledby="h-hero">
      <div className="wrap hero-grid">
        <div className="hero-copy">
          <p className="eyebrow"><span className="mono">01 / 08</span><span className="rule" aria-hidden="true" />Agentic OOXML</p>
          <h1 id="h-hero">AI can write documents.<br /><em>But does it understand them?</em></h1>
          <p className="lede">
            A .pptx file records where every shape sits. It does not record who the deck is for, where a number came from,
            or what a reviewer already rejected. Agentic OOXML is a proposed open convention for carrying that context
            inside the file, plus a Rust reference implementation that reads and writes it.
          </p>
          <div className="cta-row">
            <a className="btn" href="#package">See how it works</a>
            <a className="btn btn-quiet" href={`${BASE}why/`}>Why this exists</a>
          </div>
          <p className="status-line"><Badge kind="wip">Early release</Badge> Version 0.1. Open source (MIT). Not on crates.io or npm. Office and LibreOffice round trips are unmeasured.</p>
        </div>
        <figure className="hero-figure">
          <div className="hero-stage">
            <Slide title="A synthetic board-deck slide with three unanswered questions" />
            {QUESTIONS.map((q, i) => (
              <motion.span
                key={q.t}
                className="qtag"
                style={{ left: q.x, right: 'r' in q ? q.r : undefined, top: q.y }}
                initial={reduce ? false : { opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.7 + i * 0.5, duration: 0.5 }}
              >
                {q.t}
              </motion.span>
            ))}
            {!reduce && <motion.div className="scan" aria-hidden="true" initial={{ left: '0%', opacity: 1 }} animate={{ left: '100%', opacity: 0 }} transition={{ duration: 1.6, ease: 'easeInOut' }} />}
          </div>
          <figcaption className="small muted">
            Synthetic slide. The file stores the shapes and the numbers; the three questions above are answered nowhere in it.
            See the <a href={REPO}>repository</a> for the schema.
          </figcaption>
        </figure>
      </div>
    </section>
  )
}
