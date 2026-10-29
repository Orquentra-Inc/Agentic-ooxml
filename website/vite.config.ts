import { defineConfig, type Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import { resolve } from 'node:path'

// Deploy base path. GitHub Pages project sites live under /<repo>/. Override with SITE_BASE.
const base = normaliseBase(process.env.SITE_BASE ?? '/Agentic-ooxml/')
const siteUrl = (process.env.SITE_URL ?? 'https://orquentra-inc.github.io/Agentic-ooxml').replace(/\/$/, '')
const repoUrl = (process.env.REPO_URL ?? 'https://github.com/Orquentra-Inc/Agentic-ooxml').replace(/\/$/, '')

function normaliseBase(b: string) {
  if (b === '/' || b === '') return '/'
  return '/' + b.replace(/^\/+|\/+$/g, '') + '/'
}

// Replaces %SITE_URL% in the HTML entry points so canonical and OG tags are absolute.
function siteTokens(): Plugin {
  return {
    name: 'site-tokens',
    transformIndexHtml: (html) => html.replaceAll('%SITE_URL%', siteUrl).replaceAll('%REPO_URL%', repoUrl),
  }
}

export default defineConfig({
  base,
  plugins: [react(), siteTokens()],
  define: {
    __REPO_URL__: JSON.stringify(repoUrl),
    __SITE_URL__: JSON.stringify(siteUrl),
  },
  build: {
    target: 'es2022',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        why: resolve(__dirname, 'why/index.html'),
        contribute: resolve(__dirname, 'contribute/index.html'),
        notfound: resolve(__dirname, '404.html'),
      },
    },
  },
})
