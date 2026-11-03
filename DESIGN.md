---
name: TermZip website
description: A bright product page with one bold brand stage: warm paper, ink, a committed violet field, and the mark's teal for "it worked".
colors:
  paper: "#f6f4ef"
  card: "#ffffff"
  ink: "#16141f"
  ink-muted: "#4a4658"
  ink-faint: "#6b6778"
  line: "#e4e0d6"
  violet: "#5a47f5"
  violet-deep: "#3d2fd0"
  violet-tint: "#ebe8ff"
  teal: "#5edac8"
  teal-ink: "#0b7a6c"
  teal-tint: "#d9f5ef"
  warm-tint: "#fff1cf"
  fail: "#a8271d"
typography:
  display:
    fontFamily: "Bricolage Grotesque, Helvetica Neue, Arial, PingFang SC, Noto Sans SC, Microsoft YaHei, sans-serif"
    fontSize: "4.7rem"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.03em"
  body:
    fontFamily: "Hanken Grotesk, Helvetica Neue, Arial, PingFang SC, Noto Sans SC, Microsoft YaHei, system-ui, sans-serif"
    fontSize: "1.0625rem"
    fontWeight: 400
    lineHeight: 1.6
  mono:
    fontFamily: "JetBrains Mono, ui-monospace, SF Mono, Menlo, Consolas, monospace"
    fontSize: "0.85rem"
    fontWeight: 500
rounded:
  sm: "8px"
  md: "16px"
  lg: "28px"
  xl: "40px"
  pill: "999px"
spacing:
  gap: "16px"
  section: "6rem"
components:
  nav-pill:
    backgroundColor: "{colors.card}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
  install-card:
    backgroundColor: "{colors.card}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "1.1rem"
  stage:
    backgroundColor: "{colors.violet}"
    textColor: "#ffffff"
    rounded: "{rounded.xl}"
  copy-button:
    backgroundColor: "{colors.violet}"
    textColor: "#ffffff"
    rounded: "{rounded.md}"
  bento-tile:
    backgroundColor: "{colors.card}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "1.9rem"
  night-section:
    backgroundColor: "{colors.ink}"
    textColor: "#ffffff"
    rounded: "{rounded.xl}"
---

# Design system: TermZip website

## Overview

A confident bright product page. The real application sits on one saturated violet stage at the top, and every capability
below is a small drawn piece of the real interface (a command-block card, a tab strip, a permission list, a theme row, a set
of keycaps). It is the category standard played straight and held to the finish of the best product pages of its kind, with
one committed colour field and large radii instead of a theme. The mode is Persuade; the action is copying one install
command, which is in the first viewport. Light is chosen for daylight reading; the privacy section inverts to ink for rhythm.

## Colors

- **Paper** (`#f6f4ef`) is the ground; **card** white lifts content; **ink** is text and the one dark section.
- **Violet** (`#5a47f5`) is the brand field: the stage, the primary button, the closing card. It carries white text (5.8:1).
  **Violet-deep** is for links on light grounds.
- **Teal** (`#5edac8`) is from the app's mark and means success: statuses, checks, the prompt. On light grounds use
  **teal-ink** for text. Tints (violet, teal, warm) colour tiles.
- Red is used only for a failed command.

## Typography

Bricolage Grotesque 800 for display, tight (-0.03em) and large; Hanken Grotesk for text; JetBrains Mono for commands,
statuses and numbers only. Chinese falls back to PingFang SC, Noto Sans SC or Microsoft YaHei. Fonts are served from the
site. Headings are plain sentences, with no kickers or numbered sections.

## Layout

A 1180px wrap. A floating pill navigation with the language switcher. The first viewport is the headline and sentence on
the left and the install card on the right, with the violet stage immediately below. Features are a six-column bento whose
tiles span 3 or 2 columns, then a tabbed screenshot showcase, an ink privacy section, a performance table, an FAQ and a
closing card. Everything stacks to one column on phones.

## Elevation & Depth

Soft, low-opacity, offset shadows tinted from the brand (violet) under the install card and the screenshot; chips float over
the stage edge. No glass, no hard offset shadows.

## Shapes

Large radii: 28px tiles and cards, 40px stages and sections, 16px inner controls, pills for navigation and tabs. Two
geometric shapes (a teal disc and a translucent ring) sit on the stage and the closing card.

## Components

- **Install card**: segmented system tabs, a dark command line with a teal prompt, a violet copy button, the unsigned-build note.
- **Stage**: violet field with the real screenshot rising from its bottom edge, a command-block chip and a keycap chip.
- **Bento tiles**: text plus a drawn interface fragment, aria-hidden.
- **Showcase**: pill tabs over real screenshots.
- **FAQ**: native `details` with a round plus control.

## Do's and Don'ts

- Do show the real application and say what is not done yet (unsigned builds).
- Do keep one motion moment: the two chips rising onto the stage; everything else is a quick hover.
- Don't add testimonials, counts or comparisons: there are none.
- Don't use gradient text, kickers above headings, or identical icon cards.
- Don't use violet for decoration outside the stage, the primary actions and the closing card.
