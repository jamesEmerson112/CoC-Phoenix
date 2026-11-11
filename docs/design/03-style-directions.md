# 03 · Style direction: 1-bit with blood

**Decision (2026-11-10):** the signup wizard uses the "1-bit with blood" look, sheet A × B in the
style studies: black ink on bone paper, a pixel font, and blood red as the only accent.
References and credits are in `02-style-references.md`.

The style studies are a private design canvas: [Investigator Sheet Style Studies](https://claude.ai/artifact/GsNjWE4GdiPHb7Rb1HDLLV).
Only its owner can open it until it is shared from its Share menu. It holds the step 6 sheet in
seven styles, and wizard steps 1–5 in this one, clickable in Play mode.

## How we got here

Each single style borrows techniques from one reference; the combinations mix them. Every sheet
shows the same invented investigator and the same numbers, so only the style changes.

| Sheet | Built from |
|---|---|
| A · 1-bit pixel | WORLD OF HORROR: 1-bit art, dithering, retro computer windows |
| B · Ink and blood | Tokyo Ghoul:re: white space, wet ink, one red |
| C · Megastructure | BLAME! Master Edition: monochrome architecture around a tiny figure |
| A × C, B × C, A × B × C | Combinations |
| **A × B · 1-bit with blood** | **Chosen** |

Why this one: it keeps World of Horror's pixel dread, which is the reference the choice started
from, and adds a single red that marks what matters (the current step, a selection, Sanity).
It is also art we can make ourselves, from code and from public-domain prints, without borrowing
anyone's pictures.

## Palette

Mapped onto the prototype's existing variables (`:root` in `prototype/web/index.template.html`),
so switching is a value change rather than a restructure.

| Variable | Today | 1-bit with blood | Used for |
|---|---|---|---|
| `--bg` | `#0f141b` | `#e8e2d4` bone | Page |
| `--panel` | `#171e28` | `#e8e2d4` bone | Windows, told apart by border and shadow rather than fill |
| `--line` | `#2c3746` | `#0b0b0c` ink | Borders, rules, title-bar stripes |
| `--fg` | `#e7ecf1` | `#0b0b0c` ink | Text |
| `--muted`, `--faint` | `#95a2b3`, `#5f6c7d` | `#0b0b0c` ink | 1-bit has no greys: secondary text is smaller, not lighter |
| `--accent` | `#5cc0c9` | `#a3161e` blood | Current step, selection, Sanity, omens |
| `--accent-ink` | `#062126` | `#e8e2d4` bone | Text on blood |
| `--amber`, `--amber-ink` | `#e7ad4f`, `#2a1a02` | `#0b0b0c`, `#e8e2d4` | Primary buttons and dice: ink with bone text |
| `--rose` | `#e08cb4` | `#a3161e` blood | The seal |
| `--danger` | `#f08a7a` | `#a3161e` blood | Errors |
| `--bar-base` | `#4b596b` | The dither pattern below | Empty part of meters, steps not reached yet |

Contrast: ink on bone is 15.2:1 and blood on bone 6.0:1, both fine for body text; bone on blood
is 6.0:1. Never ink on blood (2.5:1) or blood on ink: blood only ever sits on bone.

## Type

One family for everything: **DotGothic16** (Google Fonts, open licence). It covers Latin and
Japanese and keeps digits readable. Headings 26 px, body 13–16 px, labels 11–12 px, the legal
footer 10 px at the smallest. Labels and buttons are uppercase; prose stays in sentence case.

## Components

- **Window:** 1 px ink border, a 3 px offset ink shadow, and a 20 px title bar of 1 px ink stripes
  every 3 px with a centred label (`STATS`, `OMEN.PIC`) and a small close box on the left.
- **Selected** (age band, career, hobby): 2 px blood border with a 3 px blood offset shadow.
  Unselected: 1 px ink border.
- **Buttons:** primary is an ink fill with bone uppercase text and an offset shadow; secondary is a
  1 px ink outline. At least 44 px tall.
- **Dither:** a 2 px checker, `repeating-conic-gradient(#0b0b0c 0% 25%, #e8e2d4 0% 50%) 0 0 / 2px 2px`,
  for the empty part of a meter and for steps not reached yet. The Sanity meter uses the same
  pattern in blood.
- **Progress:** six segments: done in ink, current in blood, the rest dithered.
- **Seal:** a blood square with 鳳 (phoenix) in bone, on "Sign and seal".
- **Log lines:** bracketed time stamps (`[06:06] The file is complete.`); the ominous line is in
  blood.
- **Japanese labels** beside headings are optional: 能力値 (characteristics), 正気度 (sanity),
  技能 (skills), 探索者 (investigator).

## Art

Every image is 1-bit (bone and ink) or three-colour (bone, ink, blood), drawn at a low resolution
and shown at whole-number scales with `image-rendering: pixelated`.

1. **Public-domain prints:** greyscale, shrink to about 300 px wide, Atkinson dither, apply the
   palette, scale up 3× with nearest-neighbour. Hokusai's Kohada Koheiji is the first one done.
2. **Generated scenes,** seeded so the same seed always draws the same picture: the investigator
   is a faceless figure in a long coat and a wide-brimmed hat, drawn large in greys, shrunk and
   dithered. Blood is drawn on its own layer and dithered separately as the third colour.
3. **Never** trace, redraw or dither anything from the reference works: no screenshots, covers,
   characters or logos.

The generator (dithering, palettes, splatter, the silhouette) is the `Art` artboard on the canvas;
`docs/design/references/_scripts/dither.py` is a local offline version.

## Copy and tone

Short, dry, a little ominous. Flavour lives in the log lines ("A drop lands on the page. It is not
ink."); rules text stays plain.

## Open questions

- **A dark variant:** sheet A (bone on ink) as a night mode? Blood can't be text on ink, so it
  would need a lighter red.
- **Japanese labels:** keep them as decoration, make the UI bilingual, or drop them?
- **Art per step:** which public-domain prints, and whether each step gets its own omen picture.
- **Small sizes:** check DotGothic16 at 10–11 px on low-density Android screens.
- **The prototype** still uses its original slate-and-teal tokens; switching it is a code change
  that hasn't been made.
