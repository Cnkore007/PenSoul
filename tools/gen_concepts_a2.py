#!/usr/bin/env python3
"""PenSoul Logo — 方案 A 的抽象化变体:笔尖魂焰,求"神"不求"形"。"""
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

import sys
sys.path.insert(0, str(Path(__file__).parent))
from gen_concepts import (cubic, flame_pts, paper_bg, soft_shadow, songti,
                          INK, INK_2, CINNABAR, CINNABAR_DEEP)

ROOT = Path("/Users/kimmy/Documents/PenSoul")
OUT = ROOT / "icons"


def nib_mask(S, xl0, xr0, y0, y1, lean=0.0):
    """修长笔尖轮廓;lean 控制尖端前倾。"""
    tipx = S * (0.5 + lean)
    left = cubic((S * xl0, y0 + S * 0.02), (S * (xl0 - 0.030), S * 0.42),
                 (S * 0.36, S * 0.66), (tipx, y1))
    right = cubic((tipx, y1), (S * 0.64, S * 0.66),
                  (S * (xr0 + 0.030), S * 0.42), (S * xr0, y0 + S * 0.02))
    top = cubic((S * xr0, y0 + S * 0.02), (S * 0.62, y0 - S * 0.030),
                (S * 0.38, y0 - S * 0.030), (S * xl0, y0 + S * 0.02))
    m = Image.new("L", (S, S), 0)
    ImageDraw.Draw(m).polygon(left + right + top, fill=255)
    return m.filter(ImageFilter.GaussianBlur(S * 0.0012)), tipx


def ink_gradient(S):
    grad = Image.new("RGB", (1, 256))
    for i in range(256):
        t = i / 255
        grad.putpixel((0, i), tuple(int(INK_2[c] + (INK[c] - INK_2[c]) * t) for c in range(3)))
    return grad.resize((S, S)).convert("RGBA")


def cinnabar_gradient(S):
    grad = Image.new("RGB", (1, 256))
    for i in range(256):
        t = i / 255
        grad.putpixel((0, i), tuple(int(CINNABAR[c] + (CINNABAR_DEEP[c] - CINNABAR[c]) * t) for c in range(3)))
    return grad.resize((S, S)).convert("RGBA")


def draw_flame(img, S, cx, base_y, h, w, bend, glow=True):
    pts = flame_pts(cx, base_y, h, w, bend=bend)
    m = Image.new("L", (S, S), 0)
    ImageDraw.Draw(m).polygon(pts, fill=255)
    m = m.filter(ImageFilter.GaussianBlur(S * 0.0012))
    if glow:
        glow_m = m.filter(ImageFilter.GaussianBlur(S * 0.012))
        glow_img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        glow_img.paste(Image.new("RGBA", (S, S), CINNABAR + (46,)), (0, 0),
                       glow_m.point(lambda v: min(255, int(v * 0.55))))
        img.alpha_composite(glow_img)
    soft_shadow(img, m, S * 0.004, int(S * 0.005), alpha=50)
    fl = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    fl.paste(cinnabar_gradient(S), (0, 0), m)
    img.alpha_composite(fl)


# ── A1 飞焰:斜笔如翼,魂焰离尖而起 ──
def concept_a1(S):
    img = paper_bg(S)

    # 背景一抹飞白墨弧(动势)
    arc = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(arc).arc([S * 0.10, S * 0.10, S * 0.90, S * 0.90],
                            start=70, end=205, fill=INK + (255,), width=int(S * 0.045))
    arc = arc.filter(ImageFilter.GaussianBlur(S * 0.02))
    img.alpha_composite(arc.point(lambda v: v) if False else
                        Image.composite(arc, Image.new("RGBA", (S, S), (0, 0, 0, 0)),
                                        arc.split()[3].point(lambda v: int(v * 0.10))))

    # 笔尖(斜置 14°)
    m, tipx = nib_mask(S, 0.365, 0.635, S * 0.225, S * 0.815, lean=0.008)
    nib_layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    # 中缝
    d = ImageDraw.Draw(nib_layer)
    slit_w = max(2, S * 0.005)
    d.polygon([(tipx - slit_w / 2, S * 0.81), (tipx + slit_w / 2, S * 0.81),
               (tipx + slit_w * 0.3, S * 0.50), (tipx - slit_w * 0.3, S * 0.50)],
              fill=(245, 239, 226, 255))
    slit = nib_layer.split()[3]
    inked = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    inked.paste(ink_gradient(S), (0, 0), m)
    inked.putalpha(Image.composite(m.point(lambda v: 0), m, slit))  # 中缝镂空
    rot = inked.rotate(14, resample=Image.BICUBIC, center=(S * 0.5, S * 0.52))
    sh = rot.split()[3]
    soft_shadow(img, sh, S * 0.006, int(S * 0.012))
    img.alpha_composite(rot)

    # 魂焰:离尖而起,带明显 S 形挑势
    draw_flame(img, S, S * 0.505, S * 0.455, S * 0.205, S * 0.058, bend=S * 0.030)
    # 一粒小火星
    ImageDraw.Draw(img).ellipse([S * 0.560 - S * 0.008, S * 0.250 - S * 0.008,
                                 S * 0.560 + S * 0.008, S * 0.250 + S * 0.008],
                                fill=CINNABAR + (220,))
    return img


