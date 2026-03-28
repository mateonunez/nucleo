# Nucleo — Brand Identity

> The reusable Rust CLI framework. The nucleus of your next CLI.

---

## What Is Nucleo

**nucleo** is a production-ready Rust CLI framework that gives every CLI the same hard foundations out of the box: token-based authentication, multi-environment config, multi-format output (JSON, YAML, CSV, table, Slack), a language-agnostic plugin system, shell completion, project scaffolding, and a Claude Desktop MCP server — all wired up and ready to fork.

The name comes from *nucleus*: the dense, essential core that everything else orbits around. You take nucleo, rename the constants, add your commands, and ship. No boilerplate. No wheel reinvention.

**Three words:** *essential · composable · precise*

---

## Brand Concept

The nucleus metaphor is the single organizing idea behind the entire identity. A nucleus is small, dense, and fundamental — it doesn't try to be everything, it enables everything. The visual language follows: geometric, minimal, technical. Warm nuclear orange marks what matters most. Orbital violet and electric blue suggest the layers that extend outward from the core.

The aesthetic is Vercel-inspired: pure black backgrounds, tight Geist typography, zero decorative noise. Terminal output is a first-class design element, not an afterthought.

---

## Color Palette

| Token | Hex | Role |
|-------|-----|------|
| `--bg` | `#050505` | Page / slide backgrounds |
| `--surface` | `#0F0F0F` | Cards, terminals, code blocks |
| `--border` | `#1C1C1C` | Subtle separators, card outlines |
| `--text-primary` | `#F5F5F5` | Headlines, body copy |
| `--text-secondary` | `#6B6B6B` | Captions, metadata, labels |
| `--text-muted` | `#444444` | Slide numbers, fine print |
| `--accent-core` | `#FF6B2B` | Logo mark, key numbers, CLI commands |
| `--accent-orbital` | `#7C3AED` | Secondary accent, gradient terminus |
| `--accent-shell` | `#1D4ED8` | Tertiary, links, syntax highlighting |
| `--success` | `#22C55E` | Checkmarks, passing states |
| `--error` | `#EF4444` | Errors, exit codes |

**Signature gradient** (logo mark and key accent moments only):
```
linear-gradient(135deg, #FF6B2B 0%, #7C3AED 100%)
```

**Rule:** color is layered on top of a monochrome foundation — the brand must work in pure black/white first. Orange is reserved for what matters most; never use it for decoration.

---

## Typography

| Role | Font | Weight | Size | Tracking |
|------|------|--------|------|----------|
| Display / hero | Geist | 800–900 | 64–96px | `-0.04em` |
| Section titles | Geist | 600–700 | 32–48px | `-0.03em` |
| Body | Geist | 400 | 14–16px | `0` |
| Labels / badges | Geist | 500 | 11–12px | `+0.08em`, uppercase |
| Code / terminal | Geist Mono | 400–500 | 13–14px | `0` |
| Captions | Geist | 400 | 11–12px | `0`, `#6B6B6B` |

Fallback stack: `'Geist', 'Inter', system-ui, -apple-system, sans-serif`
Mono fallback: `'Geist Mono', 'JetBrains Mono', 'Fira Code', monospace`

---

## Shape Language

- **Borders:** `1px solid #1C1C1C` on all cards and panels — no fill contrast, separation through line only.
- **Radius:** `6px` cards · `4px` code blocks · `3px` badges · `2px` inline chips.
- **Shadows:** none — elevation is communicated through border and background offset, not drop shadows.
- **Backgrounds:** flat; no gradients on surfaces. The signature gradient appears only on the logo mark and critical accent moments (a key metric, a CTA button).
- **Spacing:** 8px grid. Section padding: `48px`. Card padding: `24px`.

---

## Logo Mark

The mark is an **atomic nucleus symbol** abstracted to pure geometry:

- One small filled circle at center — the nucleus — in `#FF6B2B`.
- Two or three hairline elliptical orbital rings at different angles and rotations, evoking an atomic model without being a chemistry illustration.
- Orbital ring colors: `#7C3AED` and `#1D4ED8`, flat strokes, no fill.
- No enclosing bounding shape (no square, no circle container).
- Must read clearly at `16×16px` and remain sharp at `512×512px`.

**Do not:**
- Add gradients to the orbital rings.
- Round or soften the geometry into an illustration.
- Enclose the mark in any container shape.
- Use the mark in colors other than the defined palette.

---

## Image & Asset Generation Prompts

### Primary Logo Mark

```
Minimal vector logo mark for a developer CLI framework called "nucleo".
A single atom-inspired geometric symbol: one small filled circle at center
(the nucleus) in warm orange (#FF6B2B), surrounded by two thin elliptical
orbital rings at different tilt angles — one in deep violet (#7C3AED),
one in electric blue (#1D4ED8). Hairline flat strokes, no fill on rings.
Pure black (#050505) background. No outer bounding shape. No wordmark.
Swiss grid precision. Figma SVG vector aesthetic.
Must read sharply at 16×16px and 512×512px.
```

### Wordmark Lockup (horizontal)

```
Horizontal brand lockup for "nucleo". Left: the atomic nucleus mark (orange dot,
violet and blue orbital rings). Right: lowercase wordmark "nucleo" in Geist
weight 600, color #F5F5F5, letter-spacing -0.03em.
Mark and text vertically centered with 12px gap.
Background #050505. No tagline. No stylization on the "o" — keep it standard.
Clean, developer-tool aesthetic.
```

### Dark Hero Banner (GitHub README / website header, 1280×640)

