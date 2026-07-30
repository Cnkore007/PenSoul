#!/usr/bin/env python3
"""PenSoul Logo — 纯抽象变体:不见笔形,只留笔意与魂气。"""
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

import sys
sys.path.insert(0, str(Path(__file__).parent))
from gen_concepts import (cubic, paper_bg, soft_shadow, songti,
                          INK, INK_2, CINNABAR, CINNABAR_DEEP)

ROOT = Path("/Users/kimmy/Documents/PenSoul")
OUT = ROOT / "icons"


def ink_gradient(S):
    grad = Image.new("RGB", (1, 256))
    for i in range(256):
        t = i / 255
        grad.putpixel((0, i), tuple(int(INK_2[c] + (INK[c] - INK_2[c]) * t) for c in range(3)))
    return grad.resize((S, S)).convert("RGBA")


def stamp_stroke(pts_radii, color):
    S_local = None
    return pts_radii  # placeholder


# ── B1 圆相:一笔圈起的圆,缺口处一点朱砂,魂不自满 ──
def concept_b1(S):
    img = paper_bg(S)
    cx, cy, R = S * 0.5, S * 0.52, S * 0.265

    # 圆环开口:40°..95°(右上)
    gap_a, gap_b = math.radians(38), math.radians(96)
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ld = ImageDraw.Draw(layer)
    steps = 420
    for i in range(steps + 1):
        th = math.radians(95) + (math.radians(398) - math.radians(95)) * (i / steps)
        # 宽度:起笔细→行笔厚→收笔出锋
        t = i / steps
        w = S * (0.012 + 0.052 * math.sin(min(1.0, t * 1.12) * math.pi) ** 0.8)
        if t < 0.05:
            w *= 0.35 + 0.65 * (t / 0.05)
        if t > 0.90:
            w *= max(0.15, (1 - t) / 0.10) ** 0.7
        # 半径微颤,带手写感
        rr = R + math.sin(th * 3 + 1.2) * S * 0.006
        x, y = cx + rr * math.cos(th), cy + rr * math.sin(th)
        ld.ellipse([x - w / 2, y - w / 2, x + w / 2, y + w / 2], fill=INK + (255,))
    layer = layer.filter(ImageFilter.GaussianBlur(S * 0.0008))
    soft_shadow(img, layer.split()[3], S * 0.005, int(S * 0.009))
    img.alpha_composite(layer)

    # 缺口处的朱砂点:将满未满,魂之所在
    dth = math.radians(66)
    dot_r = S * 0.034
    dx = cx + (R + S * 0.012) * math.cos(dth)
    dy = cy - (R + S * 0.012) * math.sin(dth)
    dm = Image.new("L", (S, S), 0)
    ImageDraw.Draw(dm).ellipse([dx - dot_r, dy - dot_r, dx + dot_r, dy + dot_r], fill=255)
    soft_shadow(img, dm, S * 0.004, int(S * 0.005), alpha=50)
    ImageDraw.Draw(img).ellipse([dx - dot_r, dy - dot_r * 0.97, dx + dot_r, dy + dot_r * 0.97],
                                fill=CINNABAR + (255,))
    return img


# ── B2 一点魂:一笔独立的笔意,朱点悬于锋上,笔到魂生 ──
def concept_b2(S):
    img = paper_bg(S)
    center = cubic((S * 0.40, S * 0.74), (S * 0.46, S * 0.58),
                   (S * 0.56, S * 0.44), (S * 0.60, S * 0.30), n=160)
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ld = ImageDraw.Draw(layer)
    for i, p in enumerate(center):
        t = i / (len(center) - 1)
        r = S * (0.008 + 0.046 * math.sin(min(1.0, t * 1.05) * math.pi) ** 0.85)
        if t > 0.85:
            r *= max(0.10, (1 - t) / 0.15) ** 0.75
        if t < 0.05:
            r *= 0.4 + 0.6 * (t / 0.05)
        ld.ellipse([p[0] - r, p[1] - r, p[0] + r, p[1] + r], fill=INK + (255,))
    layer = layer.filter(ImageFilter.GaussianBlur(S * 0.0008))
    soft_shadow(img, layer.split()[3], S * 0.005, int(S * 0.010))
    img.alpha_composite(layer)

    # 悬于锋上的朱点,似落未落
    dot_r = S * 0.033
    dx, dy = S * 0.635, S * 0.225
    dm = Image.new("L", (S, S), 0)
    ImageDraw.Draw(dm).ellipse([dx - dot_r, dy - dot_r, dx + dot_r, dy + dot_r], fill=255)
    soft_shadow(img, dm, S * 0.004, int(S * 0.005), alpha=50)
    ImageDraw.Draw(img).ellipse([dx - dot_r, dy - dot_r * 0.97, dx + dot_r, dy + dot_r * 0.97],
                                fill=CINNABAR + (255,))
    # 一颗更小的墨点相伴
    ImageDraw.Draw(img).ellipse([S * 0.545 - S * 0.007, S * 0.215 - S * 0.007,
                                 S * 0.545 + S * 0.007, S * 0.215 + S * 0.007],
                                fill=INK + (150,))
    return img


