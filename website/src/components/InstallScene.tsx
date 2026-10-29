import { useState } from 'react'
import { SectionHead } from './Chrome'
import { Reveal } from './Reveal'
import { Tabs } from './Tabs'
import { REPO, docsUrl } from '../data'

const TABS = [
  { id: 'plugin', label: 'Claude Code plugin' },
  { id: 'skill', label: 'Agent Skill' },
  { id: 'mcp', label: 'MCP server' },
  { id: 'cli', label: 'CLI' },
]

const BODY: Record<string, { what: string; cmd: string[]; note: string }> = {
  plugin: { what: 'Bundles the skills and the MCP server entry into one install for Claude Code. It does not bundle the binary: install the MCP server (below) first.', cmd: ['claude plugin marketplace add Orquentra-Inc/Agentic-ooxml', 'claude plugin install agentic-ooxml@agentic-ooxml', '# inside a session: /plugin marketplace add Orquentra-Inc/Agentic-ooxml'], note: 'The manifests pass claude plugin validate. Running the skills in a live session has not been verified end to end.' },
  skill: { what: 'Skills for inspecting, editing, validating and tracking provenance, usable without the plugin.', cmd: ['git clone https://github.com/Orquentra-Inc/Agentic-ooxml', 'cp -r Agentic-ooxml/integrations/skills/ooxml-* <your agent skills directory>'], note: 'Skills ship only for behaviour the code backs. Where your agent looks for skills depends on the agent.' },
  mcp: { what: 'A stdio MCP server over the Rust library. Read tools by default; write tools need explicit opt-in.', cmd: ['cargo install --git https://github.com/Orquentra-Inc/Agentic-ooxml agentic-ooxml-mcp', 'AGENTIC_OOXML_ROOTS=/path/to/docs   # allowed directories', 'AGENTIC_OOXML_ALLOW_WRITE=1       # opt in to ao_write, ao_reconcile'], note: 'Tested over stdio by integrations/verify.sh; not yet verified with every agent client.' },
  cli: { what: 'The agentic-ooxml binary. Every result is JSON on stdout.', cmd: ['cargo install --git https://github.com/Orquentra-Inc/Agentic-ooxml agentic-ooxml-cli', 'agentic-ooxml list --file deck.pptx', 'agentic-ooxml sources --file deck.pptx', 'agentic-ooxml reconcile --file deck.pptx --dry-run'], note: 'Needs a Rust toolchain. Prebuilt binaries are below.' },
}

export function InstallScene() {
  const [id, setId] = useState('plugin')
  const b = BODY[id]
  return (
    <section id="install" className="scene" aria-labelledby="h-install">
      <div className="wrap">
        <SectionHead n="05" total="05" eyebrow="Install" id="h-install" lede="Early release, version 0.1. Not on crates.io or npm: install from source, from GitHub Releases, or through the Claude Code marketplace.">
          Install paths.
        </SectionHead>
        <Reveal>
          <Tabs tabs={TABS} active={id} onChange={setId} prefix="install" label="Install method">
            <div className="panel install-panel">
              <p>{b.what}</p>
              <pre className="code" tabIndex={0} aria-label={`Install commands for ${id}`}><code>{b.cmd.join('\n')}</code></pre>
              <p className="small muted">{b.note}</p>
            </div>
          </Tabs>
          <div className="panel from-source">
            <h3>From a checkout</h3>
            <p className="small"><span className="mono">cargo install</span> takes one <span className="mono">--path</span> per command, so installing both binaries from a checkout is two commands.</p>
            <pre className="code" tabIndex={0} aria-label="Build from source"><code>{`git clone https://github.com/Orquentra-Inc/Agentic-ooxml\ncd Agentic-ooxml\ncargo install --path crates/agentic-ooxml-cli\ncargo install --path crates/agentic-ooxml-mcp\nagentic-ooxml --version`}</code></pre>
          </div>
          <div className="panel from-source">
            <h3>Prebuilt binaries</h3>
            <p className="small"><a href={`${REPO}/releases`}>GitHub Releases</a> (v0.1.0) carries Linux x86_64, macOS arm64 and x86_64, and Windows x86_64 archives with a <span className="mono">SHA256SUMS</span> file. They are not signed. On Linux and macOS, <a href={docsUrl('scripts/install.sh')}>scripts/install.sh</a> verifies the checksum and installs into <span className="mono">~/.local/bin</span>.</p>
            <pre className="code" tabIndex={0} aria-label="Install script"><code>{`curl -fsSL https://raw.githubusercontent.com/Orquentra-Inc/Agentic-ooxml/main/scripts/install.sh | sh`}</code></pre>
            <p className="small muted">Windows x86_64 is a .zip you unpack by hand. On any other platform, build from source.</p>
          </div>
        </Reveal>
      </div>
    </section>
  )
}
