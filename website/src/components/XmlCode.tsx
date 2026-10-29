import { Fragment } from 'react'

const TOKEN = /(<!\[CDATA\[[\s\S]*?\]\]>|<!--[\s\S]*?-->|<\/?[\w:.-]+|\/?>|[\w:.-]+(?==")|"[^"]*"|\s+|[^<>"\s]+|.)/g

function highlightLine(line: string, key: number) {
  const out: JSX.Element[] = []
  let inTag = false
  let i = 0
  for (const m of line.matchAll(TOKEN)) {
    const t = m[0]
    let cls = ''
    if (t.startsWith('<![CDATA[')) cls = 'tk-cdata'
    else if (t.startsWith('<!--')) cls = 'tk-comment'
    else if (/^<\/?[\w:.-]+/.test(t)) { cls = 'tk-tag'; inTag = true }
    else if (t === '>' || t === '/>') { cls = 'tk-tag'; inTag = false }
    else if (inTag && /^[\w:.-]+$/.test(t)) cls = 'tk-attr'
    else if (inTag && t.startsWith('"')) cls = 'tk-val'
    out.push(cls ? <span key={i++} className={cls}>{t}</span> : <Fragment key={i++}>{t}</Fragment>)
  }
  return <Fragment key={key}>{out}{'\n'}</Fragment>
}

/** A small dependency-free XML highlighter. Never executes or parses the text; it only colours it. */
export function XmlCode({ code, label, mark }: { code: string; label: string; mark?: string }) {
  const lines = code.split('\n')
  return (
    <pre className="code" tabIndex={0} aria-label={label}>
      <code>
        {lines.map((l, i) =>
          mark && l.includes(mark) ? (
            <mark key={i} className="code-mark">{highlightLine(l, i)}</mark>
          ) : (
            highlightLine(l, i)
          ),
        )}
      </code>
    </pre>
  )
}
