// Renders public/og.svg -> public/og.png (1200x630) and the apple touch icon with sharp.
import sharp from 'sharp'
import { readFileSync, writeFileSync } from 'node:fs'

const og = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#07080a"/>
  <g stroke="#20252d" stroke-width="1"><path d="M0 90H1200M0 540H1200M72 0V630M1128 0V630"/></g>
  <g fill="none" stroke="#2f5bff" stroke-width="2"><path d="M72 90h56"/></g>
  <text x="72" y="72" font-family="Menlo, Courier New, monospace" font-size="18" letter-spacing="3" fill="#98a1b0">AGENTIC OOXML</text>
  <text x="72" y="215" font-family="Georgia, Times New Roman, serif" font-size="58" fill="#e9ebef">Let an agent edit</text>
  <text x="72" y="281" font-family="Georgia, Times New Roman, serif" font-size="58" fill="#e9ebef">the board deck.</text>
  <text x="72" y="355" font-family="Georgia, Times New Roman, serif" font-style="italic" font-size="58" fill="#7f9bff">Know when a number moved.</text>
  <text x="72" y="425" font-family="Helvetica, Arial, sans-serif" font-size="24" fill="#98a1b0">Intent, sources and editing rules, stored inside the file.</text>
  <text x="72" y="500" font-family="Menlo, Courier New, monospace" font-size="20" fill="#98a1b0">Open source (MIT)   .pptx  .docx  .xlsx   MCP and CLI</text>
  <g transform="translate(860 150)">
    <rect width="268" height="150" fill="#0f1217" stroke="#343b47"/>
    <rect x="16" y="18" width="110" height="10" fill="#e9ebef" opacity=".85"/>
    <rect x="16" y="36" width="70" height="5" fill="#98a1b0"/>
    <rect x="160" y="18" width="92" height="44" fill="none" stroke="#343b47"/>
    <rect x="168" y="30" width="60" height="14" fill="#e9ebef" opacity=".85"/>
    <g fill="#2a303a"><rect x="24" y="96" width="26" height="40"/><rect x="62" y="92" width="26" height="44"/></g>
    <rect x="100" y="86" width="26" height="50" fill="#2f5bff"/>
    <rect x="138" y="70" width="26" height="66" fill="none" stroke="#7f9bff" stroke-dasharray="4 3"/>
    <rect x="156" y="14" width="100" height="52" fill="#2f5bff" fill-opacity=".16" stroke="#7f9bff" stroke-width="1.5"/>
  </g>
  <path d="M994 310v60" stroke="#2f5bff" stroke-width="2"/>
  <rect x="860" y="370" width="268" height="44" fill="none" stroke="#2f5bff" stroke-dasharray="6 4"/>
  <text x="876" y="397" font-family="Menlo, Courier New, monospace" font-size="14" fill="#7f9bff">STALE: external_claim_changed</text>
</svg>`
writeFileSync('public/og.svg', og)
await sharp(Buffer.from(og)).png().toFile('public/og.png')
await sharp(readFileSync('public/favicon.svg'), { density: 600 }).resize(180, 180).png().toFile('public/apple-touch-icon.png')
console.log('assets written')
