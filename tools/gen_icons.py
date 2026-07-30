#!/usr/bin/env python3
"""PenSoul 品牌图标生成器

设计语言:宣纸底 + 浓墨「笔」字 + 朱砂印章,贴合"书斋雅韵"UI。
生成 Tauri 桌面 / Windows Store / Android / iOS / favicon 全套图标。
"""
import math
import os
import random
import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path("/Users/kimmy/Documents/PenSoul")
APP_ICONS = ROOT / "crates/pensoul-app/icons"
MASTER_OUT = ROOT / "icons"  # 工作区根 icons/,存放源稿

# ── 调色(取自 tokens.css,oklch 近似换算) ──
PAPER_LIGHT = (250, 245, 232)   # 宣纸亮部
PAPER_DEEP = (239, 231, 211)    # 宣纸暗部
PAPER_EDGE = (232, 222, 198)    # 边缘氧化
INK = (31, 26, 20)              # 松烟墨
INK_SOFT = (58, 50, 40)
CINNABAR = (166, 58, 42)        # 朱砂
CINNABAR_DEEP = (141, 46, 33)
SEAL_WHITE = (250, 245, 235)

SONGTI = "/System/Library/Fonts/Supplemental/Songti.ttc"


def load_songti(size: int, bold: bool = True) -> ImageFont.FreeTypeFont:
    """在 Songti.ttc 中寻找合适的字面(优先 Bold)。"""
    chosen = None
    for idx in range(8):
        try:
            f = ImageFont.truetype(SONGTI, size, index=idx)
        except Exception:
            break
        name = " ".join(f.getname())
        if "Songti SC" in name or "Songti" in name:
            if bold and ("Bold" in name or "Black" in name):
                return f
            if chosen is None:
                chosen = f
    if chosen is None:
        chosen = ImageFont.truetype(SONGTI, size, index=0)
    return chosen


def squircle_mask(size: int, radius: int) -> Image.Image:
    m = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(m)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return m


def paper_background(size: int, rounded: bool) -> Image.Image:
    """宣纸底:暖色纵向渐变 + 边缘氧化 + 纤维噪点 + 内框墨线。"""
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        t = y / max(size - 1, 1)
        # 轻微对角感:左亮右暗
        for x in range(size):
            tx = x / max(size - 1, 1)
            k = min(1.0, 0.65 * t + 0.35 * tx)
            r = int(PAPER_LIGHT[0] + (PAPER_DEEP[0] - PAPER_LIGHT[0]) * k)
            g = int(PAPER_LIGHT[1] + (PAPER_DEEP[1] - PAPER_LIGHT[1]) * k)
            b = int(PAPER_LIGHT[2] + (PAPER_DEEP[2] - PAPER_LIGHT[2]) * k)
            px[x, y] = (r, g, b)

    # 边缘氧化晕影
    vign = Image.new("L", (size, size), 0)
    vd = ImageDraw.Draw(vign)
    vd.rectangle([0, 0, size, size], fill=60)
    inner = int(size * 0.14)
    vd.rounded_rectangle([inner, inner, size - inner, size - inner],
                         radius=int(size * 0.22), fill=0)
    vign = vign.filter(ImageFilter.GaussianBlur(size * 0.09))
    edge = Image.new("RGB", (size, size), PAPER_EDGE)
    img = Image.composite(edge, img, vign.point(lambda v: int(v * 0.55)))

    # 纤维噪点
    noise = Image.new("L", (size, size))
    np_ = noise.load()
    rng = random.Random(20260730)
    for y in range(0, size, 2):
        for x in range(0, size, 2):
            np_[x, y] = rng.randint(118, 138)
    noise = noise.resize((size, size)).filter(ImageFilter.GaussianBlur(0.6))
    img = Image.composite(img, Image.new("RGB", (size, size), (255, 252, 244)),
                          noise.point(lambda v: max(0, v - 128)))

    # 内框墨线(装裱感)
    d = ImageDraw.Draw(img)
    m = max(2, int(size * 0.0035))
    inset = int(size * 0.045)
    frame_radius = int(size * 0.17) if rounded else int(size * 0.03)
    d.rounded_rectangle([inset, inset, size - 1 - inset, size - 1 - inset],
                        radius=frame_radius, outline=INK + (), width=m)
    # 降低墨线存在感
    img = img.filter(ImageFilter.GaussianBlur(0.3))

    if rounded:
        out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        out.paste(img, (0, 0), squircle_mask(size, int(size * 0.225)))
        return out
    return img.convert("RGBA")


