#!/usr/bin/env python3
"""PenSoul Logo 概念稿:笔 × 魂 的三个融合方向,输出对比图供挑选。"""
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path("/Users/kimmy/Documents/PenSoul")
OUT = ROOT / "icons"

PAPER_LIGHT = (250, 245, 232)
PAPER_DEEP = (239, 231, 211)
PAPER_EDGE = (232, 222, 198)
INK = (31, 26, 20)
INK_2 = (52, 44, 35)
CINNABAR = (166, 58, 42)
CINNABAR_DEEP = (141, 46, 33)
SONGTI = "/System/Library/Fonts/Supplemental/Songti.ttc"


def songti(size, bold=True):
    chosen = None
    for idx in range(8):
        try:
            f = ImageFont.truetype(SONGTI, size, index=idx)
        except Exception:
            break
        name = " ".join(f.getname())
        if "Songti" in name:
            if bold and ("Bold" in name or "Black" in name):
                return f
            chosen = chosen or f
    return chosen or ImageFont.truetype(SONGTI, size)


def cubic(p0, p1, p2, p3, n=80):
    pts = []
    for i in range(n + 1):
        t = i / n
        mt = 1 - t
        x = mt**3 * p0[0] + 3 * mt**2 * t * p1[0] + 3 * mt * t**2 * p2[0] + t**3 * p3[0]
        y = mt**3 * p0[1] + 3 * mt**2 * t * p1[1] + 3 * mt * t**2 * p2[1] + t**3 * p3[1]
        pts.append((x, y))
    return pts


def paper_bg(size, rounded=True):
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        t = y / max(size - 1, 1)
        for x in range(size):
            tx = x / max(size - 1, 1)
            k = min(1.0, 0.65 * t + 0.35 * tx)
            px[x, y] = tuple(int(PAPER_LIGHT[i] + (PAPER_DEEP[i] - PAPER_LIGHT[i]) * k) for i in range(3))
    vign = Image.new("L", (size, size), 0)
    vd = ImageDraw.Draw(vign)
    vd.rectangle([0, 0, size, size], fill=60)
    inner = int(size * 0.14)
    vd.rounded_rectangle([inner, inner, size - inner, size - inner], radius=int(size * 0.22), fill=0)
    vign = vign.filter(ImageFilter.GaussianBlur(size * 0.09))
    img = Image.composite(Image.new("RGB", (size, size), PAPER_EDGE), img,
                          vign.point(lambda v: int(v * 0.55)))
    noise = Image.new("L", (size, size))
    np_ = noise.load()
    rng = random.Random(20260730)
    for y in range(0, size, 2):
        for x in range(0, size, 2):
            np_[x, y] = rng.randint(118, 138)
    noise = noise.filter(ImageFilter.GaussianBlur(0.6))
    img = Image.composite(img, Image.new("RGB", (size, size), (255, 252, 244)),
                          noise.point(lambda v: max(0, v - 128)))
    d = ImageDraw.Draw(img)
    m = max(2, int(size * 0.0035))
    inset = int(size * 0.045)
    d.rounded_rectangle([inset, inset, size - 1 - inset, size - 1 - inset],
                        radius=int(size * 0.17), outline=INK, width=m)
    img = img.filter(ImageFilter.GaussianBlur(0.3))
    if rounded:
        mask = Image.new("L", (size, size), 0)
        ImageDraw.Draw(mask).rounded_rectangle([0, 0, size - 1, size - 1],
                                               radius=int(size * 0.225), fill=255)
        out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        out.paste(img, (0, 0), mask)
        return out
    return img.convert("RGBA")


