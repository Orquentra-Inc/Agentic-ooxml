import { useEffect, useRef, useState } from 'react'

/** A command block with a copy button. The whole text is copied, exactly as shown. */
export function CopyCommand({ text, label, wrap = false }: { text: string; label: string; wrap?: boolean }) {
  const [done, setDone] = useState(false)
  const timer = useRef<number | undefined>(undefined)
  useEffect(() => () => window.clearTimeout(timer.current), [])
  const copy = async () => {
    let ok = false
    try { await navigator.clipboard.writeText(text); ok = true } catch { /* fall through to the legacy path */ }
    if (!ok) {
      const ta = document.createElement('textarea')
      ta.value = text; ta.setAttribute('readonly', ''); ta.style.position = 'fixed'; ta.style.opacity = '0'
      document.body.appendChild(ta); ta.select()
      try { ok = document.execCommand('copy') } catch { ok = false }
      document.body.removeChild(ta)
    }
    if (!ok) return
    setDone(true)
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => setDone(false), 2000)
  }
  return (
    <div className={`cmd${wrap ? ' cmd-wrap' : ''}`}>
      <pre className="cmd-text mono" tabIndex={0} aria-label={label}><code>{text}</code></pre>
      <button type="button" className="btn btn-quiet cmd-copy" onClick={copy} aria-label={done ? 'Copied' : `Copy ${label}`}>{done ? 'Copied' : 'Copy'}</button>
      <span className="sr-only" role="status">{done ? 'Copied to clipboard' : ''}</span>
    </div>
  )
}
