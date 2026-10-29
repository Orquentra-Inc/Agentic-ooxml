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

const canon = (t: string) => { const n = Number(t); return t.trim() !== '' && Number.isFinite(n) ? String(n) : t.trim() }

/** The loop from examples/demo-output.txt: edit the workbook, see the deck go stale, update the slide, reconcile, re-verify. */
function WorkbookLoop() {
  const cellId = useId()
  const [cell, setCell] = useState('18.6')
  const [slide, setSlide] = useState('18.6')
  const [base, setBase] = useState({ cell: '18.6', slide: '18.6' })
  const [stored, setStored] = useState<'verified' | 'stale'>('verified')
  const value = canon(cell)
  const baseHash = useMemo(() => claimHash(base.cell), [base.cell])
  const hash = useMemo(() => claimHash(value), [value])
  const externalChanged = hash !== baseHash
  const slideChanged = slide !== base.slide
  const reasons = [...(externalChanged ? ['external_claim_changed'] : []), ...(slideChanged ? ['claim_changed'] : [])]
  const effective = stored === 'stale' || reasons.length > 0 ? 'stale' : 'verified'
  const slideMatches = slide === value
  const vals = [17.2, 17.9, Number.isFinite(Number(value)) && value !== '' ? Number(value) : 18.6, 21.3]
  const total = vals.reduce((a, b) => a + b, 0)
  const reset = () => { setCell('18.6'); setSlide('18.6'); setBase({ cell: '18.6', slide: '18.6' }); setStored('verified') }
  let say = 'Deck and workbook agree. Change the Q3 cell.'
  if (stored === 'stale') say = 'Reconcile wrote status="stale" into the deck file, so the stored status says stale too.'
  else if (effective === 'verified' && base.cell !== '18.6') say = 'Verified again. The re-verify re-read the cell and took a new baseline.'
  else if (externalChanged && !slideChanged) say = 'The deck was not touched, and its source is already stale: the workbook cell moved.'
  else if (slideChanged && slideMatches) say = 'The slide matches the workbook now. The record stays stale until it is re-verified.'
  else if (slideChanged) say = 'The slide shape changed, so the claim hash no longer matches either.'
  return (
    <div className="stale-grid">
      <div className="panel">
        <p className="label">financial-model.xlsx, synthetic (EUR M)</p>
        <table className="sheet mono">
          <caption className="sr-only">Quarterly revenue in EUR millions</caption>
          <thead><tr>{FIGURES.map((f) => <th key={f.q} scope="col">{f.q}</th>)}<th scope="col">Total</th></tr></thead>
          <tbody><tr>
            {FIGURES.map((f, i) => (
              <td key={f.q}>{f.q === 'Q3' ? <input id={cellId} aria-label="Q3 revenue in the workbook, EUR million" className="input mono cell" inputMode="decimal" value={cell} onChange={(e) => setCell(e.target.value)} spellCheck={false} autoComplete="off" /> : vals[i].toFixed(1)}</td>
            ))}
            <td>{total.toFixed(1)}</td>
          </tr></tbody>
        </table>
        <p className="small muted">Try 19.1, the value in the real demo. Edit the cell; the deck file is untouched.</p>
        <Slide kpiText={`EUR ${slide}M`} kpiStatus={effective} title={`Slide reading EUR ${slide}M, effective status ${effective}`} />
        <div className="loop-actions">
          <button type="button" className="btn btn-quiet" disabled={slideMatches} onClick={() => setSlide(value)}>1. Agent updates the slide</button>
          <button type="button" className="btn btn-quiet" disabled={reasons.length === 0 || stored === 'stale'} onClick={() => setStored('stale')}>2. Run reconcile</button>
          <button type="button" className="btn" disabled={effective === 'verified' || !slideMatches} onClick={() => { setBase({ cell: value, slide }); setStored('verified') }}>3. Re-verify</button>
          <button type="button" className="btn btn-quiet" onClick={reset}>Reset</button>
        </div>
        <p className="small muted">Re-verify is offered once the slide shows the workbook value; the library itself would accept it any time.</p>
      </div>
      <div className="panel">
        <h3 className="label">Source record q3-kpi, external claim</h3>
        <dl className="kv mono">
          <dt>file</dt><dd>financial-model.xlsx</dd>
          <dt>locator</dt><dd>Summary!B1</dd>
          <dt>recorded value hash</dt><dd>{shortHash(baseHash)}</dd>
          <dt>current value hash</dt><dd className={externalChanged ? 'warn' : ''}>{shortHash(hash)}</dd>
          <dt>stored status</dt><dd>{stored}</dd>
          <dt>effective status</dt><dd className={effective === 'stale' ? 'warn' : 'ok'}>{effective}{reasons.length ? ` (${reasons.join(', ')})` : stored === 'stale' ? ' (recorded as stale)' : ''}</dd>
        </dl>
        <p className="loop-say" role="status">{say}</p>
        <p><Badge kind="impl">Implemented</Badge> <span className="small">
          The crate opens the workbook beside the deck, read-only, hashes the cell's cached value and reports the source stale when it moves. It reads cached values and does not recalculate formulas.
        </span></p>
        <div className="proposed">
          <p><Badge kind="prop">Proposed, not implemented</Badge></p>
          <p className="small">Editing the slide for you. Today the file tells the agent the number is out of date; the agent makes the edit, and writing the source back to verified takes a new baseline.</p>
        </div>
      </div>
    </div>
  )
}

export function StaleScene() {
  const [mode, setMode] = useState('workbook')
  return (
    <section id="proof" className="scene" aria-labelledby="h-stale">
      <div className="wrap">
        <SectionHead n="04" total="12" eyebrow="Proof" id="h-stale" lede="Change the workbook cell below. The deck is never edited, and its source still goes stale. Then walk the loop: update the slide, reconcile, re-verify. The hash is real BLAKE3 and the values are the ones in the recorded demo.">
          The file notices when a number moves.
        </SectionHead>
        <Reveal>
          <p className="sim-note"><Badge kind="warn">Simulation</Badge> Browser-only, over synthetic data. The behaviour matches the Rust crate's output in examples/demo-output.txt.</p>
          <Tabs tabs={[{ id: 'workbook', label: 'Edit the workbook' }, { id: 'slide', label: 'Edit the slide' }]} active={mode} onChange={setMode} prefix="stale" label="Which side changes">
            {mode === 'workbook' ? <WorkbookLoop /> : <SlideMode />}
          </Tabs>
        </Reveal>
      </div>
    </section>
  )
}