def flame_pts(cx, cy_bottom, h, w, bend=0.0):
    """火苗轮廓:左弧上挑,尖部弯曲,右弧带回勾。"""
    tip = (cx + bend, cy_bottom - h)
    left = cubic((cx, cy_bottom),
                 (cx - 1.05 * w, cy_bottom - 0.30 * h),
                 (cx - 0.72 * w, cy_bottom - 0.72 * h),
                 tip)
    right = cubic(tip,
                  (cx + 0.30 * w + bend * 0.4, cy_bottom - 0.92 * h),
                  (cx + 0.80 * w, cy_bottom - 0.50 * h),
                  (cx + 0.42 * w, cy_bottom - 0.16 * h))
    bottom = cubic((cx + 0.42 * w, cy_bottom - 0.16 * h),
                   (cx + 0.30 * w, cy_bottom + 0.10 * h),
                   (cx + 0.05 * w, cy_bottom + 0.06 * h),
                   (cx, cy_bottom))
    return left + right + bottom


def soft_shadow(canvas, mask, blur, dy, alpha=70):
    sh = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    black = Image.new("RGBA", canvas.size, (20, 15, 10, alpha))
    sh.paste(black, (0, dy), mask)
    sh = sh.filter(ImageFilter.GaussianBlur(blur))
    canvas.alpha_composite(sh)


# ── 方案 A:钢笔尖,呼吸孔化作朱砂魂焰 ──
def concept_a(S):
    img = paper_bg(S)
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    # 笔尖轮廓(朝下)
    y0, y1 = S * 0.215, S * 0.795
    xl0, xr0 = S * 0.335, S * 0.665
    tipx = S * 0.5
    left = cubic((xl0, y0 + S * 0.02), (S * 0.300, S * 0.42), (S * 0.355, S * 0.66), (tipx, y1))
    right = cubic((tipx, y1), (S * 0.645, S * 0.66), (S * 0.700, S * 0.42), (xr0, y0 + S * 0.02))
    top = cubic((xr0, y0 + S * 0.02), (S * 0.62, y0 - S * 0.025), (S * 0.38, y0 - S * 0.025), (xl0, y0 + S * 0.02))
    nib = left + right + top

    nib_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(nib_mask).polygon(nib, fill=255)
    nib_mask = nib_mask.filter(ImageFilter.GaussianBlur(S * 0.0012))
    soft_shadow(img, nib_mask, S * 0.006, int(S * 0.012))

    # 笔尖本体:墨的纵向渐变
    nib_img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    grad = Image.new("RGB", (1, 256))
    for i in range(256):
        t = i / 255
        grad.putpixel((0, i), tuple(int(INK_2[c] + (INK[c] - INK_2[c]) * t) for c in range(3)))
    grad = grad.resize((S, S)).convert("RGBA")
    nib_img.paste(grad, (0, 0), nib_mask)
    img.alpha_composite(nib_img)

    # 中缝(从尖到焰根)
    d2 = ImageDraw.Draw(img)
    slit_top = S * 0.50
    slit_w = max(2, S * 0.006)
    d2.polygon([(tipx - slit_w / 2, y1 - S * 0.008), (tipx + slit_w / 2, y1 - S * 0.008),
                (tipx + slit_w * 0.3, slit_top), (tipx - slit_w * 0.3, slit_top)],
               fill=(245, 239, 226, 255))

    # 魂焰(呼吸孔位置,朱砂)
    fh, fw = S * 0.155, S * 0.062
    fcx, fb = tipx, S * 0.475
    flame = flame_pts(fcx, fb, fh, fw, bend=S * 0.012)
    flame_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(flame_mask).polygon(flame, fill=255)
    flame_mask = flame_mask.filter(ImageFilter.GaussianBlur(S * 0.0012))
    soft_shadow(img, flame_mask, S * 0.004, int(S * 0.006), alpha=60)
    fgrad = Image.new("RGB", (1, 256))
    for i in range(256):
        t = i / 255
        fgrad.putpixel((0, i), tuple(int(CINNABAR[c] + (CINNABAR_DEEP[c] - CINNABAR[c]) * t) for c in range(3)))
    fgrad = fgrad.resize((S, S)).convert("RGBA")
    fl = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    fl.paste(fgrad, (0, 0), flame_mask)
    img.alpha_composite(fl)

    return img


