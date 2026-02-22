# CollabBoard Portfolio Site — Design Specification

## "Field Survey Terminal" Aesthetic, Adapted for a Software Portfolio

---

## 0. Document Purpose

This spec defines **every visual and structural decision** for a portfolio website showcasing CollabBoard, a real-time collaborative whiteboard built entirely in Rust. The implementing developer should be able to build the site from this document alone, without needing the reference screenshots.

The visual language is lifted from a third-party webapp that presents paleontological field survey data in a retro-scientific terminal aesthetic. We are _not_ cloning that app — we are extracting its design system and re-skinning it for a software engineering portfolio context. The "field survey" metaphor becomes a **"build survey"** or **"engineering field report"** — as if documenting a 7-day construction sprint the way a 1970s geologist would document a dig site.

---

## 1. Design Philosophy

### Core Aesthetic

**Vintage institutional research terminal.** The feel is a declassified government research station UI from the 1970s–1980s: typewriter-produced field reports, analog instrument panels, cartographic survey overlays. Every element reads as if it was designed for a CRT monitor at a remote data collection outpost.

### Key Tensions That Define the Style

| Tension | How It Manifests |
|---------|-----------------|
| Analog warmth vs. digital precision | Parchment backgrounds + pixel-perfect grid alignment |
| Institutional formality vs. human touch | ALL_CAPS monospace labels + one elegant script/hand element per view |
| Dense data vs. breathing room | Packed stat strips + generous padding inside each card |
| Monochrome restraint vs. selective warmth | Near-grayscale palette with warm sepia undertones |
| Retro surface vs. modern interaction | Vintage appearance but smooth transitions and responsive layout |

### The Narrative Frame

The portfolio tells the story of a 7-day build sprint. Each "day" is treated like a survey site — with its own field notes, artifacts (screenshots/video), metrics, and observations. The overall site is the "survey report" compiling all sites into a cohesive narrative.

---

## 2. Color Palette

### Primary Palette (CSS Custom Properties)

```css
:root {
  /* Background layers (warm parchment gradient, lightest to deepest) */
  --bg-page:           #E8E0D0;   /* Overall page background — aged parchment */
  --bg-card:           #F2ECE0;   /* Card/panel surfaces — slightly lighter parchment */
  --bg-card-alt:       #EDE6D8;   /* Alternate card shade for subtle variation */
  --bg-card-inset:     #E5DDD0;   /* Inset/recessed areas within cards */
  --bg-nav:            #D8D0C0;   /* Navigation bar — slightly darker than page */
  --bg-status-bar:     #D0C8B8;   /* Footer status bar — darkest background */

  /* Text hierarchy */
  --text-primary:      #2C2416;   /* Darkest — headings, primary content */
  --text-secondary:    #5C5040;   /* Mid-tone — body text, descriptions */
  --text-tertiary:     #8C8070;   /* Lightest — metadata, timestamps, captions */
  --text-faint:        #B0A898;   /* Ghost text — watermarks, disabled states */

  /* Borders & Rules */
  --border-strong:     #6C6050;   /* Primary card borders, active tab underlines */
  --border-medium:     #A09080;   /* Table rules, dividers between sections */
  --border-light:      #C8C0B0;   /* Subtle separators, grid lines */
  --border-faint:      #D8D0C4;   /* Barely-visible structural lines */

  /* Accents (used very sparingly) */
  --accent-active:     #6B4C2A;   /* Active nav item underline, selected tab */
  --accent-green:      #4A7C5C;   /* Status "operational" dot */
  --accent-red:        #8C3C2C;   /* Error states, critical metrics */
  --accent-amber:      #8C6C2C;   /* Warning states */

  /* Map/illustration overlay */
  --map-land:          #D8D0C0;   /* Land masses on cartographic elements */
  --map-water:         #C8C0B0;   /* Water/ocean on cartographic elements */
  --map-border:        #A09888;   /* Country/region borders */
  --map-marker:        #2C2416;   /* Pin/marker dot — solid dark */
}
```

### Color Rules

1. **No saturated blues, purples, or bright accents anywhere.** Every color lives in the warm sepia/ochre family.
2. **Backgrounds use at most 3 distinct warmth levels** on any single view.
3. **The only "color" color** is the green status dot (`--accent-green`) and rare red/amber for error/warning states.
4. **Text never appears on a background with less than 4.5:1 contrast ratio** (WCAG AA).
5. **Borders are always 1px solid** — never 2px, never dashed, never rounded, never shadow.

---

## 3. Typography

### Font Stack

```css
:root {
  /* Primary — all labels, headings, metadata, body text */
  --font-mono:         'IBM Plex Mono', 'Courier Prime', 'Courier New', monospace;

  /* Display — the single human/handwritten element per view */
  --font-script:       'Playfair Display', 'Libre Baskerville', Georgia, serif;

  /* Fallback body — only if mono feels too dense in long-form paragraphs */
  --font-body:         'IBM Plex Mono', monospace;  /* still mono, just lighter weight */
}
```

