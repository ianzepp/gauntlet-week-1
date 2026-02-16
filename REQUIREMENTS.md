# Collaboard Clone — Implementation Requirements

Clean-room recreation of [Collaboard](https://www.collaboard.app/), a real-time collaborative whiteboard application.

## Core Concept

An infinite-canvas whiteboard where multiple users can collaborate in real time — adding sticky notes, drawing, placing shapes, embedding media, and organizing ideas visually.

---

## Feature Areas

### 1. Canvas

- **Infinite canvas** — pan and zoom without boundaries
- **Zoom controls** — zoom in/out, fit to content, zoom to selection
- **Grid/snap** — optional grid alignment for precise placement
- **Quick links** — bookmarked regions for fast navigation on large boards
- **Minimap** — overview of the full board with viewport indicator

### 2. Objects / Content Types

- **Sticky notes** — colored cards with text, resizable, multiple color options
- **Text blocks** — rich text with formatting (bold, italic, links, fonts, colors)
- **Shapes** — rectangles, circles, triangles, arrows, lines, connectors
- **Freehand drawing** — pen/pencil/marker/brush tools with color and thickness
- **Images** — drag-and-drop upload (JPG, PNG, SVG), resize, crop
- **Documents** — embed/preview PDFs, DOCX, PPTX, XLSX
- **Video/audio** — embed media, YouTube links, direct recording
- **Connectors/lines** — lines between objects that stay attached when objects move
- **Tables** — grid-based structured content

### 3. Object Manipulation

- **Select** — click to select, drag to multi-select, shift-click to add to selection
- **Move** — drag objects or groups
- **Resize** — drag handles, maintain aspect ratio (optional)
- **Rotate** — rotation handle
- **Copy/paste** — duplicate objects
- **Z-order** — bring to front, send to back, layer management
- **Lock** — prevent accidental edits to specific objects
- **Pin** — fix position on canvas (immune to accidental drag)
- **Hide** — temporarily hide objects (facilitator feature)
- **Group** — combine objects into a movable group

### 4. Real-Time Collaboration

- **Live cursors** — see other users' cursor positions with name labels and colors
- **Simultaneous editing** — multiple users editing the same board concurrently
- **Conflict resolution** — operational transform or CRDT for concurrent edits
- **Presence indicators** — who's online, user avatars on the board
- **WebSocket transport** — persistent connection for low-latency sync

### 5. Access Control & Sharing

- **Board roles** — owner, facilitator/moderator, editor, viewer
- **Guest access** — join without registration (via invite link)
- **Password-protected boards** — optional password on invite links
- **Permission management** — change user roles per board

### 6. Facilitation / Moderation Tools

- **Presentation mode** — all participants follow the presenter's viewport
- **Focus/attention mode** — highlight a specific area for all users
- **Timer** — countdown timer visible to all participants
- **Voting/rating** — anonymous votes on objects (e.g., dot voting on sticky notes)
- **Vote archive** — store voting results

### 7. Organization

- **Rooms** — group related boards into rooms/folders
- **Tags** — label boards for search and filtering
- **Board list** — dashboard of all accessible boards
- **Templates** — pre-built board layouts (SWOT, Kanban, mind map, retro, etc.)
- **Version history** — save and restore board snapshots
- **Activity log** — track who changed what and when

### 8. Export

- **Image export** — PNG/JPG of full board or selected area
- **PDF export** — high-resolution board export
- **Data export** — JSON or structured format for backup/migration

### 9. Authentication

- **Email/password** registration and login
- **OAuth/SSO** — social login or enterprise SSO
- **2FA** — optional two-factor authentication
- **JWT-based sessions**

---

## Technical Scope (MVP vs Full)

### MVP — Phase 1
Focus on the core whiteboard experience:

1. **Canvas** — infinite pan/zoom with a blank board
2. **Sticky notes** — create, edit text, change color, move, resize
3. **Basic shapes** — rectangles, circles, lines
4. **Freehand drawing** — single pen tool with color/thickness
5. **Real-time sync** — WebSocket-based, live cursors, multi-user editing
6. **Auth** — basic email/password login, JWT sessions
7. **Board CRUD** — create, list, open, delete boards
8. **Access** — share board via link, editor/viewer roles

### Phase 2
Layer on collaboration and organization:

9. Text blocks with rich formatting
10. Connectors that attach to objects
11. Image upload and embedding
12. Voting/dot voting on objects
13. Timer and presentation mode
14. Rooms and tags for board organization
15. Templates (5-10 starter templates)
16. Version history / snapshots

### Phase 3
Polish and advanced features:

17. Document/video embedding
18. Advanced shapes and tables
19. Export (PNG, PDF, JSON)
20. Guest access without registration
21. Activity log
22. 2FA and SSO

---

## Technical Architecture

### Backend — Generic Data API

Following the monk-api pattern: a thin, generic CRUD layer over SQLite.

**REST API**: `GET/POST/PUT/DELETE /api/data/:type/:id`
- `:type` maps 1:1 to a SQLite table (e.g., `sticky_notes`, `shapes`, `drawings`, `boards`, `users`)
- One table per object type — not a single polymorphic `board_objects` table
- **Dynamic schema**: columns created silently on first insert. If a POST includes a property the table doesn't have yet, `ALTER TABLE ADD COLUMN` creates it on the fly. No type registry or validation — just add the column.
- Formalize schema with Drizzle migrations once data shapes stabilize late in dev.
- All objects carry standard fields: `id` (UUID), `created_at`, `updated_at`, `created_by`
- Board-scoped objects also carry `board_id` for filtering

**Example object types** (tables created dynamically as needed):
- `users` — auth identity
- `boards` — board metadata (title, owner_id)
- `board_members` — board_id, user_id, role
- `sticky_notes` — board_id, x, y, width, height, text, color, z_index, ...
- `shapes` — board_id, x, y, width, height, shape_type, fill, stroke, ...
- `drawings` — board_id, path_data, color, thickness, ...
- `connectors` — board_id, from_id, to_id, path_type, ...
- `text_blocks` — board_id, x, y, content, font_size, ...

**Filtering**: query params on GET, e.g., `GET /api/data/sticky_notes?board_id=abc`

### Real-Time Sync

- **WebSocket** via Hono's WebSocket upgrade support
- **Per-board rooms**: clients join a WS room for their board
- **Broadcast on mutation**: when a REST write (POST/PUT/DELETE) succeeds, broadcast the change to all connected clients on that board
- **Live cursors**: lightweight WS messages (user_id, x, y) broadcast to room, not persisted
- **Conflict resolution**: last-write-wins at the row level (sufficient for MVP scale). Revisit CRDT if needed.

### Frontend

- **Dashboard/auth pages**: Hono + htmx (server-rendered HTML)
- **Whiteboard canvas**: HTML5 Canvas or WebGL for rendering (not DOM — performance at scale)
- Consider: **Konva.js**, **Fabric.js**, **PixiJS**, or raw Canvas 2D API
- Canvas client fetches board objects via REST on load, then subscribes to WS for live updates
- Keyboard shortcuts for power users
- Touch support for tablets

### Stack Summary
- **Runtime**: Bun
- **Backend**: Hono (REST API + WebSocket)
- **Database**: SQLite (raw `bun:sqlite`, dynamic schema; formalize with Drizzle later)
- **Frontend**: htmx (dashboard), Canvas lib (whiteboard)
- **Auth**: JWT, bcrypt for passwords
- **Real-time**: WebSocket broadcast, last-write-wins
