# CollabBoard Design System

Visual identity adapted from the Abbot "Field Survey Terminal" aesthetic.
Reference: `~/github/ianzepp/abbot-web/index.css`

## Aesthetic

**Retro-scientific research lab.** The entire app looks like a monitoring station
from a 1970s-80s field research facility. Warm earth tones, monospace typography,
zero decoration. No rounded corners. No drop shadows. No gradients. No blur.
Every pixel serves a purpose.

The canvas is a field notebook page — warm beige with a light pencil grid.
Sticky notes look handwritten. The toolbar and panels look like instrument
readouts.

## Fonts

```
@import url('https://fonts.googleapis.com/css2?family=Caveat:wght@400;500&family=IBM+Plex+Mono:wght@400;500;600;700&display=swap');
```

| Context | Font | Weight | Style |
|---------|------|--------|-------|
| UI chrome (toolbar, panels, nav, labels) | IBM Plex Mono | 400-700 | uppercase, letter-spacing 0.04-0.1em |
| Sticky note text | Caveat | 400-500 | normal case, 18-24px |
| Shape labels / text objects | Caveat | 400 | normal case |
| Status text, metadata | IBM Plex Mono | 400 | 10-11px, uppercase |
| Board title | IBM Plex Mono | 700 | 14px, uppercase |
| Presence labels (cursor names) | IBM Plex Mono | 500 | 10px, uppercase |

## Color Palette

### Light Mode (Default)

```css
:root {
  /* Canvas */
  --canvas-bg:         #E8E0D2;  /* warm beige */
  --canvas-grid:       #D4CFC6;  /* tan grid lines */
  --canvas-grid-major: #C8C0B4;  /* darker grid every 5th line */

  /* UI Chrome */
  --bg-primary:        #F5F0E8;  /* panels, sidebars */
  --bg-secondary:      #EDE8DD;  /* nested panels, inputs */
  --bg-nav:            #2C2824;  /* top toolbar bar */
  --bg-status-bar:     #F0EBE2;  /* status bar */

  /* Text */
  --text-primary:      #2C2824;  /* dark brown */
  --text-secondary:    #8A8178;  /* medium brown */
  --text-tertiary:     #B5AEA4;  /* muted, placeholder */
  --text-nav:          #D4CFC6;  /* toolbar inactive */
  --text-nav-active:   #F5F0E8;  /* toolbar active */

  /* Accents */
  --accent-green:      #2C6E49;  /* online, success, active tool */
  --accent-error:      #8B4049;  /* errors, delete actions */

  /* Borders */
  --border-default:    #D4CFC6;  /* standard border */
  --border-subtle:     #E5E0D8;  /* very light separator */

  /* Object Colors (sticky notes, shapes) */
  --obj-cream:         #F5F0E8;  /* default sticky note */
  --obj-sage:          #B8C5B0;  /* green-gray */
  --obj-terracotta:    #C4A882;  /* warm tan */
  --obj-slate:         #9AA3AD;  /* blue-gray */
  --obj-dust:          #C2A8A0;  /* dusty rose */
  --obj-gold:          #C9B97A;  /* muted gold */
  --obj-stone:         #A8A298;  /* warm gray */
  --obj-moss:          #8B9E7E;  /* deep sage */

  /* Typography */
  --font-mono:         'IBM Plex Mono', 'Courier Prime', 'JetBrains Mono', monospace;
  --font-script:       'Caveat', cursive;

  /* Spacing */
  --space-xs:  4px;
  --space-sm:  8px;
  --space-md:  16px;
  --space-lg:  24px;
  --space-xl:  32px;

  /* No decoration */
  --radius: 0;
  --shadow: none;
}
```

### Dark Mode

```css
.dark-mode {
  --canvas-bg:         #1F1D1A;
  --canvas-grid:       #2A2724;
  --canvas-grid-major: #3A3632;

  --bg-primary:        #1C1A18;
  --bg-secondary:      #252220;
  --bg-nav:            #0F0E0D;
  --bg-status-bar:     #1C1A18;

  --text-primary:      #E8E0D2;
  --text-secondary:    #9A9488;
  --text-tertiary:     #5A554D;
  --text-nav:          #7A756D;
  --text-nav-active:   #E8E0D2;

  --accent-green:      #3D9463;
  --accent-error:      #B85A63;

  --border-default:    #3A3632;
  --border-subtle:     #2A2724;

  --obj-cream:         #2C2824;
  --obj-sage:          #3A4436;
  --obj-terracotta:    #4A3D30;
  --obj-slate:         #343A40;
  --obj-dust:          #443838;
  --obj-gold:          #3E3A28;
  --obj-stone:         #343230;
  --obj-moss:          #2E3828;
}
```

## User Colors (Presence / Cursors)

Muted, earthy tones that don't clash with the palette. Each connected user gets
one. Text on cursors is IBM Plex Mono 10px uppercase.