**Why these fonts:**
- **IBM Plex Mono** — the workhorse. Available on Google Fonts. Has true weight range (300–700). Reads as "institutional terminal" without being as played-out as Courier. Excellent at small sizes.
- **Playfair Display** — for the one script/display element per view (project name "CollabBoard" in the hero, or section titles). Italic variant has the calligraphic flourish that mirrors the cursive species names in the reference.
- If neither loads, the fallbacks are intentionally "typewriter" — Courier Prime, then system Courier.

### Type Scale & Treatments

| Role | Font | Weight | Size | Tracking | Transform | Example |
|------|------|--------|------|----------|-----------|---------|
| **Nav item** | `--font-mono` | 400 | 12px | 0.08em | uppercase | `ARCHITECTURE` |
| **Nav item (active)** | `--font-mono` | 600 | 12px | 0.08em | uppercase + underline | `ARCHITECTURE` |
| **Section label** | `--font-mono` | 400 | 11px | 0.12em | uppercase, underscores | `BUILD_LOG_STATUS` |
| **Stat number** | `--font-mono` | 700 | 36px | -0.02em | none | `382` |
| **Stat caption** | `--font-mono` | 400 | 10px | 0.10em | uppercase | `COMMITS OVER 6 DAYS` |
| **Card title** | `--font-mono` | 600 | 14px | 0.06em | uppercase | `DINO-DRAFT SURVEY RECORD` → `BUILD_SPRINT FIELD REPORT` |
| **Display name** | `--font-script` | 400 italic | 48–64px | 0 | none | _CollabBoard_ |
| **Display subtitle** | `--font-mono` | 400 | 11px | 0.06em | none | `Project Field Lead` → `Ian · Rust · 7 Days` |
| **Tab label** | `--font-mono` | 400 | 13px | 0.08em | uppercase | `OVERVIEW  INVENTORY  METRICS` → `OVERVIEW  ARCHITECTURE  TIMELINE` |
| **Tab label (active)** | `--font-mono` | 600 | 13px | 0.08em | uppercase + bottom border | |
| **Key (in key-value)** | `--font-mono` | 400 | 12px | 0.06em | uppercase | `EPOCH_ID:` → `FRAMEWORK:` |
| **Value (in key-value)** | `--font-mono` | 600 | 12px | 0 | none | `Mesozoic` → `Leptos 0.8` |
| **Body text (long form)** | `--font-mono` | 300 | 13px | 0.01em | none | Paragraph descriptions |
| **Code/technical** | `--font-mono` | 400 | 12px | 0 | none | Inline code references |
| **Status bar** | `--font-mono` | 400 | 10px | 0.08em | uppercase | `ENGINE: GEMINI_FLASH_LITE` → `BUILT_WITH: RUST_2024` |

### Typography Rules

1. **Every label uses underscores instead of spaces** in uppercase contexts: `BUILD_LOG` not `BUILD LOG`.
2. **Only one `--font-script` element per viewport.** It's always the primary entity name (project name, day title in hero position).
3. **Tracking (letter-spacing) increases as size decreases.** Large display has tight or neutral tracking; tiny labels have wide tracking.
4. **Never use Title Case for metadata.** It's either `UPPERCASE_WITH_UNDERSCORES` or `lowercase normal prose`.
5. **Numbers in stats use tabular figures** (monospace digits) and are noticeably larger than their captions.
6. **Line height:** 1.2 for headings, 1.6 for body text, 1.0 for stat numbers.

---

## 4. Layout System

### Overall Page Structure

The reference app uses a **fixed full-viewport layout** with no scrolling on the outer frame. Content scrolls within panels. For a portfolio site, we adapt this to a **two-panel split** that scrolls the right panel while keeping the left panel (navigation/context) more stable.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│ NAV BAR (full width, fixed top)                                                 │
│ ☰ COLLABBOARD   [OVERVIEW]  ARCHITECTURE  TIMELINE  STACK  DEMO   ⊕ GITHUB     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  STAT STRIP (full width, 3–4 metric cards in a row)                             │
│ ┌────────────────┐ ┌────────────────┐ ┌────────────────┐ ┌────────────────┐     │
│ │ TOTAL_LINES    │ │ BUILD_DURATION │ │ TEST_COVERAGE  │ │ COMMIT_DENSITY │     │
│ │                │ │                │ │                │ │                │     │
│ │ 41,578         │ │ 7              │ │ 41.2%          │ │ ~64/DAY        │     │
│ │ SOURCE + TEST  │ │ DAYS           │ │ LINE COVERAGE  │ │ 382 TOTAL      │     │
│ └────────────────┘ └────────────────┘ └────────────────┘ └────────────────┘     │
│                                                                                 │
├──────────────────────────────────────────┬──────────────────────────────────────┤
│                                          │                                      │
│  LEFT PANEL (~60%)                       │  RIGHT PANEL (~40%)                  │
│  "Map/Visual" zone — primary content     │  "Record Card" — detail panel        │
│                                          │                                      │
│  On OVERVIEW: hero illustration/diagram  │  Project identity card               │
│  On ARCHITECTURE: crate dependency map   │  Architecture detail text            │
│  On TIMELINE: day-by-day visual strip    │  Day-specific field notes            │
│  On STACK: tech stack visualization      │  Dependency/crate breakdown          │
│  On DEMO: embedded video/screenshots     │  Feature descriptions                │
│                                          │                                      │
├──────────────────────────────────────────┴──────────────────────────────────────┤
│ STATUS BAR (full width, fixed bottom)                                           │
│ ● BUILT_WITH: RUST_2024   FRAMEWORK: LEPTOS_0.8   DEPLOY: RAILWAY     BUILD_ID │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Grid & Spacing

