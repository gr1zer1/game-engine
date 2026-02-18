#!/usr/bin/env python3
# AI
from __future__ import annotations

import re
import sys
from pathlib import Path
from xml.sax.saxutils import escape

from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import mm
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import Paragraph, Preformatted, SimpleDocTemplate, Spacer

FONT_SANS = "LiberationSans"
FONT_SANS_BOLD = "LiberationSans-Bold"
FONT_MONO = "LiberationMono"


def register_fonts() -> None:
    pdfmetrics.registerFont(
        TTFont(FONT_SANS, "/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
    )
    pdfmetrics.registerFont(
        TTFont(FONT_SANS_BOLD, "/usr/share/fonts/liberation/LiberationSans-Bold.ttf")
    )
    pdfmetrics.registerFont(
        TTFont(FONT_MONO, "/usr/share/fonts/liberation/LiberationMono-Regular.ttf")
    )


def parse_markdown(md_text: str):
    lines = md_text.splitlines()
    blocks = []
    i = 0
    in_code = False
    code_buf = []

    while i < len(lines):
        line = lines[i]

        if line.strip().startswith("```"):
            if in_code:
                blocks.append(("code", "\n".join(code_buf).rstrip()))
                code_buf = []
                in_code = False
            else:
                in_code = True
            i += 1
            continue

        if in_code:
            code_buf.append(line)
            i += 1
            continue

        if not line.strip():
            i += 1
            continue

        if re.match(r"^---+$", line.strip()):
            blocks.append(("separator", ""))
            i += 1
            continue

        heading = re.match(r"^(#{1,6})\s+(.*)$", line)
        if heading:
            level = len(heading.group(1))
            text = heading.group(2).strip()
            blocks.append(("heading", level, text))
            i += 1
            continue

        if re.match(r"^\s*(?:[-*]|\d+\.)\s+", line):
            items = []
            while i < len(lines) and re.match(r"^\s*(?:[-*]|\d+\.)\s+", lines[i]):
                item = re.sub(r"^\s*(?:[-*]|\d+\.)\s+", "", lines[i]).rstrip()
                items.append(item)
                i += 1
            blocks.append(("list", items))
            continue

        para_lines = [line.strip()]
        i += 1
        while i < len(lines):
            next_line = lines[i]
            if not next_line.strip():
                break
            if next_line.strip().startswith("```"):
                break
            if re.match(r"^(#{1,6})\s+", next_line):
                break
            if re.match(r"^\s*(?:[-*]|\d+\.)\s+", next_line):
                break
            if re.match(r"^---+$", next_line.strip()):
                break
            para_lines.append(next_line.strip())
            i += 1
        blocks.append(("para", " ".join(para_lines).strip()))

    return blocks


def page_number(canvas, doc):
    canvas.saveState()
    canvas.setFont(FONT_SANS, 9)
    canvas.drawRightString(200 * mm, 10 * mm, f"Страница {doc.page}")
    canvas.restoreState()


def build_pdf(input_path: Path, output_path: Path) -> None:
    register_fonts()

    styles = getSampleStyleSheet()
    style_body = ParagraphStyle(
        "BodyRU",
        parent=styles["Normal"],
        fontName=FONT_SANS,
        fontSize=11,
        leading=15,
        spaceAfter=6,
    )
    style_h1 = ParagraphStyle(
        "H1RU",
        parent=styles["Heading1"],
        fontName=FONT_SANS_BOLD,
        fontSize=20,
        leading=24,
        spaceBefore=10,
        spaceAfter=10,
    )
    style_h2 = ParagraphStyle(
        "H2RU",
        parent=styles["Heading2"],
        fontName=FONT_SANS_BOLD,
        fontSize=16,
        leading=20,
        spaceBefore=10,
        spaceAfter=8,
    )
    style_h3 = ParagraphStyle(
        "H3RU",
        parent=styles["Heading3"],
        fontName=FONT_SANS_BOLD,
        fontSize=13,
        leading=17,
        spaceBefore=8,
        spaceAfter=6,
    )
    style_code = ParagraphStyle(
        "CodeRU",
        fontName=FONT_MONO,
        fontSize=9,
        leading=12,
        leftIndent=8,
        rightIndent=8,
        spaceBefore=4,
        spaceAfter=8,
    )

    md_text = input_path.read_text(encoding="utf-8")
    blocks = parse_markdown(md_text)

    story = []
    for block in blocks:
        kind = block[0]

        if kind == "heading":
            _, level, text = block
            pstyle = style_h1 if level == 1 else style_h2 if level == 2 else style_h3
            story.append(Paragraph(escape(text), pstyle))
            continue

        if kind == "para":
            _, text = block
            txt = escape(text).replace("`", "")
            story.append(Paragraph(txt, style_body))
            continue

        if kind == "list":
            _, items = block
            for item in items:
                txt = f"• {escape(item).replace('`', '')}"
                story.append(Paragraph(txt, style_body))
            story.append(Spacer(1, 2))
            continue

        if kind == "code":
            _, code = block
            if code:
                story.append(Preformatted(code, style_code))
            continue

        if kind == "separator":
            story.append(Spacer(1, 8))
            continue

    doc = SimpleDocTemplate(
        str(output_path),
        pagesize=A4,
        leftMargin=18 * mm,
        rightMargin=18 * mm,
        topMargin=16 * mm,
        bottomMargin=16 * mm,
        title="Game Engine Documentation",
        author="Codex",
    )
    doc.build(story, onFirstPage=page_number, onLaterPages=page_number)


def main() -> int:
    if len(sys.argv) != 3:
        print("Usage: generate_pdf.py <input.md> <output.pdf>")
        return 2

    input_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2])

    if not input_path.exists():
        print(f"Input not found: {input_path}")
        return 1

    output_path.parent.mkdir(parents=True, exist_ok=True)
    build_pdf(input_path, output_path)
    print(f"Generated: {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