# ── A2 菱焰:笔尖抽象为菱形,焰为负形,一点朱砂逃逸而上 ──
def concept_a2(S):
    img = paper_bg(S)

    kite = [(S * 0.5, S * 0.205), (S * 0.650, S * 0.45),
            (S * 0.5, S * 0.815), (S * 0.350, S * 0.45)]
    m = Image.new("L", (S, S), 0)
    ImageDraw.Draw(m).polygon(kite, fill=255)
    m = m.filter(ImageFilter.GaussianBlur(S * 0.0015))
    soft_shadow(img, m, S * 0.006, int(S * 0.012))
    inked = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    inked.paste(ink_gradient(S), (0, 0), m)

    # 负形焰(纸色镂空,魂在笔中)
    fpts = flame_pts(S * 0.5, S * 0.565, S * 0.16, S * 0.052, bend=S * 0.010)
    fm = Image.new("L", (S, S), 0)
    ImageDraw.Draw(fm).polygon(fpts, fill=255)
    fm = fm.filter(ImageFilter.GaussianBlur(S * 0.001))
    alpha = inked.split()[3]
    inked.putalpha(Image.composite(alpha.point(lambda v: 0), alpha, fm))
    img.alpha_composite(inked)

    # 中缝:焰根至尖
    d = ImageDraw.Draw(img)
    slit_w = max(2, S * 0.005)
    d.polygon([(S * 0.5 - slit_w / 2, S * 0.808), (S * 0.5 + slit_w / 2, S * 0.808),
               (S * 0.5 + slit_w * 0.3, S * 0.585), (S * 0.5 - slit_w * 0.3, S * 0.585)],
              fill=(245, 239, 226, 255))

    # 逃逸的朱砂小焰(灵魂出窍之势)
    draw_flame(img, S, S * 0.520, S * 0.175, S * 0.105, S * 0.034, bend=S * 0.016)
    return img


# ── A3 翎焰:一笔斜挑如翎,锋芒尽头化焰 ──
def concept_a3(S):
    img = paper_bg(S)

    # 两条极淡的伴笔(飞白动势)
    for off, alpha in [(0.045, 28), (-0.045, 20)]:
        echo = cubic((S * (0.335 + off), S * 0.745), (S * (0.44 + off), S * 0.55),
                     (S * (0.55 + off), S * 0.42), (S * (0.60 + off), S * 0.30), n=80)
        layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        ld = ImageDraw.Draw(layer)
        for i, p in enumerate(echo):
            t = i / (len(echo) - 1)
            r = S * 0.009 * math.sin(t * math.pi)
            if r > 0.5:
                ld.ellipse([p[0] - r, p[1] - r, p[0] + r, p[1] + r], fill=INK + (alpha,))
        layer = layer.filter(ImageFilter.GaussianBlur(S * 0.003))
        img.alpha_composite(layer)

    # 主笔:斜挑,中锋饱满,两端出锋
    center = cubic((S * 0.335, S * 0.755), (S * 0.44, S * 0.56),
                   (S * 0.55, S * 0.42), (S * 0.615, S * 0.285), n=160)
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ld = ImageDraw.Draw(layer)
    for i, p in enumerate(center):
        t = i / (len(center) - 1)
        r = S * (0.006 + 0.050 * math.sin(min(1.0, t * 1.06) * math.pi) ** 0.85)
        if t > 0.86:
            r *= max(0.10, (1 - t) / 0.14) ** 0.75
        if t < 0.05:
            r *= 0.4 + 0.6 * (t / 0.05)
        ld.ellipse([p[0] - r, p[1] - r, p[0] + r, p[1] + r], fill=INK + (255,))
    layer = layer.filter(ImageFilter.GaussianBlur(S * 0.0008))
    soft_shadow(img, layer.split()[3], S * 0.006, int(S * 0.010))
    img.alpha_composite(layer)

    # 锋芒尽头:朱砂焰顺势上挑
    fx, fy = S * 0.615, S * 0.300
    draw_flame(img, S, fx + S * 0.010, fy, S * 0.185, S * 0.052, bend=S * 0.034)
    ImageDraw.Draw(img).ellipse([S * 0.685 - S * 0.007, S * 0.175 - S * 0.007,
                                 S * 0.685 + S * 0.007, S * 0.175 + S * 0.007],
                                fill=CINNABAR + (200,))
    return img


def main():
    S = 1024
    makers = [("a1", "A1 · 飞焰", concept_a1),
              ("a2", "A2 · 菱焰", concept_a2),
              ("a3", "A3 · 翎焰", concept_a3)]
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
    sheet.save(OUT / "concepts-a.png")
    print("变体已输出: icons/concepts-a.png + concept-a1/a2/a3.png")


if __name__ == "__main__":
    main()