```css
:root {
  --grid-unit:         8px;       /* Base grid unit — all spacing is multiples of this */
  --gap-xs:            4px;       /* Half unit — tight internal padding */
  --gap-sm:            8px;       /* 1 unit */
  --gap-md:            16px;      /* 2 units */
  --gap-lg:            24px;      /* 3 units */
  --gap-xl:            32px;      /* 4 units */
  --gap-2xl:           48px;      /* 6 units */
  --gap-3xl:           64px;      /* 8 units */

  --card-padding:      24px;      /* Internal padding of all card panels */
  --card-border:       1px solid var(--border-strong);
  --section-gap:       32px;      /* Gap between major sections */
}
```

### Panel Proportions

| Zone | Width | Height | Notes |
|------|-------|--------|-------|
| Nav bar | 100% | 48px | Fixed top, 1px bottom border |
| Stat strip | 100% | ~120px | 3–4 equal columns, 1px borders between |
| Left panel | 58–62% | remaining viewport | Primary visual content area |
| Right panel | 38–42% | remaining viewport | Detail/record card, scrollable |
| Status bar | 100% | 32px | Fixed bottom, 1px top border |

### Responsive Behavior

| Breakpoint | Behavior |
|------------|----------|
| ≥ 1200px | Full two-panel layout as spec'd above |
| 900–1199px | Panels stack: stat strip → left panel → right panel (single column) |
| < 900px | Simplified: nav collapses to hamburger, stat strip becomes 2×2 grid, panels stack full-width |

---

## 5. Component Catalog

### 5.1 Navigation Bar

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ☰ COLLABBOARD   [OVERVIEW]  ARCHITECTURE  TIMELINE  STACK  DEMO  │ ⊕ GITHUB│
└─────────────────────────────────────────────────────────────────────────────┘
```

**Structure:**
- Full-width, `height: 48px`, `background: var(--bg-nav)`
- 1px bottom border in `var(--border-strong)`
- Left section: icon + site name + primary nav items
- Right section: secondary nav items (external links)
- Vertical divider (1px, `var(--border-medium)`) separates left and right sections

**Nav items:**
- `font: var(--font-mono)`, 12px, `letter-spacing: 0.08em`, uppercase
- Color: `var(--text-secondary)` default, `var(--text-primary)` on hover
- Active item: `font-weight: 600`, `color: var(--text-primary)`, 2px bottom border in `var(--accent-active)`, offset 2px below the text
- No background change on hover — only text color shift
- Items separated by 24–32px horizontal spacing

**Left icon:** A simple geometric glyph (⊞ grid icon or similar) in `var(--text-tertiary)`, 16px

### 5.2 Stat Strip Cards

```
┌─────────────────────┐
│ TOTAL_LINES_OF_CODE │  ← section label (11px, tracked uppercase, underscored)
│                     │
│ 41,578              │  ← stat number (36px, weight 700)
│ ACROSS 6 CRATES     │  ← stat caption (10px, tracked uppercase)
│ ━━━━━━━━━━━━━━━━━━  │  ← thin accent rule (optional, 2px, var(--accent-active))
└─────────────────────┘
```

**Structure:**
- Horizontal row of 3–4 cards, equal width
- 1px borders between cards (not around — they share the strip's outer border)
- Internal padding: `24px`
- Background: `var(--bg-card)`

**Content hierarchy:**
1. Section label — top, `var(--text-tertiary)`, small + tracked
2. Stat number — center-left aligned, `var(--text-primary)`, large + bold
3. Caption — below number, `var(--text-tertiary)`, small
4. Optional accent bar — bottom, thin horizontal rule in `var(--accent-active)`, only on highlighted stat

**Separator between label and content:** Optional thin line (`var(--border-light)`, 1px) below the section label

### 5.3 Record/Detail Card (Right Panel)

This is the "Dino-Draft Survey Record" adapted for project context.

```
┌─────────────────────────────────────────────┐
│  collabboard.dev        BUILD_SPRINT         │  ← header row: URL left, title center
│                    FIELD REPORT               │
│  ┌───────────────────────────────────────┐   │
│  │ PROJECT / IDENTIFICATION              │   │
│  │                                       │   │
│  │     CollabBoard                       │   │  ← script font, 48-64px italic
│  │                                       │   │
│  │                    Ian · Rust · 7 Days│   │  ← subtitle, right-aligned, small mono
│  └───────────────────────────────────────┘   │
│                                               │
│  [OVERVIEW]  ARCHITECTURE  METRICS            │  ← tab bar
│  ━━━━━━━━━━                                   │  ← active tab underline
│                                               │
│  ┌─ Content area (scrollable) ────────────┐  │
│  │                                         │  │
│  │  (varies by active tab)                 │  │
│  │                                         │  │
│  └─────────────────────────────────────────┘  │
│                                               │
└───────────────────────────────────────────────┘
```

**Header:**
- Background: `var(--bg-card)`, 1px border all around
- Top line: small numeric/text metadata flush left and right (like coordinates in the reference — here, use URL and build ID)
- Title: centered, `--font-mono`, 14px, weight 600, tracked uppercase

**Identity block:**
- Bordered inset box with slightly different background (`var(--bg-card-alt)`)
- Label top-left: `PROJECT / IDENTIFICATION` in tiny tracked mono
- Center: Project name in `--font-script`, italic, 48–64px, `var(--text-primary)`
- Bottom-right: subtitle metadata in small mono

**Tab bar:**
- 3–4 tabs, `--font-mono`, 13px, tracked uppercase
- Active tab: `font-weight: 600`, 2px bottom border in `var(--accent-active)`
- Inactive: `var(--text-tertiary)`, no border
- Spacing between tabs: 24–32px

### 5.4 Key-Value List (Metrics/Stats Display)

```
VITAL_STATISTICS                          ← section heading
━━━━━━━━━━━━━━━━

