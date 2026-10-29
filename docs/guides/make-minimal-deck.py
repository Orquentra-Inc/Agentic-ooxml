#!/usr/bin/env python3
"""Write a minimal one-slide .pptx with synthetic figures, using only the standard library.

Usage: make-minimal-deck.py OUT.pptx [FORECAST]    (FORECAST defaults to 21.3)

The slide has two shapes: shape id 2 (a title) and shape id 3 (the Q4 forecast, EUR millions).
The file is a valid OPC package for the purposes of the Agentic OOXML tools. It is not a
polished deck, and it is not meant to be opened in PowerPoint.
"""
import sys
import zipfile

CT = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
      '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
      '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
      '<Default Extension="xml" ContentType="application/xml"/>'
      '<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>'
      '</Types>')
REL_NS = "http://schemas.openxmlformats.org/package/2006/relationships"
OD = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
ROOT_RELS = (f'<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="{REL_NS}">'
             f'<Relationship Id="rId1" Type="{OD}/officeDocument" Target="ppt/presentation.xml"/></Relationships>')
PRES_RELS = (f'<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="{REL_NS}">'
             f'<Relationship Id="rId1" Type="{OD}/slide" Target="slides/slide1.xml"/></Relationships>')
PRES = '<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>'


def shape(i, name, text):
    return (f'<p:sp><p:nvSpPr><p:cNvPr id="{i}" name="{name}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>'
            f'<p:txBody><a:bodyPr/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp>')


def main():
    out = sys.argv[1]
    forecast = sys.argv[2] if len(sys.argv) > 2 else "21.3"
    slide = ('<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" '
             'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree>'
             + shape(2, "Title", "Q4 outlook")
             + shape(3, "Forecast", f"Q4 forecast EUR {forecast}M")
             + '</p:spTree></p:cSld></p:sld>')
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("[Content_Types].xml", CT)
        z.writestr("_rels/.rels", ROOT_RELS)
        z.writestr("ppt/presentation.xml", PRES)
        z.writestr("ppt/_rels/presentation.xml.rels", PRES_RELS)
        z.writestr("ppt/slides/slide1.xml", slide)


if __name__ == "__main__":
    main()