```css
--user-0: #2C6E49;  /* forest green */
--user-1: #8B4049;  /* muted rose */
--user-2: #4A6B8A;  /* dusty blue */
--user-3: #8B6E4E;  /* earth brown */
--user-4: #6B5B7B;  /* muted purple */
--user-5: #7A8B4A;  /* olive */
--user-6: #8B5E3C;  /* sienna */
--user-7: #4A7B6B;  /* teal */
```

## Canvas

- Background: `--canvas-bg` (warm beige)
- Grid: 20px spacing, 1px lines in `--canvas-grid`, every 5th line (100px) in `--canvas-grid-major`
- Grid opacity: 0.5 (subtle, not dominant)
- No dot grid — use line grid for the "graph paper" / field notebook feel

## Sticky Notes

- Sharp corners (no border-radius)
- 1px solid border in a slightly darker shade of the note color
- Font: Caveat 20px, normal case
- Slight rotation (±1-2°) on creation for organic feel — snaps to 0° on drag
- Default size: 200x200
- No drop shadow — use a 1px darker bottom/right border to suggest depth
- Text color: `--text-primary` (dark brown, same in all note colors)
- Double-click to edit: DOM textarea overlay with Caveat font

### Note Color Palette

| Name | Light | Dark | Use |
|------|-------|------|-----|
| Cream | `#F5F0E8` | `#2C2824` | Default |
| Sage | `#B8C5B0` | `#3A4436` | Ideas, growth |
| Terracotta | `#C4A882` | `#4A3D30` | Warm, action |
| Slate | `#9AA3AD` | `#343A40` | Technical, cold |
| Dust | `#C2A8A0` | `#443838` | Questions, soft |
| Gold | `#C9B97A` | `#3E3A28` | Important, highlight |
| Stone | `#A8A298` | `#343230` | Neutral |
| Moss | `#8B9E7E` | `#2E3828` | Done, resolved |

## Shapes (Rectangle, Ellipse, Line)

- 1px solid stroke in `--text-secondary` (#8A8178)
- No fill by default (transparent)
- Fill optional, uses same palette as sticky notes
- Selected: stroke changes to `--text-primary` (dark brown)
- Sharp corners on rectangles (no border-radius)

## Connectors

- 1px solid line in `--text-secondary`
- Arrowhead: small, simple triangle
- Selected: 2px, `--text-primary`
- No curves — straight lines or right-angle segments only (fits the grid aesthetic)

## Toolbar

- Position: top of viewport, full width
- Background: `--bg-nav` (dark charcoal-brown)
- Height: 44px
- Tool icons: simple, monoline, 18x18px, `--text-nav` color, `--text-nav-active` when selected
- Active tool: bottom border 2px `--accent-green`
- Labels: IBM Plex Mono 10px uppercase, `--text-nav`
- Separator: 1px vertical line in `--border-default`
- Tool groups: [Select | Sticky | Rectangle | Ellipse | Line | Connector | Text] | [Undo | Redo] | [AI Prompt]

## Panels (Board List, Inspector, AI Chat)

- Background: `--bg-primary`
- Border: 1px solid `--border-default` on the canvas-facing edge
- Headers: IBM Plex Mono 11px 600 uppercase, letter-spacing 0.06em
- No rounded corners, no shadows
- Scrollbar: 8px, thumb in `--border-default`

## Presence Indicators

- Cursor: small arrow in user color + name label
- Label: IBM Plex Mono 10px uppercase, user color background, `--text-nav-active` text
- Board header: row of user avatars (small, square, no border-radius) with colored border
- Selection highlight: 1px dashed border in user color around selected object

## Interactive States

| State | Treatment |
|-------|-----------|
| Hover | Background shifts one step darker (e.g., primary → secondary) |
| Active/Selected | Border color → `--text-primary`, text bold |
| Disabled | opacity: 0.5 |
| Focus | border-color: `--text-primary` (no glow, no outline) |
| Error | border-color: `--accent-error`, text-color: `--accent-error` |

Transitions: 150ms ease on all interactive properties.

## Spacing

Use the spacing scale consistently:
- `4px` — tight gaps (icon to label)
- `8px` — within components (padding, small gaps)
- `16px` — between components
- `24px` — section spacing
- `32px` — major sections

## Rules

1. No `border-radius` anywhere. Not on buttons, not on inputs, not on avatars, not on sticky notes.
2. No `box-shadow` anywhere. Depth is suggested through border weight or color shift.
3. No gradients. Solid colors only.
4. No blur or glassmorphism.
5. UI text is always IBM Plex Mono, uppercase, with letter-spacing.
6. Content text (sticky notes, text objects) is always Caveat, normal case.
7. Font sizes in the chrome stay small (10-13px). The canvas objects are larger.
8. Every color must come from the CSS variables. No hardcoded hex values in components.
9. Dark mode must work from day one. Use variables everywhere.
10. Animations are minimal: 150ms transitions only. No spring physics, no bounces.