FRAMEWORK:  ·················  Leptos 0.8  ← key: value with dot-leader
LANGUAGE:   ·················  Rust 2024
SERVER:     ·················  Axum 0.8
DATABASE:   ·················  PostgreSQL
WIRE_FORMAT:·················  Protobuf
AUTH:       ·················  GitHub OAuth + Email Codes
AI_BACKEND: ·················  Anthropic / OpenAI
```

**Structure:**
- Section heading in `--font-mono`, 16px, weight 600, tracked uppercase, `var(--text-primary)`
- 2px rule below heading in `var(--border-strong)`
- Each row: key left-aligned, value right-aligned, dot-leader fill between
- Key: `--font-mono`, 12px, weight 400, tracked uppercase, `var(--text-secondary)`
- Value: `--font-mono`, 12px, weight 600, `var(--text-primary)`
- Row height: 32px (generous vertical rhythm)
- Dot leader: repeated `·` in `var(--text-faint)` or CSS `border-bottom: 1px dotted var(--border-light)`

### 5.5 Inventory/List Display

Used for: recovered specimens → crate descriptions, feature list, day-by-day entries.

```
RECOVERED_SPECIMENS / CRATE                         STRATUM
                                                    ← column header (right-aligned date/version)

CANVAS — THE ENGINE                                  v0.1
━━━━━━━━━━━━━━━━━━━
A from-scratch 2D whiteboard engine. Compiles to
native Rust for testing, compiles to WASM for the
browser. Zero browser dependencies in the core.

SERVER — THE BACKEND                                 v0.1
━━━━━━━━━━━━━━━━━━━━
Axum HTTP server, WebSocket hub, and persistence
layer. Handler functions return an Outcome enum
and a single dispatch layer decides routing.
```

**Structure:**
- Column header row: left label + right label, both small tracked mono, separated by line
- Each entry: bold title (`--font-mono`, 14px, weight 700), underline, italic description below in lighter weight
- Right-aligned metadata (stratum/date/version) in `var(--text-tertiary)`
- Entry spacing: `var(--gap-lg)` between entries

### 5.6 Map/Visual Panel (Left Side)

In the reference, this is a cartographic map with site markers. For the portfolio, this zone adapts per page:

| Page | Left Panel Content |
|------|-------------------|
| **Overview** | Full-page hero: large project title in script font over a subtle architectural diagram or crate-dependency graph rendered as a "survey map" |
| **Architecture** | Interactive or static crate dependency diagram styled as a geological survey map — crates as "sites", connections as survey lines |
| **Timeline** | Horizontal day-by-day strip with markers (Day 1 through Day 7), carousel-style with ◁ ▷ arrows |
| **Stack** | Technology visualization — Rust/Axum/Leptos/etc. as labeled points on a "terrain" |
| **Demo** | Embedded video (Loom) or screenshot carousel |

**Map panel styling:**
- Background: `var(--bg-page)` with very subtle cartographic texture (thin grid lines at 0.05 opacity, or dot grid)
- Overlay info box (top-left): bordered card with location/context metadata
- Navigation arrows: `◁` and `▷` in circular bordered buttons, positioned vertically-centered left and right edges
- Zoom controls: `+` / `−` stacked buttons, bottom-left
- Page indicator: `SITE 3 OF 7` → `DAY 3 OF 7`, centered bottom

**Overlay info card (top-left corner):**
```
┌──────────────────────────┐
│ LOCAL_SURVEY_VIEWPORT    │  ← label, tiny tracked mono
│ DAY 3 — CANVAS ENGINE    │  ← title, bold mono, 14px
│ 2026-02-17  12:00 UTC    │  ← metadata, small mono
└──────────────────────────┘
```

### 5.7 Status Bar (Fixed Bottom)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ● BUILT_WITH: RUST_2024    FRAMEWORK: LEPTOS_0.8    RECORDED_AT: 2026-02  │ BUILD_R-001-SURVEY │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Structure:**
- Full width, `height: 32px`, `background: var(--bg-status-bar)`
- 1px top border in `var(--border-strong)`
- All text: `--font-mono`, 10px, weight 400, tracked uppercase, `var(--text-tertiary)`
- Left side: status dot (● in `var(--accent-green)`) + key-value pairs separated by generous spacing
- Right side: build identifier, right-aligned

### 5.8 Tab Content — Overview

The "Overview" tab in the right panel shows the project summary:

```
DISCOVERY_DESCRIPTION / SUMMARY                    ERA / YEAR
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌─────────────────────────────────────────┐
│                                         │
│        [ARCHITECTURAL DIAGRAM]          │  ← engraving-style illustration
│        or [HERO SCREENSHOT]             │     of the system, or key screenshot
│                                         │
└─────────────────────────────────────────┘