# ── B3 墨抱朱:一团有机墨韵,怀里一点朱砂魂 ──
def concept_b3(S):
    img = paper_bg(S)
    cx, cy = S * 0.5, S * 0.52

    # 有机墨团:半径随角度平滑起伏,形如坠墨
    rng = random.Random(7)
    harmonics = [(rng.uniform(1.5, 3.5), rng.uniform(0.010, 0.030), rng.uniform(0, 6.28))
                 for _ in range(3)]
    pts = []
    for i in range(240):
        th = 2 * math.pi * i / 240
        r = S * 0.245
        for freq, amp, phase in harmonics:
            r += S * amp * math.sin(freq * th + phase)
        # 整体略呈水滴,上收下放
        r *= 1 - 0.13 * math.cos(th - 0.35)
        pts.append((cx + r * math.cos(th), cy + r * math.sin(th)))
    m = Image.new("L", (S, S), 0)
    ImageDraw.Draw(m).polygon(pts, fill=255)
    m = m.filter(ImageFilter.GaussianBlur(S * 0.002))
    soft_shadow(img, m, S * 0.006, int(S * 0.012))
    inked = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    inked.paste(ink_gradient(S), (0, 0), m)
    img.alpha_composite(inked)

    # 怀中朱砂:有机水滴形,微微左倾
    fcx, fcy = cx + S * 0.01, cy + S * 0.015
    fh, fw = S * 0.16, S * 0.075
    tear = []
    for i in range(120):
        t = i / 119
        th = -math.pi / 2 + 2 * math.pi * t
        rr = 1 - 0.42 * math.cos(th)  # 水滴:上尖下圆
        x = fcx + fw * rr * math.cos(th)
        y = fcy + fh * 0.62 * rr * math.sin(th)
        tear.append((x, y))
    d = ImageDraw.Draw(img)
    d.polygon(tear, fill=CINNABAR + (255,))
    # 朱砂晕
    halo = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(halo).polygon(tear, fill=CINNABAR + (90,))
    halo = halo.filter(ImageFilter.GaussianBlur(S * 0.012))
    img.alpha_composite(halo)
    d.polygon(tear, fill=CINNABAR + (255,))

    # 两粒溅墨卫星
    for (mx, my, mr) in [(0.715, 0.36, 0.011), (0.30, 0.63, 0.008)]:
        d.ellipse([S * mx - S * mr, S * my - S * mr, S * mx + S * mr, S * my + S * mr],
                  fill=INK + (170,))
    return img


def main():
    S = 1024
    makers = [("b1", "B1 · 圆相", concept_b1),
              ("b2", "B2 · 一点魂", concept_b2),
              ("b3", "B3 · 墨抱朱", concept_b3)]
    icons = []
    for key, label, fn in makers:
        ic = fn(S).resize((512, 512), Image.LANCZOS)
        ic.save(OUT / f"concept-{key}.png")
        icons.append((label, ic))

    pad, label_h = 48, 88
    W = pad * 4 + 512 * 3
    H = pad * 2 + 512 + label_h
    sheet = Image.new("RGB", (W, H), (238, 232, 218))
    f = songti(40)
    for i, (label, ic) in enumerate(icons):
        x = pad + i * (512 + pad)
        sh = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        ImageDraw.Draw(sh).rounded_rectangle([6, 10, 518, 522], radius=116, fill=(60, 50, 40, 60))
        sh = sh.filter(ImageFilter.GaussianBlur(10))
        sheet.paste(sh, (x, pad), sh)
        sheet.paste(ic, (x, pad), ic)
        d = ImageDraw.Draw(sheet)
        bb = d.textbbox((0, 0), label, font=f)
        d.text((x + 256 - (bb[2] - bb[0]) / 2, pad + 512 + 26), label, font=f, fill=(58, 50, 40))
    sheet.save(OUT / "concepts-b.png")
    print("变体已输出: icons/concepts-b.png + concept-b1/b2/b3.png")


if __name__ == "__main__":
    main()
