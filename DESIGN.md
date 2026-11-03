---
name: TermZip website
description: A station departure board for a terminal: matte black, split-flap letters, amber for what is active, one blue enamel sign for the install action.
colors:
  board: "#0b0d10"
  panel: "#12161b"
  panel-raised: "#191e25"
  rule: "#29303a"
  flap-white: "#ece9df"
  text-dim: "#aeb2ba"
  text-faint: "#8d929b"
  amber: "#f4b63c"
  teal-ok: "#5edac8"
  red-fail: "#ff8479"
  enamel-blue: "#2a62f0"
  hinge: "#05070a"
typography:
  display:
    fontFamily: "Barlow Condensed, Arial Narrow, Helvetica Neue, Arial, PingFang SC, Noto Sans SC, Microsoft YaHei, sans-serif"
    fontSize: "4.4rem"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "0.005em"
  body:
    fontFamily: "Barlow, Helvetica Neue, Arial, PingFang SC, Noto Sans SC, Microsoft YaHei, system-ui, sans-serif"
    fontSize: "1.0625rem"
    fontWeight: 400
    lineHeight: 1.6
  mono:
    fontFamily: "JetBrains Mono, ui-monospace, SF Mono, Menlo, Consolas, monospace"
    fontSize: "0.95rem"
    fontWeight: 500
rounded:
  sm: "4px"
  md: "6px"
  lg: "12px"
  sign: "16px"
spacing:
  strip-gap: "4px"
  section: "5rem"
components:
  nameplate:
    backgroundColor: "{colors.flap-white}"
    textColor: "{colors.board}"
    rounded: "{rounded.sm}"
    padding: "0.3rem 0.9rem"
  departure-row:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.flap-white}"
    rounded: "{rounded.md}"
    padding: "0.85rem 0.9rem"
  install-sign:
    backgroundColor: "{colors.enamel-blue}"
    textColor: "#ffffff"
    rounded: "{rounded.sign}"
  copy-button:
    backgroundColor: "{colors.amber}"
    textColor: "{colors.board}"
    rounded: "{rounded.lg}"
  cta-band:
    backgroundColor: "{colors.amber}"
    textColor: "{colors.board}"
---

# Design system: TermZip website

## Overview

The page is a board of departures. Every command is a train that left on time or failed, the name is spelled in
split-flap tiles, and the real application is "the platform". It is the world of the visitor's scene (a person at a screen,
between an editor and a browser) translated into a station sign: matte black, a few loud signs, strict rows. The mode is
Persuade; the action is copying one install command. Dark is chosen by the physical scene, a board read in low light, not
by category.

## Colors

- **Board** (`#0b0d10`) is the ground of the whole page; **panel** and **panel-raised** are the strips and cards on it.
- **Flap-white** is the text and letter color; **text-dim** and **text-faint** carry secondary and caption text (all pass
  4.5:1 on the board).
- **Amber** means active or attention: nameplate outlines, feature names, the copy button, the closing band. It is the only
  warm color and is never used as decoration.
- **Teal-ok** and **red-fail** are status only (a command that worked or failed). Teal is taken from the app's mark.
- **Enamel-blue** is for the install sign alone: one saturated block, white text on it (4.9:1).

## Typography

Barlow Condensed (600, 700), uppercase and tracked, is the station-sign voice for headings, nameplates and labels. Barlow
(400, 600) sets the text. JetBrains Mono (500, 700) is for commands, statuses and measured numbers only. Chinese text
falls back to PingFang SC, Noto Sans SC or Microsoft YaHei. Fonts are served from the site. Display tops out at 4.4rem on
the headline; the 9rem tiles are a graphic, not text.

## Layout

A 1180px wrap with strips (rows 4px apart, like a timetable) as the repeating unit. The first viewport is the board: the
nameplate rule, TERMZIP in tiles across the full width, then the headline and rows on the left and the blue install sign on
the right (stacked on phones). Sections are separated by hairlines and generous space, each opened by a flap-white
nameplate. Features are a timetable: name, text, docs link.

## Elevation & Depth

Depth is physical and quiet: tiles have a hinge line and a soft drop, screenshots a deep, wide, low-opacity shadow, the
install sign a blue glow. No hard offset shadows, no glass.

## Shapes

Rectangles with small radii (4 to 6px) for strips and tiles, 12px for images and buttons, 16px for the sign. The sign has
an inner white ring, like enamel.

## Components

- **Flap tiles**: two halves with a hinge; on load each tile flips through a few letters and lands on its own. Without
  JavaScript, or with reduced motion, the letters are simply there.
- **Departure rows**: command (mono), status chip, time; one row is one real command block.
- **Install sign**: tabs per system, one copy button per command, the unsigned-build note always visible.
- **Timetable, facts and measures**: strips with a dim label and a lit value.
- **FAQ**: native `details`, the plus sign in amber.

## Do's and Don'ts

- Do show the real application and say what is not done yet (unsigned builds).
- Do keep one motion moment: the flap flip. Everything else is a short color or lift on hover.
- Don't add testimonials, counts or comparisons: there are none.
- Don't use amber for decoration or blue outside the install sign.
- Don't put a kicker above a heading: the nameplate is the heading.
