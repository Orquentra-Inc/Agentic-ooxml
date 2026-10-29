import { motion, useReducedMotion } from 'motion/react'
import { FIGURES, TOTAL, type Region } from '../data'

const R: Record<Region, { x: number; y: number; w: number; h: number; label: string }> = {
  slide: { x: 2, y: 2, w: 636, h: 356, label: 'Whole slide' },
  title: { x: 28, y: 26, w: 330, h: 64, label: 'Title' },
  kpi: { x: 404, y: 38, w: 208, h: 96, label: 'KPI shape, id 12' },
  chart: { x: 28, y: 118, w: 350, h: 190, label: 'Chart' },
  footer: { x: 28, y: 318, w: 584, h: 28, label: 'Footnote' },
}

export type KpiState = 'verified' | 'stale' | 'none'

interface Props {
  highlight?: Region[]
  kpiText?: string
  kpiStatus?: KpiState
  title?: string
}

/** A synthetic board-deck slide drawn as inline SVG. All figures are invented for illustration. */
export function Slide({ highlight = [], kpiText = 'EUR 18.6M', kpiStatus = 'none', title = 'Slide 1 of a synthetic board deck' }: Props) {
  const reduce = useReducedMotion()
  const max = 24
  const bw = 52
  const gap = 28
  const x0 = 64
  const base = 290
  const scale = 140 / max
  const on = (r: Region) => highlight.includes(r)
  return (
    <svg className="slide" viewBox="0 0 640 360" role="img" aria-label={`${title}. Title: Q3 revenue accelerates. KPI: ${kpiText}. Bars for Q1 17.2, Q2 17.9, Q3 18.6, and Q4 forecast 21.3 EUR million.`}>
      <rect x="0" y="0" width="640" height="360" fill="var(--slide-bg)" />
      <text x="36" y="58" className="s-title">Q3 revenue accelerates</text>
      <text x="36" y="80" className="s-sub">EUR M per quarter. Q4 is a forecast.</text>

      <g>
        <rect x="404" y="38" width="208" height="96" fill="none" stroke="var(--line2)" />
        <text x="420" y="66" className="s-cap">Q3 REVENUE</text>
        <text x="420" y="110" className="s-kpi">{kpiText}</text>
        {kpiStatus !== 'none' && (
          <g>
            <rect x="404" y="140" width={kpiStatus === 'stale' ? 96 : 104} height="22" fill={kpiStatus === 'stale' ? 'var(--warn)' : 'none'} stroke={kpiStatus === 'stale' ? 'var(--warn)' : 'var(--cobalt-text)'} />
            <text x="414" y="155" className="s-badge" fill={kpiStatus === 'stale' ? '#14100a' : 'var(--cobalt-text)'}>{kpiStatus === 'stale' ? 'STALE' : 'VERIFIED'}</text>
          </g>
        )}
      </g>

      <line x1="48" y1={base} x2="372" y2={base} stroke="var(--line2)" />
      {FIGURES.map((f, i) => {
        const h = f.v * scale
        const x = x0 + i * (bw + gap)
        return (
          <g key={f.q}>
            <rect x={x} y={base - h} width={bw} height={h} fill={f.forecast ? 'none' : i === 2 ? 'var(--cobalt)' : 'var(--graphite-2)'} stroke={f.forecast ? 'var(--cobalt-text)' : 'none'} strokeDasharray={f.forecast ? '4 3' : undefined} />
            <text x={x + bw / 2} y={base - h - 8} textAnchor="middle" className="s-val">{f.v.toFixed(1)}</text>
            <text x={x + bw / 2} y={base + 16} textAnchor="middle" className="s-axis">{f.q}</text>
          </g>
        )
      })}
      <text x="36" y="338" className="s-foot">FY total {TOTAL.toFixed(1)} includes the Q4 forecast. Synthetic figures.</text>

      {(Object.keys(R) as Region[]).filter(on).map((k) => {
        const r = R[k]
        return (
          <motion.rect
            key={k}
            x={r.x} y={r.y} width={r.w} height={r.h}
            fill="var(--cobalt-glow)" stroke="var(--cobalt-text)" strokeWidth="1.5"
            initial={reduce ? false : { opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.25 }}
          />
        )
      })}
    </svg>
  )
}

export const regionLabel = (r: Region) => R[r].label
