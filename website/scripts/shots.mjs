// Full-page screenshots at 1440, 768 and 375 for every page, plus overflow and console-error checks.
import { chromium } from 'playwright'
import { mkdirSync } from 'node:fs'
import { PAGES, startPreview, url } from './lib.mjs'

const out = process.env.SHOTS_DIR ?? 'shots'
mkdirSync(out, { recursive: true })
const server = await startPreview()
const browser = await chromium.launch()
let bad = 0
for (const width of [1440, 768, 375]) {
  const ctx = await browser.newContext({ viewport: { width, height: 900 }, reducedMotion: 'reduce', deviceScaleFactor: 1 })
  for (const pg of PAGES) {
    const page = await ctx.newPage()
    const errs = []
    page.on('console', (m) => m.type() === 'error' && errs.push(m.text()))
    page.on('pageerror', (e) => errs.push(String(e)))
    await page.goto(url(pg.path), { waitUntil: 'networkidle' })
    await page.evaluate(() => document.fonts.ready)
    const m = await page.evaluate(() => ({ sw: document.documentElement.scrollWidth, iw: window.innerWidth, h: document.documentElement.scrollHeight }))
    const wide = await page.evaluate(() => [...document.querySelectorAll('body *')].filter((e) => e.getBoundingClientRect().right > window.innerWidth + 1 && !e.closest('.code, .site-header')).slice(0, 5).map((e) => e.tagName + '.' + e.className))
    await page.screenshot({ path: `${out}/${pg.name}-${width}.png`, fullPage: true })
    const flag = m.sw > m.iw ? 'OVERFLOW' : 'ok'
    if (m.sw > m.iw || errs.length) bad++
    console.log(`${pg.name} @${width}: scrollWidth ${m.sw}/${m.iw} height ${m.h} ${flag}${wide.length ? ' wide:' + wide.join(',') : ''}${errs.length ? ' ERR ' + errs.join(' | ') : ''}`)
    await page.close()
  }
  await ctx.close()
}
await browser.close(); server.kill()
process.exit(bad ? 1 : 0)