```
Dark developer-tool hero banner, 1280×640px, for a CLI framework called "nucleo".
Background #050505.
Left half: display text "nucleo" in Geist weight 800, ~96px, white (#F5F5F5),
tracking -0.04em. Below it in #6B6B6B at 18px: "the nucleus of your CLI".
Below that, three inline pills: "Rust" | "Plugin system" | "MCP-ready" —
each with #1C1C1C border, #0F0F0F fill, 12px Geist text in #888888.
Right half: a terminal window mockup, #0F0F0F background, 1px #1C1C1C border,
6px radius, showing:

  $ nucleo setup
  Welcome to nucleo setup!
  ✓  Authenticated as mateo
  ✓  Environment: production
  ✓  Claude Desktop MCP configured
  ✓  Ready.

Green checkmarks (#22C55E). Command text in orange (#FF6B2B). Normal output
in #F5F5F5. The atomic nucleus mark appears as a 4% opacity watermark behind
the full banner. No decorative gradients on the background.
Vercel-inspired aesthetic.
```

### Social Card (OpenGraph, 1200×630)

```
Dark OG social card for "nucleo" CLI framework. 1200×630px.
Background #050505 with a very subtle centered radial glow (radius 500px,
#FF6B2B at 3% opacity — barely perceptible).
Center-aligned vertical layout:
  1. Atomic nucleus mark, 80px tall
  2. "nucleo" wordmark in Geist weight 700, 72px, #F5F5F5
  3. "the nucleus of your CLI" — Geist 24px, #6B6B6B
  4. 1px #1C1C1C horizontal rule, 400px wide
  5. Three pills: "Rust" | "Plugin system" | "MCP-ready"
     — #1C1C1C border, #0F0F0F fill, 12px Geist, #888888 text
No photos. Pure type and geometry.
```

### App Icon / Favicon (512×512)

```
Square app icon for "nucleo", 512×512px.
Background #0F0F0F (dark grey — must read on both dark and light contexts).
Center: the atomic nucleus mark scaled to ~60% of canvas width.
One small filled orange dot (#FF6B2B) at center.
Two thin elliptical orbital rings at different tilts: violet (#7C3AED) and
blue (#1D4ED8). Flat strokes, no glow, no shadow.
No wordmark. Rounded corners at 18% radius (for macOS/iOS icon shape).
Pure geometry — not illustrated, not 3D, not glossy. SVG-style precision.
```

### Presentation Slide Template

```
Dark presentation slide template for "nucleo" CLI framework. 16:9 ratio.
Background #050505.
Top-left: atomic nucleus mark at 20px + "nucleo" in Geist 500 at 13px,
color #444444 — persistent brand watermark.
Bottom: full-width 1px #1C1C1C rule. Slide number right-aligned below
in #444444, Geist Mono 11px.
Typography: Geist throughout.
  - Headlines: #F5F5F5, weight 700, tracking -0.03em
  - Body: #A1A1A1, weight 400
  - Key numbers / accented terms: #FF6B2B
Code blocks: #0F0F0F bg, 1px #1C1C1C border, 4px radius, Geist Mono 13px.
Syntax colors: keywords in #FF6B2B, types in #7C3AED, strings in #22C55E,
comments in #444444.
No background gradients. No decorative shapes. Zero visual noise.
```

### Dark CLI Documentation Card (for docs / blog)

```
Dark feature card for CLI documentation, 800×400px.
Background #0F0F0F, 1px #1C1C1C border, 6px radius.
Top: category label in Geist 500 11px uppercase, tracking +0.08em, #7C3AED.
Title in Geist 700 28px #F5F5F5, max two lines.
Body copy in Geist 400 14px #A1A1A1, max three lines.
Bottom-left: a terminal snippet in Geist Mono 13px:
  the most representative command for the feature, with the "nucleo" keyword
  in #FF6B2B and arguments in #F5F5F5.
Bottom-right: the atomic nucleus mark at 32px, #1C1C1C opacity (watermark).
No photos. No gradients.
```

---

## Voice & Tone

- **Precise, not verbose.** Documentation reads like code — declarative and specific.
- **Confident, not loud.** The framework is opinionated; say so plainly.
- **Builder-to-builder.** The audience forks this; treat them as peers.

**Phrases that fit:**
- "the nucleus of your CLI"
- "fork it, rename it, ship it"
- "batteries included, opinions included"
- "built for builders who don't want to start from scratch"

**Phrases to avoid:**
- "powerful", "blazing fast", "seamless", "leverage", "game-changer"

---

## Brand Principles

1. **Precision over decoration** — every element earns its place. When in doubt, remove.
2. **Orange marks the core** — `#FF6B2B` is reserved for the mark, key metrics, and CLI commands. Never use it as a fill or background.
3. **Monochrome first** — the entire identity works in black and white. Color is an enhancement, not a load-bearing element.
4. **Terminal output is design** — CLI output, code blocks, and command lines are first-class typographic elements.
5. **Atomic without literalism** — the nucleus/orbital motif informs the mark's geometry; it never becomes a chemistry diagram or icon.
6. **Consistent scale** — use the logo mark at `16px`, `24px`, `32px`, `48px`, `80px`, `512px`. Never at arbitrary intermediate sizes.

---

## Asset Checklist

- [ ] Logo mark (SVG, transparent background)
- [ ] Wordmark lockup — horizontal (SVG, dark bg)
- [ ] Wordmark lockup — stacked (SVG, dark bg)
- [ ] App icon 512×512 (PNG + SVG)
- [ ] Favicon 32×32 / 16×16 (ICO / PNG)
- [ ] GitHub social card 1280×640 (PNG)
- [ ] OpenGraph card 1200×630 (PNG)
- [ ] Presentation slide template (Figma / PPTX)
- [ ] Documentation card template (Figma)
- [ ] Color tokens (CSS custom properties)
- [ ] Font specimen sheet
