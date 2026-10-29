import { useRef, type KeyboardEvent, type ReactNode } from 'react'

export interface TabDef { id: string; label: string }

/** WAI-ARIA tabs with automatic activation and arrow/Home/End keys. Renders the panel through `children`. */
export function Tabs({ tabs, active, onChange, prefix, label, children }: {
  tabs: TabDef[]; active: string; onChange: (id: string) => void; prefix: string; label: string; children: ReactNode
}) {
  const refs = useRef<Record<string, HTMLButtonElement | null>>({})
  const onKey = (e: KeyboardEvent) => {
    const i = tabs.findIndex((t) => t.id === active)
    let n = -1
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') n = (i + 1) % tabs.length
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') n = (i - 1 + tabs.length) % tabs.length
    else if (e.key === 'Home') n = 0
    else if (e.key === 'End') n = tabs.length - 1
    if (n < 0) return
    e.preventDefault()
    onChange(tabs[n].id)
    refs.current[tabs[n].id]?.focus()
  }
  return (
    <>
      <div className="tabs" role="tablist" aria-label={label} onKeyDown={onKey}>
        {tabs.map((t) => (
          <button
            key={t.id}
            ref={(el) => { refs.current[t.id] = el }}
            role="tab"
            id={`${prefix}-tab-${t.id}`}
            aria-selected={t.id === active}
            aria-controls={`${prefix}-panel`}
            tabIndex={t.id === active ? 0 : -1}
            className="tab"
            onClick={() => onChange(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div role="tabpanel" id={`${prefix}-panel`} aria-labelledby={`${prefix}-tab-${active}`} className="tabpanel">
        {children}
      </div>
    </>
  )
}