COLLABBOARD — REAL-TIME COLLABORATIVE WHITEBOARD
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

A real-time collaborative whiteboard where multiple
users draw, chat, and let AI rearrange things on a
shared canvas — built entirely in Rust, from the
server all the way down to the browser.
```

**Illustration box:**
- Bordered container with `var(--bg-card-inset)` background
- Contains either a line-art/etching-style system diagram, or a screenshot with a slight desaturated warm filter applied via CSS
- For screenshots: `filter: sepia(15%) contrast(95%) brightness(98%)`

### 5.9 Illustration & Image Treatment

All images in the portfolio should feel like they belong in a vintage field report:

**Screenshots:**
```css
.screenshot {
  filter: sepia(12%) contrast(95%) brightness(98%) saturate(85%);
  border: 1px solid var(--border-strong);
  /* NO border-radius — sharp corners always */
  /* NO box-shadow */
}
```

**Diagrams/architecture visuals:**
- Rendered in a line-art/etching style where possible (black ink on parchment)
- If using actual diagrams (Mermaid, D3, etc.), style with the palette: `var(--text-primary)` lines, `var(--bg-card)` fills, `var(--border-medium)` secondary lines
- Avoid gradients, glows, or modern chart aesthetics

**Video embeds:**
- Contained in a bordered frame matching the illustration box pattern
- Optional "PLAYBACK_DEVICE" label above the embed in tiny tracked mono

---

## 6. Page-by-Page Layout Specifications

### 6.1 Overview (Landing Page)

The first thing a visitor sees. Must immediately communicate: what the project is, the tech stack, the scale, and the aesthetic sensibility of the builder.

**Stat strip (top, 4 columns):**
| Card | Number | Caption |
|------|--------|---------|
| `TOTAL_LINES_OF_CODE` | `41,578` | `SOURCE + TEST ACROSS 6 CRATES` |
| `BUILD_DURATION` | `7` | `DAYS · 382 COMMITS` |
| `TEST_RESULTS` | `989` | `PASSED · 0 FAILED` |
| `CODE_COVERAGE` | `41.2%` | `LINE COVERAGE` |

**Left panel:** Hero visual — a stylized diagram of the crate architecture (server ↔ client ↔ canvas ↔ frames) rendered as a "survey map" with connection lines. Or: a full-bleed screenshot of the running app, treated with the vintage filter.

**Left panel overlay (top-left):**
```
LOCAL_SURVEY_VIEWPORT
GAUNTLET WEEK 1 — SPRINT REPORT
2026-02-14 → 2026-02-20
```

**Left panel bottom center:** `PROJECT 1 OF 1` (maintaining the reference pattern)

**Right panel — Record Card:**
- Identity block: "CollabBoard" in script font
- Subtitle: `Ian · Rust · Leptos · Axum · 7 Days`
- Tabs: `OVERVIEW` | `ARCHITECTURE` | `METRICS`
- Default tab (Overview) shows: brief project description, then a "Quick Facts" key-value list:

```
QUICK_FACTS
━━━━━━━━━━━

