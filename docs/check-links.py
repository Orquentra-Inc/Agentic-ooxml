#!/usr/bin/env python3
"""Check that every relative link and heading anchor in the repository's Markdown resolves.

Usage: python3 docs/check-links.py [ROOT]      (ROOT defaults to the repository root)

Checks, for each *.md file outside docs/internal, node_modules and target:
  - [text](path) and ![alt](path): the file or directory exists (relative to the file);
  - [text](path#anchor) and [text](#anchor): the anchor matches a heading in the target file,
    using GitHub's slug rules;
  - links into docs/internal are reported, because public pages should not depend on them.
External links (http, https, mailto) are not fetched. Exit code 1 if anything fails.
"""
import re
import sys
from pathlib import Path

SKIP_DIRS = {"node_modules", "target", ".git", "internal", "dist"}
LINK = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
FENCE = re.compile(r"^\s*(```|~~~)")


def slug(heading: str) -> str:
    s = re.sub(r"`", "", heading.strip().lower())
    s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", s)
    s = re.sub(r"[^\w\- ]", "", s, flags=re.UNICODE)
    return s.replace(" ", "-")


def headings(path: Path) -> set:
    out, seen, fenced = set(), {}, False
    for line in path.read_text(encoding="utf-8").splitlines():
        if FENCE.match(line):
            fenced = not fenced
            continue
        m = None if fenced else re.match(r"^#{1,6}\s+(.*?)\s*#*$", line)
        if m:
            s = slug(m.group(1))
            n = seen.get(s, 0)
            seen[s] = n + 1
            out.add(s if n == 0 else f"{s}-{n}")
    return out


def md_files(root: Path):
    for p in sorted(root.rglob("*.md")):
        if not (set(p.relative_to(root).parts) & SKIP_DIRS):
            yield p


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent).resolve()
    errors, checked, cache = [], 0, {}
    for md in md_files(root):
        fenced = False
        for n, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
            if FENCE.match(line):
                fenced = not fenced
                continue
            if fenced:
                continue
            for target in LINK.findall(re.sub(r"`[^`]*`", "", line)):
                if re.match(r"^[a-z][a-z0-9+.-]*:", target, re.I):
                    continue
                checked += 1
                path_part, _, anchor = target.partition("#")
                dest = md if not path_part else (md.parent / path_part).resolve()
                where = f"{md.relative_to(root)}:{n}"
                if not dest.exists():
                    errors.append(f"{where}: missing target {target}")
                    continue
                if "internal" in dest.relative_to(root).parts[:2] and dest != md:
                    errors.append(f"{where}: public page links into docs/internal: {target}")
                if anchor and dest.suffix == ".md":
                    if dest not in cache:
                        cache[dest] = headings(dest)
                    if anchor.lower() not in cache[dest]:
                        errors.append(f"{where}: no heading '#{anchor}' in {dest.relative_to(root)}")
    for e in errors:
        print(e)
    print(f"checked {checked} relative links in {len(list(md_files(root)))} files: {len(errors)} problem(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
