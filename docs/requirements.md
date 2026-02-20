# CollabBoard — Post-MVP Requirements

All items below are beyond the 24-hour MVP hard gate. Organized by section from the project spec.

---

## MVP Requirements (Hard Gate)

- [x] Infinite board with pan/zoom
- [x] Sticky notes with editable text
- [x] At least one shape type (rectangle, circle, or line)
- [x] Create, move, and edit objects
- [x] Real-time sync between 2+ users
- [x] Multiplayer cursors with name labels
- [x] Presence awareness (who's online)
- [x] User authentication
- [x] Deployed and publicly accessible

---

## Core Whiteboard — Board Features

- [x] **Workspace** — Infinite board with smooth pan/zoom
- [~] **Sticky Notes** — Create, edit text, change colors — *no color picker UI; uses generic text/rect object, no preset sticky styling*
- [x] **Shapes** — Rectangles, circles, lines with solid colors
- [x] **Connectors** — Lines/arrows connecting objects
- [x] **Text** — Standalone text elements
- [x] **Frames** — Group and organize content areas
- [x] **Transforms** — Move, resize, rotate objects
- [~] **Selection** — Single and multi-select (shift-click, drag-to-select) — *single select only; no shift-click, no drag-to-select box*
- [~] **Operations** — Delete, duplicate, copy/paste — *delete only; no duplicate, no copy/paste*

---

## Core Whiteboard — Real-Time Collaboration

- [x] **Cursors** — Multiplayer cursors with names, real-time movement
- [x] **Sync** — Object creation/modification appears instantly for all users
- [x] **Presence** — Clear indication of who's currently on the board
- [x] **Conflicts** — Handle simultaneous edits (last-write-wins acceptable; document approach)
- [x] **Resilience** — Graceful disconnect/reconnect handling
- [x] **Persistence** — Board state survives all users leaving and returning

---

## Performance Targets

- [~] Frame rate: 60 FPS during pan, zoom, object manipulation — *event-driven rendering, no rAF loop or culling; likely 30-40 FPS at 500+ objects due to O(n) iteration and per-frame text measurement*
- [~] Object sync latency: <100ms — *33ms client-side drag throttle + RTT puts worst-case at ~120ms; borderline*
- [~] Cursor sync latency: <50ms — *cursor moves send immediately with no throttle; 40ms cap only applies to camera-only updates (pan/zoom with no cursor); achievability depends on network RTT*
- [~] Object capacity: 500+ objects without performance drops — *no viewport culling; all objects rendered every frame regardless of visibility*
- [x] Concurrent users: 5+ without degradation — *async broadcast with 256-frame per-client buffer; no server bottlenecks identified*

### Performance TODOs

- [ ] Implement viewport culling — skip rendering objects outside the current camera view to reduce O(n) per-frame cost

---

## Testing Scenarios

- [x] 2 users editing simultaneously in different browsers
- [ ] One user refreshing mid-edit (state persistence check) — *in-flight edits not guaranteed to survive refresh; 100ms async flush window means last edit may be lost*
- [~] Rapid creation and movement of sticky notes and shapes (sync performance) — *`board_complexity_object_create_perf_test` in perf crate covers object creation at scale; movement not covered*
- [ ] Network throttling and disconnection recovery — *reconnect logic exists; manual throttle test required*
- [~] 5+ concurrent users without degradation — *`mass_user_concurrent_perf_test` in perf crate covers concurrent load; requires live server to run*

---

## AI Board Agent

### Required Capabilities (6+ distinct commands)

**Creation Commands**
- [x] "Add a yellow sticky note that says 'User Research'"
- [x] "Create a blue rectangle at position 100, 200"
- [x] "Add a frame called 'Sprint Planning'"

**Manipulation Commands**
- [x] "Move all the pink sticky notes to the right side"
- [x] "Resize the frame to fit its contents"
- [x] "Change the sticky note color to green"

**Layout Commands**
- [x] "Arrange these sticky notes in a grid"
- [x] "Create a 2x3 grid of sticky notes for pros and cons"
- [x] "Space these elements evenly"

**Complex Commands**
- [x] "Create a SWOT analysis template with four quadrants"
- [x] "Build a user journey map with 5 stages"
- [x] "Set up a retrospective board with What Went Well, What Didn't, and Action Items columns"

### Tool Schema (Minimum)

- [x] `createStickyNote(text, x, y, color)`
- [x] `createShape(type, x, y, width, height, color)`
- [x] `createFrame(title, x, y, width, height)`
- [x] `createConnector(fromId, toId, style)`
- [x] `moveObject(objectId, x, y)`
- [x] `resizeObject(objectId, width, height)`
- [x] `updateText(objectId, newText)`
- [x] `changeColor(objectId, color)`
- [x] `getBoardState()` — returns current board objects for context

### Shared AI State

- [x] All users see AI-generated results in real-time
- [x] Multiple users can issue AI commands simultaneously without conflict

### AI Agent Performance Targets

- [ ] Response latency: <2 seconds for single-step commands — *Sonnet + YAML mode takes 2-10s with tool calls; consider switching to Haiku for speed or disabling YAML_ONLY_MODE for simple single-step commands*
- [x] Command breadth: 6+ command types
- [x] Complexity: multi-step operation execution
- [x] Reliability: consistent, accurate execution — *retry logic + stale version handling implemented*

---

## AI-First Development Requirements

- [x] Use at least 2 AI coding tools from: Claude Code, Cursor, Codex, MCP integrations — *Claude Code confirmed; Codex also used*

### AI Development Log (Required — 1-page document)

- [ ] **Tools & Workflow** — Which AI coding tools used and how they were integrated
- [ ] **MCP Usage** — Which MCPs used (if any), what they enabled
- [ ] **Effective Prompts** — 3–5 prompts that worked well (include the actual prompts)
- [ ] **Code Analysis** — Rough % of AI-generated vs hand-written code
- [ ] **Strengths & Limitations** — Where AI excelled, where it struggled
- [ ] **Key Learnings** — Insights about working with coding agents

### AI Cost Analysis (Required)

**Development & Testing Costs**
- [ ] LLM API costs (OpenAI, Anthropic, etc.)
- [ ] Total tokens consumed (input/output breakdown)
- [ ] Number of API calls made
- [ ] Any other AI-related costs (embeddings, hosting, etc.)

**Production Cost Projections**
- [ ] Monthly cost estimate at 100 users
- [ ] Monthly cost estimate at 1,000 users
- [ ] Monthly cost estimate at 10,000 users
- [ ] Monthly cost estimate at 100,000 users
- [ ] Include assumptions: avg AI commands per user per session, avg sessions per user per month, token counts per command type

---

## Submission Requirements (Deadline: Sunday 10:59 PM CT)

- [x] **GitHub Repository** — Setup guide, architecture overview, deployed link — *README covers setup and architecture; deployed at https://gauntlet-week-1-production.up.railway.app/*
- [~] **Demo Video (3–5 min)** — Real-time collaboration, AI commands, architecture explanation — *multiple dev process videos exist; no final end-to-end demo yet*
- [x] **Pre-Search Document** — Completed checklist from Phases 1–3 — *docs/PRE-SEARCH.md*
- [ ] **AI Development Log** — 1-page breakdown using template above — *pending; drafting from transcript dailies*
- [ ] **AI Cost Analysis** — Dev spend + projections for 100/1K/10K/100K users
- [x] **Deployed Application** — Publicly accessible, supports 5+ users with auth — *live at https://gauntlet-week-1-production.up.railway.app/ on Railway*
- [x] **Social Post** — Share on X or LinkedIn: description, features, demo/screenshots, tag @GauntletAI
