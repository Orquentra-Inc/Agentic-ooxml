// axe-core on every page (default and reduced motion), after exercising the interactive states, plus behaviour checks.
import { chromium } from 'playwright'
import AxeBuilder from '@axe-core/playwright'
import { PAGES, startPreview, url } from './lib.mjs'

const server = await startPreview()
const browser = await chromium.launch()
let fail = 0
const settle = (page) => page.waitForTimeout(1000) // let scroll-reveal fades finish before contrast checks
const check = (ok, msg) => { console.log(`${ok ? 'PASS' : 'FAIL'}  ${msg}`); if (!ok) fail++ }

async function axe(page, label) {
  // Scan only once the page has stopped changing: scroll reveals animate opacity from JavaScript, and an element caught
  // mid-fade reads as low contrast. Sample every opacity and the web animations until two samples in a row agree.
  const snap = () => page.evaluate(() => [...document.querySelectorAll('body *')].map((el) => getComputedStyle(el).opacity).join(',') + '|' + document.getAnimations().filter((an) => an.effect?.getComputedTiming().iterations !== Infinity && an.playState === 'running').length)
  let prev = await snap()
  for (let i = 0; i < 40; i++) {
    await page.waitForTimeout(150)
    const cur = await snap()
    if (cur === prev) { await page.waitForTimeout(100); if ((await snap()) === cur) break }
    prev = cur
  }
  const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa', 'best-practice']).analyze()
  check(r.violations.length === 0, `axe ${label}: ${r.violations.length} violations (${r.passes.length} rules passed)`)
  for (const v of r.violations) console.log('   ', v.id, v.impact, v.help, v.nodes.slice(0, 3).map((n) => n.target.join(' ')).join(' | '))
}

for (const [width, reduced] of [[1440, 'no-preference'], [375, 'reduce']]) {
  const ctx = await browser.newContext({ viewport: { width, height: 900 }, reducedMotion: reduced })
  for (const pg of PAGES) {
    const page = await ctx.newPage()
    await page.goto(url(pg.path), { waitUntil: 'networkidle' })
    if (pg.name === 'home' || pg.name === 'how') {
      // reveal everything (Reveal animates on scroll), then wait for it to settle
      await page.evaluate(async () => { for (let y = 0; y < document.body.scrollHeight; y += 500) { window.scrollTo(0, y); await new Promise((r) => setTimeout(r, 60)) } window.scrollTo(0, 0) })
      await page.waitForTimeout(1200)
    }
    await axe(page, `${pg.name} @${width} motion=${reduced}`)
    if (pg.name === 'home' && width === 1440) {
      // proof loop: edit the workbook, see the deck go stale, then update, reconcile, re-verify
      await page.getByLabel('Q3 revenue in the workbook, EUR million').fill('19.1'); await page.waitForTimeout(100)
      let panel = await page.locator('#stale-panel').innerText()
      check(panel.includes('stale (external_claim_changed)'), 'proof: editing the workbook cell makes the deck source stale (external_claim_changed)')
      await axe(page, 'home proof stale state')
      await page.getByRole('button', { name: '2. Run reconcile' }).click()
      check((await page.locator('#stale-panel').innerText()).includes('Reconcile wrote status="stale"'), 'proof: reconcile persists the stale status')
      await page.getByRole('button', { name: '1. Agent updates the slide' }).click()
      await page.getByRole('button', { name: '3. Re-verify' }).click(); await page.waitForTimeout(100)
      panel = await page.locator('#stale-panel').innerText()
      check(panel.includes('effective status') && panel.includes('verified') && !panel.includes('external_claim_changed)'), 'proof: re-verify takes a new baseline and the source is verified again')
      await page.getByRole('button', { name: 'Reset' }).first().click()
      await page.click('#stale-tab-slide'); await settle(page); await axe(page, 'home proof slide tab')
      const input = page.getByLabel('KPI text on the slide (shape id 12)')
      await input.fill('EUR 19.4M'); await page.waitForTimeout(100)
      check((await page.locator('#stale-panel').innerText()).includes('stale (claim_changed)'), 'proof: editing the slide claim makes effective status stale (claim_changed)')
      await input.fill('EUR 18.6M')
      check(!(await page.locator('#stale-panel').innerText()).includes('claim_changed'), 'proof: restoring the claim restores verified (hash equal)')
      // run stepper
      for (let k = 0; k < 5; k++) { await page.click(`.run-steps li >> nth=${k} >> button`); await page.waitForTimeout(80) }
      check((await page.locator('#run-detail').innerText()).includes('"warnings": []'), 'run: last step shows the clean validate')
      await settle(page); await axe(page, 'home run step 5')
      // copy button gives feedback
      await page.context().grantPermissions(['clipboard-read', 'clipboard-write']).catch(() => {})
      await page.locator('.hero .cmd-copy').click(); await page.waitForTimeout(150)
      check((await page.locator('.hero .cmd-copy').innerText()) === 'Copied', 'hero: copy button confirms')
      // FAQ accordion: keyboard toggles
      await page.locator('.faq summary >> nth=0').focus(); await page.keyboard.press('Enter')
      check(await page.locator('.faq details >> nth=0').getAttribute('open') !== null, 'faq: Enter opens the first question')
      await settle(page); await axe(page, 'home faq open')
    }
    if (pg.name === 'how' && width === 1440) {
      for (const id of ['agents', 'design', 'sources', 'spec']) {
        await page.click(`#layers-tab-${id}`); await page.waitForTimeout(350)
        await axe(page, `how layers tab ${id}`)
      }
      await page.focus('#layers-tab-spec'); await page.keyboard.press('ArrowRight')
      check(await page.getAttribute('#layers-tab-agents', 'aria-selected') === 'true', 'layers tabs: ArrowRight wraps to first tab and selects it')
      await page.keyboard.press('End')
      check(await page.getAttribute('#layers-tab-spec', 'aria-selected') === 'true', 'layers tabs: End selects last tab')
      for (const id of ['skill', 'mcp', 'cli']) { await page.click(`#install-tab-${id}`); }
      await settle(page); await axe(page, 'how install cli tab')
      await page.click('.step >> nth=5'); await settle(page); await axe(page, 'how workflow step 6')
      await page.click('.part >> nth=9'); await settle(page); await axe(page, 'how package part selected')
    }
    await page.close()
  }
  await ctx.close()
}
// reduced motion: no running animations on hero
const ctx = await browser.newContext({ reducedMotion: 'reduce', viewport: { width: 1440, height: 900 } })
const page = await ctx.newPage(); await page.goto(url(''), { waitUntil: 'networkidle' })
check(await page.locator('.scan').count() === 0, 'reduced motion: hero scan line not rendered')
check((await page.evaluate(() => document.getAnimations().length)) === 0, 'reduced motion: zero running animations/transitions at load')
await browser.close(); server.kill()
process.exit(fail ? 1 : 0)
