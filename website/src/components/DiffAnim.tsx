import { useEffect, useRef, useState } from 'react'
import { motion, useInView, useReducedMotion } from 'motion/react'
import { Badge } from './Chrome'

const ROWS = [
  { part: 'native_parts.changed: ppt/slides/slide2.xml', kind: 'native', before: 'blake3:3a47555e…', after: 'blake3:af06780a…', changed: true, note: 'hash changed' },
  { part: 'native_parts.added, removed', kind: 'native', before: '[ ]', after: '[ ]', changed: false, note: '' },
  { part: 'package_plumbing', kind: 'native', before: '[ ]', after: '[ ]', changed: false, note: '' },
  { part: 'sources q3-kpi', kind: 'ao', before: 'effective_status: verified', after: 'effective_status: stale (external_claim_changed)', changed: true, note: '' },
] as const

/** Before/after structural diff. Plays once on entering view; replay on demand. Static when motion is reduced. */
export function DiffAnim() {
  const ref = useRef<HTMLDivElement>(null)
  const inView = useInView(ref, { once: true, margin: '0px 0px -20% 0px' })
  const reduce = useReducedMotion()
  const [run, setRun] = useState(0)
  const [phase, setPhase] = useState(0) // 0 before, 1 after
  useEffect(() => {
    if (reduce) { setPhase(1); return }
    if (!inView) return
    setPhase(0)
    const t = setTimeout(() => setPhase(1), 900)
    return () => clearTimeout(t)
  }, [inView, run, reduce])
  return (
    <div className="diff panel" ref={ref}>
      <div className="diff-head">
        <h3>agentic-ooxml diff: before and after one edit</h3>
        <button className="btn btn-quiet" onClick={() => setRun((r) => r + 1)} disabled={reduce === true}>Replay</button>
      </div>
      <p className="small muted"><Badge kind="impl">Real output, condensed</Badge> From step 7 of the recorded demo: the deck after the slide was updated, compared with the deck before. Parts that did not change are not listed by the command.</p>
      <div className="diff-table" role="table" aria-label="Structural diff">
        <div className="diff-row diff-cols mono small muted" role="row"><span role="columnheader">Field in the report</span><span role="columnheader">Before</span><span role="columnheader">After</span></div>
        {ROWS.map((r, k) => (
          <motion.div
            key={r.part + run}
            role="row"
            className={`diff-row ${r.kind} ${r.changed && phase === 1 ? 'changed' : ''}`}
            initial={reduce ? false : { opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: reduce ? 0 : 0.08 * k, duration: 0.3 }}
          >
            <span role="cell" className="mono">{r.part}</span>
            <span role="cell" className="mono small">{r.before}</span>
            <span role="cell" className="mono small">
              {phase === 1 ? <>{r.after} <em className="diff-note">{r.changed && r.note ? `(${r.note})` : ''}</em></> : <span className="muted">pending</span>}
            </span>
          </motion.div>
        ))}
      </div>
    </div>
  )
}
