//! `agentic-ooxml-mcp`: stdio MCP server. Configuration comes from the environment only:
//! `AGENTIC_OOXML_ROOTS` (colon-separated allowlist, default the current directory) and
//! `AGENTIC_OOXML_ALLOW_WRITE=1` (enables the write tools). Logs go to stderr; stdout carries
//! protocol messages only.

use agentic_ooxml_mcp::{serve, Config, Server};

fn main() {
    let config = Config::from_env();
    for w in &config.warnings {
        eprintln!("agentic-ooxml-mcp: {w}");
    }
    eprintln!(
        "agentic-ooxml-mcp: {} root(s); writes {}",
        config.roots.len(),
        if config.allow_write {
            "ENABLED"
        } else {
            "disabled (set AGENTIC_OOXML_ALLOW_WRITE=1)"
        }
    );
    let server = Server::new(config);
    let stdin = std::io::stdin();
    if let Err(e) = serve(&server, stdin.lock(), std::io::stdout().lock()) {
        eprintln!("agentic-ooxml-mcp: {e}");
        std::process::exit(1);
    }
}
