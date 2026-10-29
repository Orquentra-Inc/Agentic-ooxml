import type { ReactNode } from 'react'
import { BASE, REPO, docsUrl } from '../data'

export function Header({ current }: { current: 'home' | 'why' | 'contribute' | 'other' }) {
  return (
    <header className="site-header">
      <a className="skip" href="#main">Skip to content</a>
      <div className="wrap header-row">
        <a className="brand" href={BASE} aria-label="Agentic OOXML, home">
          <svg width="22" height="22" viewBox="0 0 32 32" aria-hidden="true"><rect x="3.5" y="3.5" width="25" height="25" fill="none" stroke="currentColor" strokeWidth="1.6" /><path d="M3.5 21.5h25" stroke="var(--cobalt)" strokeWidth="1.6" /><rect x="9" y="9" width="8" height="8" fill="var(--cobalt)" /></svg>
          <span>Agentic OOXML</span>
        </a>
        <nav aria-label="Primary">
          <ul>
            <li className="hide-sm"><a href={`${BASE}#inspector`}>Inspector</a></li>
            <li className="hide-sm"><a href={`${BASE}#install`}>Install</a></li>
            <li><a href={`${BASE}why/`} aria-current={current === 'why' ? 'page' : undefined}>Why</a></li>
            <li><a href={`${BASE}contribute/`} aria-current={current === 'contribute' ? 'page' : undefined}>Contribute</a></li>
            <li><a href={docsUrl('docs/')}>Docs</a></li>
            <li><a href={REPO} className="btn-ghost">GitHub</a></li>
          </ul>
        </nav>
      </div>
    </header>
  )
}

export function Footer() {
  return (
    <footer className="site-footer">
      <div className="wrap footer-grid">
        <div>
          <p className="brandline">Agentic OOXML</p>
          <p className="small muted">MIT licensed. Early release, version 0.1. Not on crates.io or npm; install from source, GitHub Releases or the Claude Code marketplace.</p>
        </div>
        <div>
          <p className="small label">Project</p>
          <ul className="plain">
            <li><a href={REPO}>Repository</a></li>
            <li><a href={docsUrl('docs/')}>Documentation</a></li>
            <li><a href={`${BASE}contribute/`}>Contribute</a></li>
            <li><a href={docsUrl('LICENSE')}>License (MIT)</a></li>
          </ul>
        </div>
        <div>
          <p className="small label">Notice</p>
          <p className="small muted">
            Agentic OOXML is an application-level convention built on Office Open XML. It is not part of ECMA-376 or
            ISO/IEC 29500, and is not affiliated with or endorsed by Microsoft, Ecma International or ISO.
          </p>
        </div>
      </div>
    </footer>
  )
}

export function Page({ current, children }: { current: 'home' | 'why' | 'contribute' | 'other'; children: ReactNode }) {
  return (
    <>
      <Header current={current} />
      <main id="main" tabIndex={-1}>{children}</main>
      <Footer />
    </>
  )
}

export function SectionHead({ n, total = '08', eyebrow, id, children, lede }: { n: string; total?: string; eyebrow: string; id: string; children: ReactNode; lede?: ReactNode }) {
  return (
    <div className="section-head">
      <p className="eyebrow"><span className="mono">{n}{total ? ` / ${total}` : ''}</span><span className="rule" aria-hidden="true" />{eyebrow}</p>
      <h2 id={id}>{children}</h2>
      {lede && <p className="lede">{lede}</p>}
    </div>
  )
}

export function Badge({ kind, children }: { kind: 'impl' | 'wip' | 'prop' | 'warn'; children: ReactNode }) {
  return <span className={`badge badge-${kind}`}>{children}</span>
}
