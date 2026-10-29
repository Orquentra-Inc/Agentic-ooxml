import { spawn } from 'node:child_process'

export const BASE = process.env.SITE_BASE ?? '/Agentic-ooxml/'
export const PORT = 4173
export const PAGES = [
  { name: 'home', path: '' },
  { name: 'why', path: 'why/' },
  { name: 'contribute', path: 'contribute/' },
  { name: '404', path: '404.html' },
]

export async function startPreview() {
  const p = spawn('npx', ['vite', 'preview', '--port', String(PORT), '--strictPort'], { stdio: 'ignore' })
  for (let i = 0; i < 50; i++) {
    try { const r = await fetch(`http://localhost:${PORT}${BASE}`); if (r.ok) return p } catch { /* wait */ }
    await new Promise((r) => setTimeout(r, 200))
  }
  p.kill(); throw new Error('preview did not start')
}
export const url = (path) => `http://localhost:${PORT}${BASE}${path}`
