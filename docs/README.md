# Documentation

| page | read it when |
|---|---|
| [Getting started](getting-started.md) | you want to run something in ten minutes |
| [Concepts](concepts.md) | you want the ideas: OPC, identity, provenance, staleness, intent versus specification |
| [Why Agentic OOXML?](why.md) | you want the problem, the prior work, and an honest novelty assessment |
| [Architecture](architecture.md) | you want the design choices and the evidence behind them |
| [Schema reference](schema.md) | you need the fields of a part, and which parts are implemented or only proposed |
| [Guides](guides/README.md) | you have a task: carry intent, track sources, read v0.2, use from an agent |
| [Security](security.md) | you are putting this in front of untrusted files or agents |
| [Compatibility](compatibility.md) | you want to know what has and has not been measured |
| [Troubleshooting](troubleshooting.md) | something failed |
| [Contributing](contributing.md) | you want to help: round-trip reports, examples, schema proposals, integrations |
| [Round-trip evidence](evidence/round-trip-2026-10-19/README.md) | you want the raw numbers behind the compatibility page |
| [Architecture paper](paper/architecture.md) | you want the full technical write-up and the research questions |

## Maintaining the docs

Every command in these pages was run in this repository. When you change a command's output, rerun it and update the page.
Check links with:

```bash
python3 docs/check-links.py
```

The script checks relative links and heading anchors in every Markdown file outside `docs/internal`. It does
not fetch external URLs. The sample-file helpers used by the guides are
[make-minimal-deck.py](guides/make-minimal-deck.py) and [edit-forecast.py](guides/edit-forecast.py).