# ── 方案 B:一笔墨魂,竖钩化烟,朱砂点魂 ──
def concept_b(S):
    img = paper_bg(S)

    def stroke_layer(pts_radii, color):
        layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        ld = ImageDraw.Draw(layer)
        for (x, y), r in pts_radii:
            ld.ellipse([x - r, y - r, x + r, y + r], fill=color)
        return layer

    # 主笔:自右上起笔,中锋下行,底端出锋
    centerline = cubic((S * 0.545, S * 0.265), (S * 0.505, S * 0.42),
                       (S * 0.575, S * 0.58), (S * 0.50, S * 0.795), n=140)
    pr = []
    n = len(centerline)
    for i, p in enumerate(centerline):
        t = i / (n - 1)
        r = S * (0.012 + 0.052 * math.sin(min(1.0, t * 1.15) * math.pi) ** 0.9)
        if t > 0.82:  # 出锋
            r *= max(0.12, (1 - t) / 0.18) ** 0.8
        if t < 0.06:  # 起笔藏锋
            r *= 0.55 + 0.45 * (t / 0.06)
        pr.append((p, max(S * 0.004, r)))
    stroke = stroke_layer(pr, INK + (255,))
    stroke = stroke.filter(ImageFilter.GaussianBlur(S * 0.0008))

    mask = stroke.split()[3]
    soft_shadow(img, mask, S * 0.006, int(S * 0.010))
    img.alpha_composite(stroke)

    # 墨烟(魂):从顶端飘出两缕,渐隐
    smoke = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    sd = ImageDraw.Draw(smoke)
    for k, (bend, r0, alpha0) in enumerate([(0.16, 0.020, 110), (0.30, 0.013, 80)]):
        wisp = cubic((S * 0.545, S * 0.27), (S * (0.56 + bend * 0.3), S * 0.20),
                     (S * (0.52 + bend * 0.8), S * 0.16), (S * (0.55 + bend), S * 0.075), n=60)
        for i, p in enumerate(wisp):
            t = min(1.0, i / (len(wisp) - 1))
            r = S * r0 * (1 - t * 0.75)
            a = int(alpha0 * (1 - t) ** 1.4)
            sd.ellipse([p[0] - r, p[1] - r, p[0] + r, p[1] + r], fill=INK + (a,))
    smoke = smoke.filter(ImageFilter.GaussianBlur(S * 0.004))
    img.alpha_composite(smoke)

    # 朱砂点(魂印之点)落在右下,如钤印前的一点朱
    d2 = ImageDraw.Draw(img)
    dot_c = (S * 0.665, S * 0.685)
    dot_r = S * 0.036
    dot_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(dot_mask).ellipse([dot_c[0] - dot_r, dot_c[1] - dot_r,
                                      dot_c[0] + dot_r, dot_c[1] + dot_r], fill=255)
    soft_shadow(img, dot_mask, S * 0.004, int(S * 0.006), alpha=60)
    d2.ellipse([dot_c[0] - dot_r, dot_c[1] - dot_r * 0.96,
                dot_c[0] + dot_r, dot_c[1] + dot_r * 0.96], fill=CINNABAR + (255,))

    # 几颗飞白墨点
    for (mx, my, mr) in [(0.415, 0.36, 0.006), (0.63, 0.46, 0.005), (0.44, 0.63, 0.004)]:
        d2.ellipse([S * mx - S * mr, S * my - S * mr, S * mx + S * mr, S * my + S * mr],
                   fill=INK + (140,))
    return img


