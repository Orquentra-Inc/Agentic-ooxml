# Security policy

## Reporting a vulnerability

**Contact: [agenticoffcie70@gmail.com](mailto:agenticoffcie70@gmail.com).** Please report vulnerabilities privately by e-mail, or use
GitHub's private vulnerability reporting on this repository (Security tab, "Report a vulnerability").
Do not open public issues for vulnerabilities. We aim to acknowledge a report within a few days;
this is a small project and that is a goal, not a guarantee.

Please include the affected version or commit, a minimal package (`.docx`, `.pptx` or `.xlsx`)
that reproduces the problem if one is needed, and the command you ran.

## Supported versions

The project is an early release (`0.1.x`). Only the latest release and the latest commit on the
default branch are supported.

## What is in scope

Agentic OOXML reads and rewrites ZIP packages that come from untrusted sources, so these are in
scope:

- decompression bombs, path traversal (`..`, absolute names) and entry-count or size cap bypasses
- XML parser abuse (DOCTYPE and entity handling, nesting, node counts)
- a write that alters native (non-Agentic-OOXML) package entries, or that leaves a corrupt package
- embedded text that is returned without the `"trust": "document"` marker
- any path by which embedded document content can change the tool's permissions or behaviour
- escaping `AGENTIC_OOXML_ROOTS`, or writing without
  `AGENTIC_OOXML_ALLOW_WRITE=1`

Out of scope: the claim that a hash stored inside an editable file is not proof of authenticity.
That is a documented limitation, see [docs/security.md](docs/security.md).

For the threat model and the defences that exist today, read [docs/security.md](docs/security.md).