LANGUAGE:    ··········  Rust (Edition 2024)
FRONTEND:    ··········  Leptos 0.8 (SSR + WASM)
BACKEND:     ··········  Axum 0.8 + SQLx 0.8
DATABASE:    ··········  PostgreSQL
WIRE_FORMAT: ··········  Protobuf (Prost 0.13)
AUTH:        ··········  GitHub OAuth + Email Codes
AI:          ··········  Anthropic / OpenAI
DEPLOY:      ··········  Railway + Docker
```

### 6.2 Architecture Page

Deep dive into the crate structure and design decisions.

**Left panel:** Crate dependency diagram styled as a geological survey map. Six "sites" (crates) positioned spatially with connection lines showing dependencies. Each site is a small labeled marker.

**Right panel — Record Card:**
- Identity block: "Architecture" in script font (or keep "CollabBoard" and change the card title to `CRATE_ARCHITECTURE`)
- Tabs: `OVERVIEW` | `CRATES` | `DECISIONS`
  - **OVERVIEW tab:** High-level architecture description (the system diagram explanation)
  - **CRATES tab:** Inventory-style list of all 6 crates with descriptions (use the 5.5 Inventory pattern)
  - **DECISIONS tab:** Key-value pairs of design decisions:
    ```
    DESIGN_DECISIONS
    ━━━━━━━━━━━━━━━━

    WHY_RUST_END_TO_END:
    "No JavaScript runtime anywhere in the stack..."

    WHY_PROTOBUF:
    "Binary wire format for WebSocket frames..."

    WHY_OUTCOME_ENUM:
    "Handlers return an Outcome — Broadcast, Reply,
    ReplyStream — and dispatch decides routing..."

    WHY_TWO_SPEED_PERSISTENCE:
    "Object dirty flush at 100ms, frame log queue
    with batched writer at 5ms..."
    ```

### 6.3 Timeline Page (Day-by-Day)

The heart of the portfolio — showing the build progression.

**Left panel:** Carousel of days. Each "slide" shows the primary screenshot or video for that day, treated with the vintage filter. Navigation with ◁ ▷ arrows. Bottom center: `DAY 3 OF 7`.

**Left panel overlay (top-left):**
```
LOCAL_SURVEY_VIEWPORT
DAY 3 — CANVAS ENGINE & HIT TESTING
2026-02-16
```

**Right panel — Record Card:**
- Tabs: `OVERVIEW` | `FIELD_NOTES` | `METRICS`
  - **OVERVIEW tab:** Description of what was built that day, illustrated with a screenshot
  - **FIELD_NOTES tab:** Narrative prose about challenges, decisions, breakthroughs (inventory-style list of entries)
  - **METRICS tab:** Day-specific stats:
    ```
    DAY_3_STATISTICS
    ━━━━━━━━━━━━━━━━

    COMMITS:      ··········  47
    LINES_ADDED:  ··········  3,200
    LINES_REMOVED:··········  890
    NET_DELTA:    ··········  +2,310
    FOCUS_AREA:   ··········  Canvas Engine
    CRATES_TOUCHED:·········  canvas, client
    KEY_FEATURE:  ··········  Hit Testing & Gesture FSM
    ```

### 6.4 Stack Page

Technology deep-dive.

**Left panel:** Visual representation of the tech stack — could be a layered diagram (browser → WASM → Leptos → WebSocket → Axum → PostgreSQL) styled as a geological cross-section / stratigraphy column.

**Right panel — Record Card:**
- Tabs: `OVERVIEW` | `DEPENDENCIES` | `STATS`
  - **OVERVIEW tab:** Stack narrative
  - **DEPENDENCIES tab:** Crate dependency table (from README stats)
  - **STATS tab:** Lines of code, functions, tests, coverage tables — all rendered as key-value lists

### 6.5 Demo Page

Live demo link and video walkthroughs.

**Left panel:** Embedded Loom video or screenshot carousel, in a bordered frame.

**Right panel — Record Card:**
- Tabs: `OVERVIEW` | `FEATURES` | `LINKS`
  - **OVERVIEW tab:** What the demo shows, how to interact
  - **FEATURES tab:** Feature inventory (drawing tools, AI integration, chat, follow camera, etc.)
  - **LINKS tab:** Live demo URL, Loom videos, GitHub repo

---

## 7. Interaction & Motion

### Philosophy

Motion is **minimal, functional, and analog-feeling.** Nothing bounces, nothing elastic. Think: a terminal cursor blinking, a mechanical dial clicking into place.

### Transitions

```css
:root {
  --transition-fast:    120ms ease-out;
  --transition-medium:  200ms ease-out;
  --transition-slow:    350ms ease-out;
}
```

| Interaction | Transition | Properties |
|------------|------------|------------|
| Nav hover | `--transition-fast` | `color` only |
| Tab switch | `--transition-medium` | `border-bottom-color`, `font-weight` (or use opacity crossfade on content) |
| Carousel slide | `--transition-slow` | `transform: translateX()` or `opacity` crossfade |
| Card content swap | `--transition-medium` | `opacity` fade (no slide) |
| Stat number count-up | 600ms, linear | On initial page load only, numbers count up from 0 |
| Status bar dot pulse | 2s, infinite | Subtle opacity pulse 1.0 → 0.6 → 1.0 on the green dot |

### Rules

1. **No elastic/spring easing.** Only `ease-out` or `linear`.
2. **No parallax scrolling.** Content scrolls at page speed.
3. **No hover-triggered size changes** (scale, grow). Only color and opacity shifts.
4. **Carousel transitions** are a simple crossfade or horizontal slide. No 3D transforms, no flip effects.
5. **Page transitions** (if using SPA routing): crossfade the content area, keep nav and status bar static.

---

## 8. Borders, Corners & Shadows

### The Rules (Non-Negotiable)

| Property | Value | Notes |
|----------|-------|-------|
| `border-radius` | `0` | **Always.** Every element has sharp 90° corners. |
| `box-shadow` | `none` | **Never.** No drop shadows, no glow, no elevation. |
| `border-width` | `1px` | **Always 1px.** Exception: active tab underline at 2px. |
| `border-style` | `solid` | **Always solid.** Exception: dot-leaders use `dotted`. |
| `outline` on focus | `2px solid var(--accent-active)` | For accessibility. Offset by 2px. |

### Border Hierarchy

- **Outer container borders:** `var(--border-strong)` — the perimeter of major panels
- **Internal dividers:** `var(--border-medium)` — between columns in the stat strip, between sections
- **Subtle structure:** `var(--border-light)` — below section labels, between key-value rows
- **Ghost lines:** `var(--border-faint)` — grid lines on the map panel, barely visible structure

---

## 9. Iconography

### Style

All icons should be:
- **Monoline, 1px stroke** in `var(--text-secondary)` or `var(--text-tertiary)`
- **Geometric and minimal** — no filled icons, no gradients
- **16×16 or 20×20px** at standard size
- Source: Lucide icons (open source, monoline) or custom SVG

### Specific Icons

| Location | Icon | Notes |
|----------|------|-------|
| Nav left | `⊞` grid or `☰` menu | 16px, `var(--text-tertiary)` |
| Nav right | `⊕` or external link | For GitHub link |
| Globe icon (nav far-right) | `⊕` circle with cross | Matches reference globe icon |
| Status dot | `●` filled circle | 8px, `var(--accent-green)` |
| Carousel arrows | `◁` `▷` | In 32px circular bordered buttons |
| Zoom controls | `+` `−` | In 28px square bordered buttons, stacked |
| Status indicators | `●` `◻` `◎` | Green dot = operational, box = count, target = search |

---

## 10. Accessibility

| Requirement | Implementation |
|-------------|---------------|
| Color contrast | All text meets WCAG AA (4.5:1 for normal, 3:1 for large) |
| Focus indicators | `outline: 2px solid var(--accent-active); outline-offset: 2px` |
| Keyboard navigation | All interactive elements reachable via Tab; carousel supports ← → |
| Screen reader | Semantic HTML (nav, main, article, section); ARIA labels on icon-only buttons |
| Reduced motion | `@media (prefers-reduced-motion: reduce)` disables all transitions |
| Font scaling | Layout doesn't break up to 200% browser zoom |

---

## 11. Technical Implementation Notes

### Recommended Stack for the Portfolio Site

Given the CollabBoard project is all-Rust, the portfolio site should ideally demonstrate web chops:

**Option A (Recommended):** Static HTML/CSS/JS — simple, fast, deployable anywhere. The vintage aesthetic doesn't need a framework. Use vanilla JS for tab switching, carousel, and counter animations.

**Option B:** React/Next.js if the implementer prefers — but keep it simple. The design doesn't call for complex state management.

### CSS Architecture

```
styles/
  variables.css      ← all custom properties from this spec
  reset.css          ← minimal reset (box-sizing, margin, etc.)
  typography.css     ← font imports, type scale classes
  layout.css         ← grid structure, panel splits, responsive breakpoints
  components.css     ← nav, stat-card, record-card, tab-bar, key-value, inventory
  pages.css          ← page-specific overrides
  utilities.css      ← tracked-uppercase, dot-leader, vintage-filter, etc.
