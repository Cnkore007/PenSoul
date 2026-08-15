---
name: PenSoul
description: 书斋雅韵——古典书房的温润质感，朱砂点睛
colors:
  paper: "oklch(96.8% 0.005 78)"
  paper-warm: "oklch(95% 0.008 74)"
  paper-deep: "oklch(92% 0.012 70)"
  paper-cream: "oklch(97.8% 0.006 82)"
  ink: "oklch(16% 0.010 58)"
  ink-2: "oklch(33% 0.010 62)"
  ink-3: "oklch(52% 0.008 68)"
  ink-faint: "oklch(70% 0.006 72)"
  rule: "oklch(80% 0.010 74)"
  rule-light: "oklch(87% 0.008 76)"
  accent: "oklch(44% 0.140 24)"
  accent-soft: "oklch(50% 0.115 24)"
  accent-wash: "oklch(94% 0.035 24)"
  accent-text: "oklch(98% 0 0)"
  indigo: "oklch(34% 0.075 270)"
  indigo-wash: "oklch(93% 0.030 270)"
  ochre: "oklch(48% 0.115 68)"
  ochre-wash: "oklch(94% 0.040 68)"
  jade: "oklch(44% 0.105 153)"
  jade-wash: "oklch(94% 0.035 153)"
  dark: "oklch(20% 0.012 50)"
  dark-mid: "oklch(28% 0.012 52)"
  dark-text: "oklch(70% 0.010 58)"
  dark-active: "oklch(94% 0.015 72)"
  success: "oklch(44% 0.095 153)"
  warning: "oklch(53% 0.105 76)"
  error: "oklch(44% 0.125 22)"
  surface-raised: "oklch(98% 0.004 80 / 0.85)"
  surface-overlay: "oklch(99% 0.002 82 / 0.95)"
  surface-sunken: "oklch(94% 0.008 74)"
  surface-tooltip: "oklch(22% 0.014 50)"
typography:
  display:
    fontFamily: '"LXGW WenKai", "STKaiti", "KaiTi", serif'
    fontSize: "1.75rem"
    fontWeight: 400
    letterSpacing: "4px"
    lineHeight: 1.3
  headline:
    fontFamily: '"LXGW WenKai", "STKaiti", "KaiTi", serif'
    fontSize: "1.375rem"
    fontWeight: 400
    letterSpacing: "2px"
    lineHeight: 1.35
  body:
    fontFamily: '"Noto Serif SC", "Source Han Serif SC", "STSong", "Songti SC", "Georgia", serif'
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.75
  label:
    fontFamily: '"LXGW WenKai", "Noto Serif SC", -apple-system, sans-serif'
    fontSize: "0.625rem"
    fontWeight: 400
    letterSpacing: "0.5px"
    lineHeight: 1.4
  mono:
    fontFamily: '"JetBrains Mono", "Fira Code", "SF Mono", monospace'
    fontSize: "0.8125rem"
rounded:
  xs: "2px"
  sm: "4px"
  md: "6px"
  lg: "10px"
spacing:
  3xs: "0.125rem"
  2xs: "0.25rem"
  xs: "0.5rem"
  sm: "0.75rem"
  md: "1rem"
  lg: "1.5rem"
  xl: "2rem"
  2xl: "3rem"
  3xl: "4rem"
  4xl: "6rem"
components:
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.sm}"
    padding: "7px 16px"
    typography: "{typography.label}"
  button-primary-hover:
    backgroundColor: "{colors.dark-mid}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.sm}"
    padding: "7px 16px"
    typography: "{typography.label}"
  button-accent:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-text}"
    rounded: "{rounded.sm}"
    padding: "7px 16px"
  card:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "{spacing.lg}"
  tag:
    backgroundColor: "{colors.accent-wash}"
    textColor: "{colors.accent}"
    rounded: "999px"
    padding: "2px 10px"
---

# Design System: PenSoul

## Overview

**Creative North Star: "书斋雅韵 — The Scholar's Studio"**

PenSoul 的视觉语言取自中国古典书房：铺开的宣纸是底色，松烟墨是文字，朱砂是唯一强调色。整个系统追求"安静的力量"——不喧哗、不浮夸，但每一处细节都经过精心雕琢。色调以暖白纸色为基，深墨为骨，五彩（花青、赭石、翡翠）为辅，朱砂只在最关键的操作点上出现，如同印章盖在画作上——点到为止。

