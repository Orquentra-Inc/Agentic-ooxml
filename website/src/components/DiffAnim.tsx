import { useEffect, useRef, useState } from 'react'
import { motion, useInView, useReducedMotion } from 'motion/react'
import { Badge } from './Chrome'

const ROWS = [
  { part: 'ppt/slides/slide1.xml', kind: 'native', before: 'EUR 18.6M', after: 'EUR 19.4M', changed: true, note: 'bytes changed' },
  { part: 'ppt/charts/chart1.xml', kind: 'native', before: 'unchanged', after: 'unchanged', changed: false, note: 'byte-identical' },
  { part: 'ppt/theme/theme1.xml', kind: 'native', before: 'unchanged', after: 'unchanged', changed: false, note: 'byte-identical' },
  { part: 'customXml/intent.xml', kind: 'ao', before: 'unchanged', after: 'unchanged', changed: false, note: 'byte-identical' },
  { part: 'customXml/sources.xml', kind: 'ao', before: 'src-q3-revenue: verified', after: 'src-q3-revenue: stale', changed: true, note: 'status changed' },
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
        <h3>Structural diff: before and after one edit</h3>
        <button className="btn btn-quiet" onClick={() => setRun((r) => r + 1)} disabled={reduce === true}>Replay</button>
      </div>
      <p className="small muted"><Badge kind="warn">Illustrative</Badge> The kind of report the planned <span className="mono">diff</span> command is meant to give. Values are synthetic.</p>
      <div className="diff-table" role="table" aria-label="Structural diff">
        <div className="diff-row diff-cols mono small muted" role="row"><span role="columnheader">Part</span><span role="columnheader">Before</span><span role="columnheader">After</span></div>
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
              {phase === 1 ? <>{r.after} <em className="diff-note">{r.changed ? `(${r.note})` : ''}</em></> : <span className="muted">pending</span>}
            </span>
          </motion.div>
        ))}
      </div>
    </div>
  )
}