def draw_glyph(canvas: Image.Image, ch: str, size: int, center, fill,
               bold=True, rotate=0.0, pad_ratio=1.35):
    """在临时层渲染汉字并合成,支持旋转。"""
    font = load_songti(int(size), bold=bold)
    tmp = Image.new("RGBA", (int(size * pad_ratio), int(size * pad_ratio)), (0, 0, 0, 0))
    td = ImageDraw.Draw(tmp)
    bbox = td.textbbox((0, 0), ch, font=font)
    w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
    td.text(((tmp.width - w) / 2 - bbox[0], (tmp.height - h) / 2 - bbox[1]),
            ch, font=font, fill=fill + (255,))
    if rotate:
        tmp = tmp.rotate(rotate, resample=Image.BICUBIC, expand=False)
    canvas.alpha_composite(tmp, (int(center[0] - tmp.width / 2), int(center[1] - tmp.height / 2)))


def ink_bleed(canvas: Image.Image, center, radius, color=INK, alpha=10, count=3):
    """墨韵渗化:极淡的同心晕圈。"""
    layer = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for i in range(count, 0, -1):
        r = radius * (1 + i * 0.06)
        d.ellipse([center[0] - r, center[1] - r, center[0] + r, center[1] + r],
                  fill=color + (alpha,))
    layer = layer.filter(ImageFilter.GaussianBlur(radius * 0.10))
    canvas.alpha_composite(layer)