密度上偏宽松：留白是尊重创作空间的方式。动效克制而有弹性，像毛笔提按——落笔有重量，收笔有回弹。组件哲学是"印章触感"：按钮像印章按下（有物理反馈），卡片像信笺展开（有纸张层次），输入像宣纸上落笔（笔尖有焦点光晕）。

**Key Characteristics:**
- 暖白宣纸底色 + 深墨文字 + 朱砂点睛的三色主调
- 五色辅色（花青/赭石/翡翠/焦茶/纸色层次）各自承载语义
- 书斋毛笔字体（LXGW WenKai）贯穿标题与 UI，宋体（Noto Serif SC）承载正文
- 圆角极度克制（2-10px），不追求现代圆润感
- 阴影如水墨晕染——低透明度、暖色相、层层递进
- 书脊侧边栏（深色焦茶 + 竹纹纹理）是空间锚点

## Colors

The palette is derived from the Chinese scholar's desk: aged paper, pine-soot ink, cinnabar seal paste, and the five traditional pigment families. Colors are defined in OKLCH for perceptual uniformity.

### Primary

- **Paper** (`oklch(96.8% 0.005 78)`): The base canvas — aged Xuan paper, warm and lived-in. Used for all view backgrounds, card surfaces, and the editor canvas.
- **Paper Warm** (`oklch(95% 0.008 74)`): Slightly warmer variant for elevated surfaces — status bars, warm panels, secondary containers.
- **Paper Deep** (`oklch(92% 0.012 70)`): The deepest paper tone — used for sunken areas and subtle depth hierarchy.
- **Paper Cream** (`oklch(97.8% 0.006 82)`): The lightest paper — used sparingly for the brightest highlights.

### Secondary

- **Cinnabar Accent** (`oklch(44% 0.140 24)`): The soul of the palette. Deep cinnabar red, used exclusively on primary action buttons, active states, focus rings, and the view-header diamond marker. Its rarity is the point.
- **Cinnabar Soft** (`oklch(50% 0.115 24)`): Lighter cinnabar for hover states and secondary accent usage.
- **Cinnabar Wash** (`oklch(94% 0.035 24)`): Near-transparent cinnabar tint for tag backgrounds, active tab washes, and hover backgrounds.

### Tertiary

- **Indigo** (`oklch(34% 0.075 270)`): Scholarly deep blue — used for info tags, draft status dots, and the sidebar active state. Carries intellectual depth.
- **Ochre** (`oklch(48% 0.115 68)`): Earthy warm amber — used for warning states, reviewing status, and the ochre tag family. Evokes aged silk.
- **Jade** (`oklch(44% 0.105 153)`): Living green — used for success states, reviewed/polished status dots, and the jade tag family. Represents completion and vitality.

### Neutral

- **Ink** (`oklch(16% 0.010 58)`): Pine-soot ink, rich and deep. Primary text color and the default button background.
- **Ink 2** (`oklch(33% 0.010 62)`): Secondary text — descriptions, secondary labels, muted body text.
- **Ink 3** (`oklch(52% 0.008 68)`): Tertiary text — field labels, metadata, timestamps.
- **Ink Faint** (`oklch(70% 0.006 72)`): The faintest ink — placeholder text, disabled states, subtle dividers.
- **Rule** (`oklch(80% 0.010 74)`): Ink-wash rules — borders, dividers, card edges. Organic and soft.
- **Rule Light** (`oklch(87% 0.008 76)`): The faintest rule — subtle separators that almost disappear.

### Dark Surfaces

- **Dark** (`oklch(20% 0.012 50)`): Warm charcoal — the sidebar background, dark panels.
- **Dark Mid** (`oklch(28% 0.012 52)`): Mid-tone dark — sidebar hover states, raised dark surfaces.
- **Dark Text** (`oklch(70% 0.010 58)`): Text on dark backgrounds — sidebar labels, dark panel text.
- **Dark Active** (`oklch(94% 0.015 72)`): Active text on dark — brand name, active sidebar item, highlighted dark text.

### Named Rules

**The Cinnabar Rule.** The accent color appears on ≤5% of any given screen. It is reserved for the single most important action or state indicator per view. Overuse dilutes its meaning.

