import { motion, useReducedMotion } from 'motion/react'
import { Slide } from './Slide'
import { Badge } from './Chrome'
import { CopyCommand } from './CopyCommand'
import { BASE, INSTALL_CMD, docsUrl } from '../data'

const QUESTIONS = [
  { x: '6%', y: '4%', t: 'Who is this for?' },
  { x: 'auto', r: '3%', y: '5%', t: 'Where does 18.6 come from?' },
  { x: '60%', y: '58%', t: 'What must not change?' },
]

export function Hero() {
  const reduce = useReducedMotion()
  return (
    <section className="hero" aria-labelledby="h-hero">
      <div className="wrap hero-grid">
        <div className="hero-copy">
          <p className="eyebrow"><span className="rule" aria-hidden="true" />Agentic OOXML</p>
          <h1 id="h-hero">Let an agent edit the board deck.<br /><em>Know when a number moved.</em></h1>
          <p className="lede">
            Who the deck is for, where each figure came from and what must not change are stored inside the .pptx, .docx or .xlsx itself.
            The next edit starts with that context, and a number that moved gets flagged instead of silently trusted.
          </p>
          <div className="cta-row">
            <a className="btn" href="#install">Install in one command</a>
            <a className="btn btn-quiet" href="#proof">See it catch a stale number</a>
          </div>
          <CopyCommand wrap text={INSTALL_CMD} label="install command for Linux and macOS" />
          <p className="small muted hero-note">
            Linux and macOS, no Rust needed. Windows, or if you prefer Cargo: <a href="#install">other ways to install</a>.
            Read the <a href={docsUrl('scripts/install.sh')}>script</a> first if you like.
          </p>
          <p className="status-line">
            <Badge kind="impl">MIT</Badge>
            <span>Open source. MCP server and CLI tested; Claude Code plugin installs from the marketplace; Codex, Cursor and Pi experimental. <a href={`${BASE}why/`}>What is and is not proven</a>.</span>
          </p>
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
            Synthetic slide. The file stores the shapes and the numbers. These questions, and two more, are answered nowhere in it.
          </figcaption>
        </figure>
      </div>
    </section>
  )
}