```

### Key CSS Utilities

```css
/* Tracked uppercase with underscores (apply to labels) */
.label {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 400;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--text-tertiary);
}

/* Script display (project name) */
.display-script {
  font-family: var(--font-script);
  font-style: italic;
  font-size: clamp(36px, 5vw, 64px);
  color: var(--text-primary);
  line-height: 1.1;
}

/* Stat number */
.stat-number {
  font-family: var(--font-mono);
  font-size: 36px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--text-primary);
  line-height: 1.0;
}

/* Key-value row with dot leader */
.kv-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px dotted var(--border-light);
}
.kv-key {
  font-family: var(--font-mono);
  font-size: 12px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-secondary);
  white-space: nowrap;
}
.kv-value {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  text-align: right;
  white-space: nowrap;
}

/* Vintage screenshot filter */
.vintage-frame {
  border: 1px solid var(--border-strong);
  filter: sepia(12%) contrast(95%) brightness(98%) saturate(85%);
}

/* Card container */
.card {
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  padding: var(--card-padding);
}
```

### Google Fonts Import

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:ital,wght@0,300;0,400;0,500;0,600;0,700;1,400&family=Playfair+Display:ital,wght@0,400;0,700;1,400&display=swap" rel="stylesheet">
```

---

## 12. Content Mapping — Reference → Portfolio

