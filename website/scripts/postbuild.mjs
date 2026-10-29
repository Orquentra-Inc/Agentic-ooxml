// After vite build: sitemap.xml, robots.txt, .nojekyll. SITE_URL must match the deploy URL.
import { writeFileSync, existsSync } from 'node:fs'

const site = (process.env.SITE_URL ?? 'https://orquentra-inc.github.io/Agentic-ooxml').replace(/\/$/, '')
const pages = ['/', '/why/', '/contribute/']
const today = new Date().toISOString().slice(0, 10)
const xml = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${pages
  .map((p) => `  <url><loc>${site}${p}</loc><lastmod>${today}</lastmod></url>`)
  .join('\n')}\n</urlset>\n`
writeFileSync('dist/sitemap.xml', xml)
writeFileSync('dist/robots.txt', `User-agent: *\nAllow: /\nSitemap: ${site}/sitemap.xml\n`)
writeFileSync('dist/.nojekyll', '')
if (!existsSync('dist/404.html')) throw new Error('404.html missing from dist')
console.log('postbuild: sitemap, robots, .nojekyll written for', site)