# ── 方案 C:笔尖藏灵,负形小魂栖于笔中 ──
def concept_c(S):
    img = paper_bg(S)
    # 笔尖(同 A,略矮)
    y0, y1 = S * 0.235, S * 0.78
    xl0, xr0 = S * 0.345, S * 0.655
    tipx = S * 0.5
    left = cubic((xl0, y0 + S * 0.02), (S * 0.312, S * 0.42), (S * 0.365, S * 0.64), (tipx, y1))
    right = cubic((tipx, y1), (S * 0.635, S * 0.64), (S * 0.688, S * 0.42), (xr0, y0 + S * 0.02))
    top = cubic((xr0, y0 + S * 0.02), (S * 0.62, y0 - S * 0.025), (S * 0.38, y0 - S * 0.025), (xl0, y0 + S * 0.02))
    nib = left + right + top
    nib_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(nib_mask).polygon(nib, fill=255)
    nib_mask = nib_mask.filter(ImageFilter.GaussianBlur(S * 0.0012))
    soft_shadow(img, nib_mask, S * 0.006, int(S * 0.012))
    nib_img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    grad = Image.new("RGB", (1, 256))
    for i in range(256):
        t = i / 255
        grad.putpixel((0, i), tuple(int(INK_2[c] + (INK[c] - INK_2[c]) * t) for c in range(3)))
    grad = grad.resize((S, S)).convert("RGBA")
    nib_img.paste(grad, (0, 0), nib_mask)
    img.alpha_composite(nib_img)

    # 小魂灵(负形,纸色)
    gcx, gcy = S * 0.5, S * 0.475
    gr = S * 0.085
    ghost = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    gd = ImageDraw.Draw(ghost)
    paper_tone = (246, 240, 227, 255)
    gd.ellipse([gcx - gr, gcy - gr * 1.05, gcx + gr, gcy + gr * 0.35], fill=paper_tone)
    gd.rectangle([gcx - gr, gcy - gr * 0.1, gcx + gr, gcy + gr * 0.72], fill=paper_tone)
    # 底部波浪
    bumps = 3
    for i in range(bumps):
        bx = gcx - gr + gr * (1 + 2 * i) / bumps
        gd.ellipse([bx - gr / bumps, gcy + gr * 0.42, bx + gr / bumps, gcy + gr * 1.02],
                   fill=paper_tone)
    # 眼睛(墨色)
    er = gr * 0.13
    for ex in (-gr * 0.36, gr * 0.36):
        gd.ellipse([gcx + ex - er, gcy - gr * 0.32 - er * 1.3,
                    gcx + ex + er, gcy - gr * 0.32 + er * 1.3], fill=INK + (255,))
    ghost_mask = ghost.split()[3]
    soft_shadow(img, ghost_mask, S * 0.003, int(S * 0.004), alpha=40)
    img.alpha_composite(ghost)

    # 一缕朱砂小焰在头顶
    fh, fw = S * 0.075, S * 0.030
    flame = flame_pts(gcx, S * 0.335, fh, fw, bend=S * 0.008)
    ImageDraw.Draw(img).polygon(flame, fill=CINNABAR + (255,))

    # 中缝
    d2 = ImageDraw.Draw(img)
    slit_top = gcy + gr * 0.95
    slit_w = max(2, S * 0.0055)
    d2.polygon([(tipx - slit_w / 2, y1 - S * 0.006), (tipx + slit_w / 2, y1 - S * 0.006),
                (tipx + slit_w * 0.3, slit_top), (tipx - slit_w * 0.3, slit_top)],
               fill=(245, 239, 226, 255))
    return img


def main():
    SS = 2
    S = 512 * SS
    makers = [("a", "方案 A · 笔尖魂焰", concept_a),
              ("b", "方案 B · 一笔墨魂", concept_b),
              ("c", "方案 C · 笔尖藏灵", concept_c)]
    icons = []
    for key, label, fn in makers:
        ic = fn(S).resize((512, 512), Image.LANCZOS)
        ic.save(OUT / f"concept-{key}.png")
        icons.append((label, ic))

    # 对比图
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
    sheet.save(OUT / "concepts.png")
    print("概念稿已输出: icons/concepts.png + concept-a/b/c.png")


if __name__ == "__main__":
    main()
