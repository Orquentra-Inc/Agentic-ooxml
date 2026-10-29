#!/usr/bin/env python3
"""Simulate another tool editing the forecast on the sample slide, leaving every other entry intact.

Usage: edit-forecast.py FILE.pptx OLD NEW     (for example: 21.3 23.0)

This stands in for "someone changed the number in PowerPoint": it rewrites only
ppt/slides/slide1.xml and copies every other ZIP entry unchanged, including the Agentic OOXML parts.
"""
import os
import sys
import zipfile

path, old, new = sys.argv[1:4]
tmp = path + ".tmp"
with zipfile.ZipFile(path) as src, zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as out:
    for info in src.infolist():
        data = src.read(info.filename)
        if info.filename == "ppt/slides/slide1.xml":
            if old.encode() not in data:
                sys.exit(f"{old!r} not found in the slide")
            data = data.replace(old.encode(), new.encode())
        out.writestr(info, data)
os.replace(tmp, path)
