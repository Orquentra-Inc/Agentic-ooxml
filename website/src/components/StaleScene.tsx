import { useId, useMemo, useState } from 'react'
import { Slide } from './Slide'
import { SectionHead, Badge } from './Chrome'
import { Reveal } from './Reveal'
import { Tabs } from './Tabs'
import { BASELINE_HASH, FIGURES, KPI_BASELINE, claimHash, kpiShape, shortHash } from '../data'

function SlideMode() {
  const inputId = useId()
  const [text, setText] = useState(KPI_BASELINE)
  const [reconciled, setReconciled] = useState(false)
  const hash = useMemo(() => claimHash(kpiShape(text)), [text])
  const changed = hash !== BASELINE_HASH
  const effective = changed ? 'stale' : 'verified'
  const stored = reconciled && changed ? 'stale' : 'verified'
  return (
    <div className="stale-grid">
      <div className="panel">
        <label className="label" htmlFor={inputId}>KPI text on the slide (shape id 12)</label>
        <div className="field-row">
          <input id={inputId} className="input mono" value={text} onChange={(e) => { setText(e.target.value); setReconciled(false) }} spellCheck={false} autoComplete="off" />
          <button className="btn btn-quiet" onClick={() => { setText(KPI_BASELINE); setReconciled(false) }}>Reset</button>
        </div>
        <p className="small muted">Try changing 18.6 to 19.4. Any change to the shape's bytes counts, including formatting inside the shape.</p>
        <Slide kpiText={text} kpiStatus={effective === 'stale' ? 'stale' : 'verified'} title={`Slide with KPI reading ${text}, effective status ${effective}`} />
      </div>
      <div className="panel">
        <h3 className="label">Source record src-q3-revenue</h3>
        <dl className="kv mono">
          <dt>recorded claimHash</dt><dd>{shortHash(BASELINE_HASH)}</dd>
          <dt>current claim hash</dt><dd className={changed ? 'warn' : ''}>{shortHash(hash)}</dd>
          <dt>stored status</dt><dd>{stored}</dd>
          <dt>effective status</dt><dd className={changed ? 'warn' : 'ok'} aria-live="polite">{effective}{changed ? ' (claim_changed)' : ''}</dd>
        </dl>
        <button className="btn" disabled={!changed || reconciled} onClick={() => setReconciled(true)}>Run reconcile</button>
        <p className="small muted">
          Same fields the Rust crate reports from <span className="mono">sources</span>. Reconcile writes status="stale" and keeps the old hash as the baseline.
          A stored "verified" is never trusted on its own.
        </p>
        <p><Badge kind="impl">Implemented</Badge> <span className="small">Claim hash, effective status and reconcile are in the crate and tested.</span></p>
      </div>
    </div>
  )
}

function WorkbookMode() {
  const [q3, setQ3] = useState('18.6')
  const n = Number(q3)
  const valid = q3.trim() !== '' && Number.isFinite(n)
  // The crate hashes the cell's value text, normalised so 75 and 75.0 agree.
  const value = valid ? String(n) : q3.trim()
  const baseline = useMemo(() => claimHash('18.6'), [])
  const hash = useMemo(() => claimHash(value), [value])
  const differs = hash !== baseline
  const vals = [17.2, 17.9, valid ? n : 18.6, 21.3]
  const total = vals.reduce((a, b) => a + b, 0)
  return (
    <div className="stale-grid">
      <div className="panel">
        <p className="label">financial-model.xlsx, synthetic (EUR M)</p>
        <table className="sheet mono">
          <caption className="sr-only">Quarterly revenue in EUR millions</caption>
          <thead><tr>{FIGURES.map((f) => <th key={f.q} scope="col">{f.q}</th>)}<th scope="col">Total</th></tr></thead>
          <tbody><tr>
            {FIGURES.map((f, i) => (
              <td key={f.q}>{f.q === 'Q3' ? <input aria-label="Q3 revenue in the workbook, EUR million" className="input mono cell" inputMode="decimal" value={q3} onChange={(e) => setQ3(e.target.value)} /> : vals[i].toFixed(1)}</td>
            ))}
            <td>{total.toFixed(1)}</td>
          </tr></tbody>
        </table>
        <p className="small muted">Edit Q3. The deck file is untouched, so the slide still reads 18.6.</p>
        <Slide kpiText={KPI_BASELINE} kpiStatus={differs ? 'stale' : 'verified'} title={`Slide still reading 18.6, effective status ${differs ? 'stale' : 'verified'}`} />
      </div>
      <div className="panel">
        <h3 className="label">Source record q3-kpi, external claim</h3>
        <dl className="kv mono">
          <dt>file</dt><dd>financial-model.xlsx</dd>
          <dt>locator</dt><dd>Summary!B1</dd>
          <dt>recorded claimHash</dt><dd>{shortHash(baseline)}</dd>
          <dt>current value hash</dt><dd className={differs ? 'warn' : ''}>{shortHash(hash)}</dd>
          <dt>effective status</dt><dd className={differs ? 'warn' : 'ok'} aria-live="polite">{differs ? 'stale (external_claim_changed)' : 'verified'}</dd>
        </dl>
        <p><Badge kind="impl">Implemented</Badge> <span className="small">
          The crate opens the workbook next to the deck, read-only, hashes the cell's cached value, and reports the source stale when it moves.
          It reads cached values and does not recalculate formulas.
        </span></p>
        <div className="proposed">
          <p><Badge kind="prop">Proposed, not implemented</Badge></p>
          <p className="small">Editing the slide for you. Today the file only tells the agent the KPI is out of date; the agent makes the edit, then writes the source back to
            verified, which re-reads the cell and takes a new baseline.</p>
        </div>
      </div>
    </div>
  )
}

export function StaleScene() {
  const [mode, setMode] = useState('slide')
  return (
    <section id="staleness" className="scene" aria-labelledby="h-stale">
      <div className="wrap">
        <SectionHead n="04" eyebrow="Source staleness" id="h-stale" lede="A source record is only worth having if it can go out of date visibly. The kernel stores a hash of the claim as it was verified, and recomputes it every time the record is read.">
          When a number moves, the file notices.
        </SectionHead>
        <Reveal>
          <p className="sim-note"><Badge kind="warn">Simulation</Badge> Browser-only. The hash is real BLAKE3; the file and workbook are synthetic.</p>
          <Tabs tabs={[{ id: 'slide', label: 'Edit the slide claim' }, { id: 'workbook', label: 'Edit the workbook' }]} active={mode} onChange={setMode} prefix="stale" label="Which side changes">
            {mode === 'slide' ? <SlideMode /> : <WorkbookMode />}
          </Tabs>
        </Reveal>
      </div>
    </section>
  )
}