def compose(size: int, *, rounded: bool, with_seal: bool, master: bool = False) -> Image.Image:
    img = paper_background(size, rounded)

    # 墨韵衬底
    ink_bleed(img, (size * 0.47, size * 0.47), size * 0.34)

    # 主字「笔」
    glyph_size = size * (0.62 if with_seal else 0.66)
    center = (size * (0.485 if with_seal else 0.5), size * (0.475 if with_seal else 0.5))
    # 淡淡的纸下阴影托底
    draw_glyph(img, "笔", glyph_size, (center[0] + size * 0.004, center[1] + size * 0.006),
               (0, 0, 0), bold=True)
    # 用墨迹色覆盖:重绘主字(阴影层已淡,直接主字)
    draw_glyph(img, "笔", glyph_size, center, INK, bold=True)

    if with_seal:
        seal = int(size * 0.235)
        scx, scy = size * 0.745, size * 0.78
        seal_layer = Image.new("RGBA", (seal * 2, seal * 2), (0, 0, 0, 0))
        sd = ImageDraw.Draw(seal_layer)
        rad = int(seal * 0.16)
        # 印章底:朱砂渐变感(两层)
        sd.rounded_rectangle([seal // 2, seal // 2, seal // 2 + seal, seal // 2 + seal],
                             radius=rad, fill=CINNABAR_DEEP + (255,))
        sd.rounded_rectangle([seal // 2, seal // 2, seal // 2 + seal, seal // 2 + seal - max(2, seal // 22)],
                             radius=rad, fill=CINNABAR + (255,))
        # 印章字「印」
        f = load_songti(int(seal * 0.62), bold=True)
        bbox = sd.textbbox((0, 0), "印", font=f)
        w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
        sd.text((seal - w / 2 - bbox[0] + seal * 0.005, seal - h / 2 - bbox[1] - seal * 0.01),
                "印", font=f, fill=SEAL_WHITE + (255,))
        # 做旧:边缘轻微模糊 + 旋转
        seal_layer = seal_layer.filter(ImageFilter.GaussianBlur(0.4))
        seal_layer = seal_layer.rotate(-3.5, resample=Image.BICUBIC, expand=False)
        img.alpha_composite(seal_layer, (int(scx - seal), int(scy - seal)))

    return img


def downscale(img: Image.Image, size: int) -> Image.Image:
    return img.resize((size, size), Image.LANCZOS)


def save_png(img: Image.Image, path: Path, size: int):
    path.parent.mkdir(parents=True, exist_ok=True)
    out = img if img.size == (size, size) else downscale(img, size)
    out.save(path, "PNG")


def main():
    SS = 2  # 超采样
    master_size = 1024 * SS

    # ── 主稿:圆角(桌面/macOS 风格)与方稿(Windows/iOS 全出血) ──
    rounded_master = compose(master_size, rounded=True, with_seal=True, master=True)
    square_master = compose(master_size, rounded=False, with_seal=True, master=True)
    small_master = compose(master_size, rounded=True, with_seal=False)  # 小尺寸简化版

    MASTER_OUT.mkdir(exist_ok=True)
    save_png(rounded_master, MASTER_OUT / "logo-master.png", 1024)
    save_png(square_master, MASTER_OUT / "logo-square.png", 1024)

    # ── Tauri 桌面图标 ──
    save_png(rounded_master, APP_ICONS / "icon.png", 512)
    save_png(rounded_master, APP_ICONS / "256x256.png", 256)
    save_png(rounded_master, APP_ICONS / "128x128.png", 128)
    save_png(rounded_master, APP_ICONS / "128x128@2x.png", 256)
    save_png(rounded_master, APP_ICONS / "64x64.png", 64)
    save_png(small_master, APP_ICONS / "32x32.png", 32)

    # ── Windows Store 方标(全出血) ──
    square_sizes = {
        "Square30x30Logo.png": 30, "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71, "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107, "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150, "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310, "StoreLogo.png": 50,
    }
    for name, s in square_sizes.items():
        src = square_master if s >= 44 else compose(master_size, rounded=False, with_seal=False)
        save_png(src, APP_ICONS / name, s)

    # ── macOS icon.icns ──
    iconset = APP_ICONS / "pensoul.iconset"
    iconset.mkdir(exist_ok=True)
    icns_map = {
        "icon_16x16.png": 16, "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32, "icon_32x32@2x.png": 64,
        "icon_64x64.png": 64, "icon_64x64@2x.png": 128,
        "icon_128x128.png": 128, "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256, "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512, "icon_512x512@2x.png": 1024,
    }
    for name, s in icns_map.items():
        src = rounded_master if s >= 48 else small_master
        save_png(src, iconset / name, s)
    subprocess.run(["iconutil", "-c", "icns", str(iconset),
                    "-o", str(APP_ICONS / "icon.icns")], check=True)
    shutil.rmtree(iconset)

    # ── Windows icon.ico(PIL 以首帧尺寸为上限,最大帧必须在首位) ──
    ico_base = downscale(rounded_master, 256)
    ico_others = [downscale(rounded_master, s) for s in (128, 64, 48, 24)] + \
                 [downscale(small_master, s) for s in (32, 16)]
    ico_base.save(APP_ICONS / "icon.ico", format="ICO",
                  sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
                  append_images=ico_others)

    # ── favicon ──
    pub = ROOT / "public"
    pub.mkdir(exist_ok=True)
    fav_base = downscale(rounded_master, 48)
    fav_others = [downscale(small_master, s) for s in (32, 16)]
    fav_base.save(pub / "favicon.ico", format="ICO",
                  sizes=[(16, 16), (32, 32), (48, 48)],
                  append_images=fav_others)

    # ── Android ──
    densities = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
    for dens, s in densities.items():
        d = APP_ICONS / f"android/mipmap-{dens}"
        save_png(square_master, d / "ic_launcher.png", s)
        # 圆形版
        circ = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        mask = Image.new("L", (s, s), 0)
        ImageDraw.Draw(mask).ellipse([0, 0, s, s], fill=255)
        circ.paste(downscale(square_master, s), (0, 0), mask)
        circ.save(d / "ic_launcher_round.png", "PNG")
        # adaptive foreground:透明底,字形缩到安全区 66%
        fg_size = s * 3  # foreground 以 108dp 基准 ×3
        fg = Image.new("RGBA", (fg_size, fg_size), (0, 0, 0, 0))
        inner = compose(master_size, rounded=False, with_seal=True)
        inner = downscale(inner, int(fg_size * 0.62))
        fg.alpha_composite(inner, ((fg_size - inner.width) // 2, (fg_size - inner.height) // 2))
        fg.save(d / "ic_launcher_foreground.png", "PNG")

    # ── iOS(不透明、全出血方稿) ──
    ios_dir = APP_ICONS / "ios"
    for f in ios_dir.glob("AppIcon-*.png"):
        stem = f.stem.replace("AppIcon-", "")
        base = stem.split("@")[0]           # e.g. 20x20 / 512
        scale = stem.split("@")[1].replace("x", "") if "@" in stem else "1x"
        scale = int(scale.replace("-1", "")) if "-" in scale else int(scale)
        if "x" in base:
            pts = float(base.split("x")[0])
        else:
            pts = float(base)               # 512@2x → 1024
        px = int(round(pts * scale))
        save_png(square_master, f, px)

    print("图标生成完成")


if __name__ == "__main__":
    main()