**The Ink-Wash Rule.** Borders and shadows use warm-tinted neutrals (hue 50-75), never pure gray or blue-gray. This maintains the organic, hand-painted quality throughout.

## Typography

**Display Font:** LXGW WenKai (霞鹜文楷) with STKaiti / KaiTi fallback
**Body Font:** Noto Serif SC (思源宋体) with Source Han Serif SC / STSong fallback
**UI Font:** LXGW WenKai (same as display, unifying the entire interface)
**Mono Font:** JetBrains Mono with Fira Code / SF Mono fallback

**Character:** The LXGW WenKai typeface gives PenSoul its distinctive calligraphic personality — it reads as handwritten brush strokes without being illegible, carrying the scholar's study aesthetic into every UI element. Noto Serif SC provides the formal, book-like gravity for body text and long-form content. The pairing is warm and literary, never cold or corporate.

### Hierarchy

- **Display** (400 weight, `--text-xl` 1.375rem, letter-spacing 4px): View headers — the page title in each major section. Appears with a rotated cinnabar diamond marker.
- **Headline** (400 weight, `--text-md` 0.9375rem, letter-spacing 1.5px): Card headers, section subtitles within views.
- **Title** (500 weight, `--text-body` 1rem): Item titles in lists, chapter names, character names.
- **Body** (400 weight, `--text-body` 1rem, line-height 1.75): Editor content, discussion text, descriptions. Max comfortable width ~70ch.
- **Label** (400 weight, `--text-2xs` 0.625rem, letter-spacing 0.5px): Field labels, metadata chips, timestamps, status bar text.

### Named Rules

**The Brush Voice Rule.** All headings and navigation text use LXGW WenKai. Body text in the editor uses Noto Serif SC. Never swap these roles — the brush font is for interface navigation, the serif font is for content consumption.

## Layout

The layout is a fixed sidebar + scrollable content model. The sidebar (`--sidebar-width: 224px`, collapsed to `64px`) is the spatial anchor — always visible, always dark. Content views max out at `1200px` width with generous horizontal padding (`--space-xl: 2rem`). The status bar at bottom is `28px` tall.

**Density:** Generous. Vertical rhythm follows an 8px base grid (spacing tokens from `3xs: 2px` to `4xl: 6rem`). Card internal padding is consistently `1.5rem`. View headers have `1.5rem` bottom margin with a gradient ink-wash rule underneath.

**Responsive:** The sidebar collapses to icon-only at narrow widths. Content views are not grid-based — they use single-column flow with max-width constraints. No multi-column layouts exist in the current implementation.

## Elevation & Depth

PenSoul uses a **layered tonal** approach — depth is conveyed through paper color hierarchy (paper → paper-warm → paper-deep) rather than dramatic shadows. Shadows exist but are extremely subtle: warm-tinted (hue ~55, matching the paper warmth), low-opacity (3-14%), and short-range. They evoke ink wash bleeding into paper, not material elevation.

### Shadow Vocabulary

- **Subtle** (`0 1px 2px oklch(20% 0.010 55 / 3%)`): Default card shadow — barely perceptible at rest.
- **Small** (`0 2px 6px oklch(20% 0.010 55 / 5%)`): Card hover, button hover — gentle lift.
- **Medium** (`0 4px 16px oklch(20% 0.010 55 / 7%)`): Popover, dropdown — moderate presence.
- **Large** (`0 8px 32px oklch(20% 0.010 55 / 10%)`): Modal, dialog — significant but still warm.
- **Inner** (`inset 0 1px 3px oklch(20% 0.010 55 / 3%)`): Input focus, sunken areas.

### Named Rules

**The Flat-By-Default Rule.** Surfaces are flat at rest. Shadows appear only as a response to state (hover, elevation, focus). The sidebar is the one exception — it carries a fixed subtle shadow as the spatial anchor.

## Shapes

Corner radius is extremely restrained — the system avoids the modern trend of aggressively rounded shapes. The largest radius in the system is `10px` (used on modals/dialogs). Most elements use `4-6px`. Tags and status dots use `999px` (pill) and `50%` (circle) respectively. This restraint reinforces the scholarly, deliberate character — nothing feels bubbly or casual.

**Border treatment:** Borders use `1px solid` with ink-wash colors (rule/rule-light). The view header underline is a gradient that fades from ink-faint to rule to transparent — an ink-wash wash effect.

