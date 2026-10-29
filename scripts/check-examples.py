#!/usr/bin/env python3
"""Independent package check for the example files, using only the Python standard library.

For each .pptx/.xlsx/.docx given (default: examples/*): zip CRC test, every .xml/.rels part parses
as XML, every part has a content type, every internal relationship target exists, and the AO
parts (customXml parts in the AO namespaces) parse. This is a structural check written separately
from the Rust validator; it does NOT show that Microsoft Office or LibreOffice opens the files.
"""
import posixpath
import sys
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

CT = "{http://schemas.openxmlformats.org/package/2006/content-types}"
RL = "{http://schemas.openxmlformats.org/package/2006/relationships}"
AO_NS = ("urn:agentic-office:ooxml:v0.1", "urn:agentic-office:context:v0.2")


def check(path: Path) -> list[str]:
    problems: list[str] = []
    with zipfile.ZipFile(path) as z:
        bad = z.testzip()
        if bad:
            problems.append(f"CRC failure in {bad}")
        names = [n for n in z.namelist() if not n.endswith("/")]
        parsed = {}
        for n in names:
            if n.endswith((".xml", ".rels")):
                try:
                    parsed[n] = ET.fromstring(z.read(n))
                except ET.ParseError as e:
                    problems.append(f"{n}: not well-formed: {e}")
        ct = parsed.get("[Content_Types].xml")
        if ct is None:
            problems.append("no [Content_Types].xml")
        else:
            defaults = {d.get("Extension").lower() for d in ct.findall(CT + "Default")}
            overrides = {o.get("PartName").lstrip("/").lower() for o in ct.findall(CT + "Override")}
            for n in names:
                if n != "[Content_Types].xml" and n.lower() not in overrides and n.rsplit(".", 1)[-1].lower() not in defaults:
                    problems.append(f"{n}: no content type")
            for o in overrides:
                if o not in {n.lower() for n in names}:
                    problems.append(f"override for missing part {o}")
        for n, root in parsed.items():
            if not n.endswith(".rels"):
                continue
            src_dir = posixpath.dirname(posixpath.dirname(n))  # strip "_rels"
            ids = set()
            for r in root.findall(RL + "Relationship"):
                if r.get("Id") in ids:
                    problems.append(f"{n}: duplicate relationship id {r.get('Id')}")
                ids.add(r.get("Id"))
                if r.get("TargetMode") == "External":
                    continue
                t = r.get("Target")
                target = t.lstrip("/") if t.startswith("/") else posixpath.normpath(posixpath.join(src_dir, t))
                if target not in names:
                    problems.append(f"{n}: relationship {r.get('Id')} -> {target} does not exist")
        ao = 0
        for n, root in parsed.items():
            if n.startswith("customXml/") and root.tag.startswith("{") and root.tag[1:].split("}")[0] in AO_NS:
                ao += 1
        if ao < 4:
            problems.append(f"expected 4 AO parts (agents, design, intent, sources), found {ao}")
    return problems


def main() -> int:
    files = [Path(a) for a in sys.argv[1:]] or sorted(p for p in Path("examples").iterdir() if p.suffix in (".pptx", ".xlsx", ".docx"))
    failed = False
    for f in files:
        probs = check(f)
        print(f"{f}: {'OK' if not probs else 'FAIL'}")
        for p in probs:
            print(f"  - {p}")
        failed |= bool(probs)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