| Reference Element | Portfolio Adaptation |
|---|---|
| `TAXA_INDEX` nav section | `COLLABBOARD` site name |
| Dinosaur species tabs (T_REX, TRICERATOPS...) | Page tabs (OVERVIEW, ARCHITECTURE, TIMELINE...) |
| `SECTORS` nav section | External links (GITHUB, DEMO, LOOM) |
| `GLOBAL_TAXA_DENSITY` stat card | `TOTAL_LINES_OF_CODE` stat card |
| `SECTOR_BIO_SIGNATURES` stat card | `BUILD_DURATION` stat card |
| `SURVEY_LOG_STATUS` stat card | `TEST_RESULTS` stat card |
| `DINO-DRAFT SURVEY RECORD` card title | `BUILD_SPRINT FIELD REPORT` |
| `BORROWER'S NAME / SPECIES ID` | `PROJECT / IDENTIFICATION` |
| Cursive species name ("Velociraptor") | Cursive project name ("CollabBoard") |
| `Project Field Lead` subtitle | `Ian · Rust · 7 Days` |
| `OVERVIEW / INVENTORY / METRICS` tabs | `OVERVIEW / ARCHITECTURE / METRICS` (or per-page variants) |
| `VITAL STATISTICS` key-value section | `QUICK_FACTS` or `CRATE_STATISTICS` |
| `RECOVERED SPECIMENS / TAXA` inventory | `CRATE_INVENTORY` or `FEATURE_INVENTORY` |
| Cartographic map with site markers | Architecture diagram / screenshot carousel |
| `LOCAL_SURVEY_VIEWPORT` overlay | Day/section context card |
| `SITE 3 OF 7` pagination | `DAY 3 OF 7` pagination |
| `ENGINE: GEMINI_FLASH_LITE` status bar | `BUILT_WITH: RUST_2024` status bar |
| `BUILD_R-990-VINTAGE` build ID | `BUILD_R-001-SPRINT` or actual git hash |
| Etching-style dinosaur illustration | Line-art system diagram or architectural sketch |
| `EPOCH_ID: Mesozoic` metadata | `EDITION: 2024` metadata |
| `CONFIDENCE: 0.982 ALPHA` | `TEST_PASS_RATE: 100%` |

---

## 13. Do / Don't Quick Reference

| ✓ Do | ✗ Don't |
|---|---|
| Use monospace everywhere for labels and body | Use sans-serif display fonts |
| Replace spaces with underscores in labels | Use camelCase or Title Case for field names |
| Keep all colors warm and desaturated | Use any saturated blue, purple, or bright accent |
| Use 1px borders, no shadows | Use box-shadow, drop-shadow, or glow effects |
| Square corners on everything | Round any corners (border-radius > 0) |
| Use uppercase with wide tracking for labels | Use sentence case for metadata labels |
| Include ONE script/handwritten element per view | Overuse the script font |
| Use engraving/etching style for diagrams | Use photography or modern flat illustration |
| Keep motion minimal and mechanical | Add playful, elastic, or bouncy animations |
| Design as if rendering on a CRT/thermal printer | Design as if it's a modern SaaS product |
| Use dot-leaders between keys and values | Use plain whitespace or colon-only separation |
| Maintain generous padding inside cards | Cram content edge-to-edge |
| Let stats be the largest type on screen | Make headings the largest type |
| Use 1px solid borders only | Use 2px+ borders, dashed, double, or decorative |
| Apply vintage filter to all screenshots | Show raw, saturated screenshots |

---

## 14. Asset Checklist

The implementing developer will need:

- [ ] Screenshots for each day (Day 1–7), ideally 16:9 or similar
- [ ] Loom video embed URLs (Day 2 MVP, Day 5 Early Release, plus any others)
- [ ] Live demo URL (Railway deployment)
- [ ] GitHub repo URL
- [ ] An architectural diagram or system sketch (can be generated as SVG)
- [ ] Optionally: a hand-drawn or etching-style illustration of the CollabBoard concept (for the Overview hero)
- [ ] Project stats from the README (already provided — lines of code, tests, coverage, etc.)
- [ ] Day-by-day commit counts / focus areas / narrative notes

---

## 15. Mood Board Keywords

For communicating this style to collaborators, AI image generators, or search:

> **vintage scientific survey terminal, parchment UI, field research station, analog instrument panel, 1970s government database, typewriter interface, warm monochrome dashboard, retro data visualization, institutional utilitarian design, cartographic survey tool, engineering field report, build log terminal, sprint documentation station**