## Components

### Buttons — "印章" (Seal) Style

- **Shape:** 4px radius (`--radius-sm`), inline-flex with 5px gap
- **Primary:** Ink background (`--color-ink`), paper text, subtle shadow. Hover: lifts 1px, shadow grows, background shifts to dark-mid. Active: scales to 0.98.
- **Secondary:** Transparent background, ink-2 text, 1px rule border. Hover: paper-warm background, ink text, rule border.
- **Accent:** Cinnabar background (`--color-accent`), white text, cinnabar-tinted shadow (20% opacity). Hover: cinnabar-soft background, deeper shadow, lifts 1px.
- **Small variant:** 4px/10px padding, 11px font, 12px icons.

### Cards — "信笺" (Letter) Style

- **Shape:** 6px radius (`--radius-md`)
- **Background:** Paper (default), paper-warm (warm variant)
- **Border:** 1px solid rule-light, transitions to rule on hover
- **Shadow:** Subtle at rest, small on hover
- **Internal Padding:** `1.5rem` (`--space-lg`)
- **Header:** Flex row with bottom border (rule-light), brush font title

### Tags / Chips

- **Shape:** Fully rounded pill (`999px`), 2px/10px padding
- **Variants:** accent (cinnabar-wash bg, cinnabar text), success (jade-wash), warning (ochre-wash), error (error-wash), info (indigo-wash)
- **Usage:** Status indicators, category labels, metadata badges

### Inputs — "宣纸落笔" (Brush on Paper)

- **Style:** 1px solid rule border, paper background, 4px radius
- **Focus:** Border shifts to cinnabar, 2px cinnabar-wash glow ring (`box-shadow: 0 0 0 2px var(--color-accent-wash)`)
- **Typography:** UI font (LXGW WenKai), 12px

### Navigation — "书脊" (Book Spine) Sidebar

- **Style:** Dark background (`--color-dark`) with bamboo-texture overlay (`--texture-bamboo`) and top-to-bottom gradient vignette
- **Brand:** Logo (34px, 8px radius, spring-bounce on hover) + brush font title (3px letter-spacing)
- **Nav Items:** Ink-faint text, cinnabar accent on active (left border + text color), hover brightens to dark-active
- **Collapsed:** 64px width, icon-only, labels hidden

### View Headers

- **Title:** Brush font, `--text-xl`, 4px letter-spacing, preceded by a rotated (−4°) 8px cinnabar diamond with subtle glow
- **Rule:** Full-width gradient underline (ink-faint → rule → transparent), fading like an ink wash

### Status Dots

- **Shape:** 6px circle
- **Colors:** Draft (indigo, 40% opacity), Reviewing (ochre), Reviewed (cinnabar, 60% opacity), Polished (jade), Published (indigo)

### Textures (Signature Element)

- **Paper texture:** SVG fractal noise at 2% opacity — subtle grain on the main canvas
- **Bamboo texture:** Repeating horizontal lines at 1% opacity — the sidebar's defining detail
- **Ink wash:** Radial gradient at 2.5% opacity — used on dark surfaces for depth

## Do's and Don'ts

### Do:
- **Do** use cinnabar (`--color-accent`) sparingly — one primary action per view, one active indicator per list.
- **Do** use the ink-wash gradient rule under view headers — it's a signature element that reinforces the scholarly aesthetic.
- **Do** use brush font (LXGW WenKai) for all headings, navigation, and UI labels.
- **Do** use serif font (Noto Serif SC) for body text in the editor and long-form content.
- **Do** maintain generous spacing — the 8px grid and generous padding reflect the respect for creative space.
- **Do** use warm-tinted shadows (hue ~55) — never blue-gray or pure black shadows.
- **Do** apply the paper texture on the main canvas — it's subtle but essential for the handcrafted feel.

### Don't:
- **Don't** use rounded corners larger than 10px — the system deliberately avoids bubbly shapes.
- **Don't** use pure black (`#000`) or pure white (`#fff`) — everything is warm-tinted.
- **Don't** use the accent color for large surface areas — it's a point color, not a fill color.
- **Don't** mix brush font and serif font in the same UI element — brush for interface, serif for content.
- **Don't** use heavy drop shadows — the shadow system maxes out at 14% opacity.
- **Don't** add animations longer than 350ms — transitions should feel like brush strokes, not movies.
