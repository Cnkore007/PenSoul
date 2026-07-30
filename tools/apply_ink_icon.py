#!/usr/bin/env python3
"""以 ink-wisp.png(水墨朱砂烟)为正稿,去除 AI 水印并重出全套平台图标。"""
import random
import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path("/Users/kimmy/Documents/PenSoul")
APP_ICONS = ROOT / "crates/pensoul-app/icons"
SRC = ROOT / "icons/ink-wisp.png"


def squircle_mask(size, radius_ratio=0.225):
    m = Image.new("L", (size, size), 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, size - 1, size - 1],
                                        radius=int(size * radius_ratio), fill=255)
    return m


def clean_master() -> Image.Image:
    """裁边:左右上 5%,底部裁至水印之上,再回正 1024 方稿。"""
    img = Image.open(SRC).convert("RGB")
    W = img.width
    c = int(W * 0.05)
    # 水印在左下 y≈0.92 以下,直接裁掉;非方形区拉伸回 1024(水墨画面对微拉伸不敏感)
    img = img.crop((c, c, W - c, int(W * 0.915))).resize((1024, 1024), Image.LANCZOS)
    return img


def downscale(img, size):
    return img.resize((size, size), Image.LANCZOS)


def save_png(img, path, size):
    path.parent.mkdir(parents=True, exist_ok=True)
    out = img if img.size == (size, size) else downscale(img, size)
    out.save(path, "PNG")


def main():
    square = clean_master().convert("RGBA")

    # 圆角稿(macOS/桌面风格,角外透明)
    rounded = Image.new("RGBA", square.size, (0, 0, 0, 0))
    rounded.paste(square, (0, 0), squircle_mask(square.width))

    # 小尺寸特写稿:只留笔尖 + 朱砂烟,保证 32px 可读
    S = square.width
    small = square.crop((int(S * 0.36), int(S * 0.01), int(S * 0.74), int(S * 0.39)))
    small = small.resize((1024, 1024), Image.LANCZOS)
    small_r = Image.new("RGBA", small.size, (0, 0, 0, 0))
    small_r.paste(small, (0, 0), squircle_mask(small.width))

    save_png(rounded, ROOT / "icons/logo-master.png", 1024)
    save_png(square, ROOT / "icons/logo-square.png", 1024)

    # ── Tauri 桌面 ──
    save_png(rounded, APP_ICONS / "icon.png", 512)
    save_png(rounded, APP_ICONS / "256x256.png", 256)
    save_png(rounded, APP_ICONS / "128x128.png", 128)
    save_png(rounded, APP_ICONS / "128x128@2x.png", 256)
    save_png(rounded, APP_ICONS / "64x64.png", 64)
    save_png(small_r, APP_ICONS / "32x32.png", 32)

    # ── Windows 方标(全出血方稿) ──
    squares = {"Square30x30Logo.png": 30, "Square44x44Logo.png": 44,
               "Square71x71Logo.png": 71, "Square89x89Logo.png": 89,
               "Square107x107Logo.png": 107, "Square142x142Logo.png": 142,
               "Square150x150Logo.png": 150, "Square284x284Logo.png": 284,
               "Square310x310Logo.png": 310, "StoreLogo.png": 50}
    for name, s in squares.items():
        save_png(small if s < 44 else square, APP_ICONS / name, s)

    # ── macOS icns ──
    iconset = APP_ICONS / "pensoul.iconset"
    iconset.mkdir(exist_ok=True)
    icns_map = {"icon_16x16.png": 16, "icon_16x16@2x.png": 32,
                "icon_32x32.png": 32, "icon_32x32@2x.png": 64,
                "icon_64x64.png": 64, "icon_64x64@2x.png": 128,
                "icon_128x128.png": 128, "icon_128x128@2x.png": 256,
                "icon_256x256.png": 256, "icon_256x256@2x.png": 512,
                "icon_512x512.png": 512, "icon_512x512@2x.png": 1024}
    for name, s in icns_map.items():
        save_png(small_r if s < 48 else rounded, iconset / name, s)
    subprocess.run(["iconutil", "-c", "icns", str(iconset),
                    "-o", str(APP_ICONS / "icon.icns")], check=True)
    shutil.rmtree(iconset)

    # ── Windows ico(最大帧在首位) ──
    ico_base = downscale(rounded, 256)
    ico_others = [downscale(rounded, s) for s in (128, 64, 48, 24)] + \
                 [downscale(small_r, s) for s in (32, 16)]
    ico_base.save(APP_ICONS / "icon.ico", format="ICO",
                  sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
                  append_images=ico_others)

    # ── favicon ──
    pub = ROOT / "public"
    pub.mkdir(exist_ok=True)
    fav_base = downscale(rounded, 48)
    fav_base.save(pub / "favicon.ico", format="ICO",
                  sizes=[(16, 16), (32, 32), (48, 48)],
                  append_images=[downscale(small_r, s) for s in (32, 16)])

    # ── Android ──
    densities = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
    for dens, s in densities.items():
        d = APP_ICONS / f"android/mipmap-{dens}"
        save_png(square, d / "ic_launcher.png", s)
        circ = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        cm = Image.new("L", (s, s), 0)
        ImageDraw.Draw(cm).ellipse([0, 0, s, s], fill=255)
        circ.paste(downscale(square, s), (0, 0), cm)
        circ.save(d / "ic_launcher_round.png", "PNG")
        fg_size = s * 3
        fg = Image.new("RGBA", (fg_size, fg_size), (0, 0, 0, 0))
        inner = downscale(square, int(fg_size * 0.78))
        fg.alpha_composite(inner, ((fg_size - inner.width) // 2, (fg_size - inner.height) // 2))
        fg.save(d / "ic_launcher_foreground.png", "PNG")

    # ── iOS(不透明全出血) ──
    ios_dir = APP_ICONS / "ios"
    for f in ios_dir.glob("AppIcon-*.png"):
        stem = f.stem.replace("AppIcon-", "")
        base = stem.split("@")[0]
        scale = stem.split("@")[1].replace("x", "") if "@" in stem else "1"
        scale = int(scale.replace("-1", "")) if "-" in scale else int(scale)
        pts = float(base.split("x")[0]) if "x" in base else float(base)
        px = int(round(pts * scale))
        save_png(square, f, px)

    # ── UI 内引用的 Logo ──
    save_png(rounded, ROOT / "src/assets/logo.png", 512)
    print("全套图标已按水墨正稿重出")


if __name__ == "__main__":
    main()
