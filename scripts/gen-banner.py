#!/usr/bin/env python3
"""Generates fralsare-daily promo banners using Pillow + DejaVu fonts.

Outputs:
  docs/banner.png  1600x500  wide web banner (dark)
  docs/og.png      1200x630  social / Open Graph card (light)

Run:  python3 scripts/gen-banner.py
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT_DIR = Path("/usr/share/fonts/truetype/dejavu")
LOGO = ROOT / "logo.png"

BOLD = FONT_DIR / "DejaVuSans-Bold.ttf"
REG = FONT_DIR / "DejaVuSans.ttf"

BLUE = (25, 99, 235)
BLUE_DARK = (30, 64, 175)


def font(path, size):
    return ImageFont.truetype(str(path), size)


def vgrad(w, h, top, bottom):
    img = Image.new("RGB", (w, h))
    draw = ImageDraw.Draw(img)
    for y in range(h):
        g = y / h
        c = tuple(round(top[i] + (bottom[i] - top[i]) * g) for i in range(3))
        draw.line([(0, y), (w, y)], fill=c)
    return img


def strip(im, y, h, color):
    d = ImageDraw.Draw(im)
    d.rectangle([0, y, im.width, y + h], fill=color)


def ghost_page(im, box, radius, color):
    """Translucent rounded rect (the 'stacked page' motif)."""
    overlay = Image.new("RGBA", im.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    d.rounded_rectangle(box, radius=radius, fill=color)
    im.alpha_composite(overlay)


def paste_logo(im, logo_path, x, y, size):
    logo = Image.open(logo_path).convert("RGBA").resize((size, size), Image.Resampling.LANCZOS)
    im.alpha_composite(logo, (x, y))


def text_size(d, text, fnt):
    left, top, right, bottom = d.textbbox((0, 0), text, font=fnt)
    return right - left, bottom - top


# ---------------------------------------------------------------- banner A
def gen_banner():
    W, H = 1600, 500
    im = vgrad(W, H, (15, 22, 34), (9, 14, 20)).convert("RGBA")

    ghost_page(im, (1150, -70, 1720, 420), 36, (255, 255, 255, 10))
    strip(im, H - 8, 8, BLUE)
    paste_logo(im, LOGO, 80, 100, 300)

    d = ImageDraw.Draw(im)
    title = "fralsare-daily"
    f_title = font(BOLD, 92)
    f_sub = font(REG, 38)

    tw, th = text_size(d, title, f_title)
    d.text((460, 96), title, font=f_title, fill=(244, 248, 252, 255))
    d.rectangle([464, 96 + th + 22, 704, 96 + th + 30], fill=BLUE + (255,))

    d.text((462, 232), "Five topics. One window.", font=f_sub, fill=(160, 176, 192, 255))
    d.text((462, 300), "No accounts. No API keys.", font=f_sub, fill=(160, 176, 192, 255))

    out = ROOT / "docs" / "banner.png"
    im.convert("RGB").save(out)
    print(f"wrote {out} ({W}x{H})")


# ---------------------------------------------------------------- banner B
def gen_og():
    W, H = 1200, 630
    im = vgrad(W, H, (250, 250, 251), (240, 245, 250)).convert("RGBA")

    ghost_page(im, (760, -80, 1320, 420), 36, (233, 239, 246, 255))
    strip(im, H - 10, 10, BLUE)
    paste_logo(im, LOGO, 500, 54, 200)

    d = ImageDraw.Draw(im)
    f_title = font(BOLD, 72)
    f_sub = font(REG, 34)
    f_small = font(REG, 27)

    def centered(text, y, fnt, fill):
        w, _ = text_size(d, text, fnt)
        d.text(((W - w) // 2, y), text, font=fnt, fill=fill)

    centered("fralsare-daily", 300, f_title, (16, 24, 38, 255))
    centered("A fast, free, cross-platform", 412, f_sub, (100, 116, 139, 255))
    centered("news reader in five topics.", 460, f_sub, (100, 116, 139, 255))
    centered("Open source  ·  No accounts  ·  No API keys", 528, f_small, (71, 85, 105, 255))

    out = ROOT / "docs" / "og.png"
    im.convert("RGB").save(out)
    print(f"wrote {out} ({W}x{H})")


gen_banner()
gen_og()
