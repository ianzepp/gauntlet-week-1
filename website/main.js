/* CollabBoard Portfolio — Vanilla JS */
(function () {
  'use strict';

  var prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  /* --- Nav Page Switching --- */
  var navButtons = document.querySelectorAll('.nav-item[data-page]');
  var pages = document.querySelectorAll('.page[data-page]');

  var validPages = [];
  navButtons.forEach(function (btn) { validPages.push(btn.getAttribute('data-page')); });

  function switchPage(pageName, updateHash) {
    if (validPages.indexOf(pageName) === -1) pageName = 'overview';
    navButtons.forEach(function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-page') === pageName);
      if (btn.getAttribute('data-page') === pageName) {
        btn.setAttribute('aria-current', 'page');
      } else {
        btn.removeAttribute('aria-current');
      }
    });
    pages.forEach(function (page) {
      page.classList.toggle('active', page.getAttribute('data-page') === pageName);
    });
    if (updateHash !== false) {
      history.pushState(null, '', '#' + pageName);
    }
  }

  navButtons.forEach(function (btn) {
    btn.addEventListener('click', function () {
      switchPage(btn.getAttribute('data-page'));
    });
  });

  /* --- Hash-based Routing --- */
  function getPageFromHash() {
    var hash = window.location.hash.replace('#', '');
    return hash || 'overview';
  }

  window.addEventListener('popstate', function () {
    switchPage(getPageFromHash(), false);
  });

  /* Load initial page from URL hash */
  var initialPage = getPageFromHash();
  if (initialPage !== 'overview') {
    switchPage(initialPage, false);
  }

  /* --- Hamburger Menu --- */
  var hamburger = document.querySelector('.nav-hamburger');
  var navBar = document.querySelector('.nav-bar');

  if (hamburger) {
    hamburger.addEventListener('click', function () {
      var isOpen = navBar.classList.toggle('nav-open');
      hamburger.setAttribute('aria-expanded', isOpen);
    });

    /* Close menu when a nav item is clicked on mobile */
    navButtons.forEach(function (btn) {
      btn.addEventListener('click', function () {
        navBar.classList.remove('nav-open');
        hamburger.setAttribute('aria-expanded', 'false');
      });
    });
  }

  /* --- Tab Switching --- */
  var tabBars = document.querySelectorAll('.tab-bar');

  tabBars.forEach(function (bar) {
    var tabs = bar.querySelectorAll('.tab');
    tabs.forEach(function (tab) {
      tab.addEventListener('click', function () {
        var targetId = tab.getAttribute('data-tab');

        /* Deactivate siblings */
        tabs.forEach(function (t) { t.classList.remove('active'); });
        tab.classList.add('active');

        /* Find closest page or record-card container to scope content lookup */
        var container = bar.closest('.record-card') || bar.closest('.page');
        if (!container) return;

        var contents = container.querySelectorAll('.tab-content');
        contents.forEach(function (tc) {
          tc.classList.toggle('active', tc.getAttribute('data-tab-content') === targetId);
        });
      });
    });
  });

  /* --- Stat Counter Animation --- */
  function animateCounters() {
    var statNumbers = document.querySelectorAll('.stat-number[data-target]');
    var duration = prefersReducedMotion ? 0 : 600;

    statNumbers.forEach(function (el) {
      var target = parseFloat(el.getAttribute('data-target'));
      var suffix = el.getAttribute('data-suffix') || '';
      var decimals = parseInt(el.getAttribute('data-decimals') || '0', 10);

      if (duration === 0) {
        el.textContent = formatNumber(target, decimals) + suffix;
        return;
      }

      var start = performance.now();

      function step(now) {
        var elapsed = now - start;
        var progress = Math.min(elapsed / duration, 1);
        var current = target * progress;
        el.textContent = formatNumber(current, decimals) + suffix;
        if (progress < 1) {
          requestAnimationFrame(step);
        }
      }

      requestAnimationFrame(step);
    });
  }

  function formatNumber(num, decimals) {
    if (decimals > 0) {
      return num.toFixed(decimals);
    }
    return Math.round(num).toLocaleString('en-US');
  }

  animateCounters();

  /* --- Timeline Day Data --- */
  var timelineDays = [
    {
      day: 1, date: '2026-02-16', title: 'FULL-STACK SCAFFOLD', commits: 40,
      fieldNote: 'Stood up the entire stack in a single day: Rust/Axum backend, React/Konva frontend, GitHub OAuth, WebSocket sync, AI tool loop, and frame persistence. Iterated the DB flush strategy three times before settling on direct writes.',
      clusters: [
        { name: 'BUG_FIXES_AND_INFRA', commits: 7, summary: 'Fixed a stream of integration issues: Frame deserialization with null handling and valid UUIDs, AI "stuck on thinking" from unfiltered blocks, and canvas object selection/transform bugs. Added dotenvy for .env loading, structured logging, static file serving with SPA fallback, and a docker-compose file.' },
        { name: 'PRE_SEARCH_AND_DESIGN_DOCS', commits: 6, summary: 'Drafted and iterated on the CollabBoard pre-search document covering budget, auth, architecture, and testing strategy. Added the project brief PDF, design system spec, and organized all docs into a docs/ directory.' },
        { name: 'PERSISTENCE_LAYER', commits: 6, summary: 'Added buffered frame persistence to the frames table, then tightened the flush interval from 1s to 100ms and switched to sleep-after-flush. Later removed the buffer entirely in favor of direct DB persistence on each inbound frame.' },
        { name: 'BACKEND_AND_FRONTEND_SCAFFOLD', commits: 5, summary: 'Stood up the Rust/Axum backend scaffold and the React/Vite/Konva frontend scaffold. Moved backend into server/, added the AI agent loop and WebSocket dispatch entry points.' },
        { name: 'UI_LAYOUT_AND_FRONTEND', commits: 5, summary: 'Redesigned the UI with a left tool rail, tabbed right panel, and board stamp. Added the AI chat panel as a collapsible sidebar, placeholder drawing tools, and Transformer-based selection/resize.' },
        { name: 'AI_SERVICE_AND_TOOLING', commits: 4, summary: 'Built the LLM multi-provider adapter with rate limiting and prompt injection defense. Replaced the consolidated tool set with 9 spec-matching tools.' },
        { name: 'AUTH_AND_REALTIME_SYNC', commits: 4, summary: 'Implemented GitHub OAuth authentication with env var configuration. Wired up WebSocket real-time sync with cursor presence broadcasting across connected clients.' },
        { name: 'USER_STATS_AND_PROFILES', commits: 3, summary: 'Added user field report popovers on status bar chips showing per-user activity. Fixed profile stats to stamp user_id on frames and query both in-memory boards and legacy frame data.' }
      ]
    },
    {
      day: 2, date: '2026-02-17', title: 'CANVAS REBUILD & PANEL LAYOUT', commits: 76,
      fieldNote: 'Gutted the canvas and rebuilt from scratch with full-viewport rendering, hit-testing, and inline text editing. Redesigned the right panel into a tabbed layout with chat, AI, and inspector. Added board dashboard and deployment pipeline.',
      clusters: [
        { name: 'RIGHT_PANEL_REDESIGN', commits: 12, summary: 'Replaced the right panel toggle with an always-visible collapsed icon rail, then iterated heavily \u2014 extracting tabs for Boards, Chat, AI, and Inspector. Added real-time chat, board switcher, and open/close chevron.' },
        { name: 'CANVAS_GUTTING_AND_REBUILD', commits: 10, summary: 'Gutted the existing canvas and replaced it with a full-viewport layer: grid overlay, pan/zoom, coordinate display. Rebuilt rectangle creation, selectable dragging, hit-testing, inline text editing, and sticky notes from scratch.' },
        { name: 'INSPECTOR_AND_VISUAL_POLISH', commits: 9, summary: 'Added inspector controls for font size and border width, moved presence chips to the top bar, merged the tool rail into a unified left panel. Added confirmed delete with keyboard shortcut and tuned selection ring animation.' },
        { name: 'AI_AND_CHAT_FEATURES', commits: 8, summary: 'Added chat:history and ai:history syscalls, feeding recent conversation into LLM context scoped to the authenticated user. Rendered markdown in AI responses and fixed tool mutations to use current object versions.' },
        { name: 'DEPLOYMENT_AND_DEVOPS', commits: 8, summary: 'Added run-dev.sh and switched to Docker Compose. Prepared Fly.io deployment, then removed it. Renamed project to gauntlet-week-1 and isolated tests from the live database.' },
        { name: 'SERVER_TEST_INFRASTRUCTURE', commits: 7, summary: 'Moved server tests into dedicated *_test.rs files per project convention. Added integration suites for WebSocket AI flows, multi-user sync, board service syscalls, and chat/history.' },
        { name: 'GRID_AND_VIEWPORT_LAYOUT', commits: 7, summary: 'Added Battleship-style grid coordinates to the viewport with labels on all four sides and a formal z-index layering system. After struggling with grid gutter attachment, reverted and removed the grid overlay UI entirely.' },
        { name: 'MISC_CONFIG_AND_DOCS', commits: 7, summary: 'Added runtime tuning knobs, documented environment configuration, and logged sanitized startup config. Updated README, ran cargo fmt and clippy, fixed minor frontend selection bugs.' },
        { name: 'DASHBOARD_AND_BOARD_MANAGEMENT', commits: 5, summary: 'Added a board dashboard page with album grid layout displaying board names. Extracted a BoardCard component, fixed board list reload issues, and maintained full presence state from join/part broadcasts.' },
        { name: 'WEBSOCKET_ERROR_HANDLING', commits: 3, summary: 'Added error logging for failed WebSocket frames on both server and client sides. Centralized the outbound WebSocket frame path to reduce duplication.' }
      ]
    },
    {
      day: 3, date: '2026-02-18', title: 'LEPTOS CLIENT FULL INTEGRATION', commits: 84,
      fieldNote: 'Biggest day of the sprint. Replaced React/Konva with Leptos 0.8 SSR across eight sequential phases. Built the canvas engine with hit-testing (99 tests), input state machine (55 tests), and full Canvas2D rendering. Added multiplayer presence, placement tools, and frame grouping.',
      clusters: [
        { name: 'LEPTOS_CLIENT_PHASES', commits: 12, summary: 'Built the Leptos 0.8 + Axum SSR client across eight sequential phases: scaffold, SSR integration, pages/auth/REST, WebSocket frame client, toolbar/statusbar, left panel, right panel, and dark mode polish.' },
        { name: 'CANVAS_ENGINE_CORE', commits: 11, summary: 'Implemented hit-testing with 99 geometry tests, the input state machine with 55 edge-case tests, and the render/draw pipeline with full Canvas2D output. Fixed resize accumulation and text action bugs.' },
        { name: 'CLIENT_UI_RESTRUCTURE', commits: 10, summary: 'Rewrote the client stylesheet to a React-token-based UI system. Wired WebSocket board join/list/create flows, normalized frame parsing, polished toolbar and dashboard interactions.' },
        { name: 'CANVAS_BROWSER_INTEGRATION', commits: 8, summary: 'Mounted the canvas engine in the browser and wired pointer, wheel, and keyboard events. Propagated canvas actions to WebSocket mutation frames, centered the world origin, and fixed rotated resize handle drift.' },
        { name: 'SERVER_TESTS_AND_TOOLCHAIN', commits: 8, summary: 'Added 102 new server tests. Centralized Rust toolchain configuration, aligned rust-version across all crates and the Dockerfile, and resolved Docker build issues.' },
        { name: 'MULTIPLAYER_PRESENCE', commits: 7, summary: 'Implemented remote cursor rendering with server cursor frame support. Built adaptive drag interpolation, stale cursor expiry, and conflict guards for live move/resize/rotate broadcasting.' },
        { name: 'PLACEMENT_TOOLS_AND_SHAPES', commits: 7, summary: 'Replaced the tool flyout with click-to-place ghost preview workflow. Enabled circle, line, and arrow placement tools with shape attachment points and endpoint markers.' },
        { name: 'DESIGN_DOCS_AND_SCAFFOLDING', commits: 6, summary: 'Refreshed the README, drafted the konva-to-rust design doc with a public API boundary section. Added the canvas crate scaffold with 131 passing tests.' },
        { name: 'FRAMES_AND_POLISH', commits: 5, summary: 'Added frames grouping with persistent rail tooltips and savepoint/rewind timeline. Implemented grouped transform rotation for frame contents and shipped a YouTube TV embed as an easter egg.' }
      ]
    },
    {
      day: 4, date: '2026-02-19', title: 'OBSERVABILITY, AI & ROTATION', commits: 104,
      fieldNote: 'Highest commit count of the sprint. Built the traces crate for observability, migrated to protobuf wire format, implemented viewport rotation with compass widget, rebuilt AI integration with YAML grammar, and shipped auth, board sharing, and perf benchmarks.',
      clusters: [
        { name: 'OBSERVABILITY_AND_TRACING', commits: 14, summary: 'Added a traces crate with derivation helpers and client-side trace view UI. Linked AI tool calls and object frames into a prompt trace tree, emitted per-round LLM spans with metrics, and iterated heavily on the trace UI.' },
        { name: 'AI_TOOLS_AND_YAML_GRAMMAR', commits: 12, summary: 'Built AI assistant integration with markdown rendering, session-scoped context, and a strict quoted-YAML grammar for LLM I/O. Added an applyChangesYaml parser/executor and routed tool calls through a shared syscall dispatcher.' },
        { name: 'CANVAS_AND_UI_POLISH', commits: 11, summary: 'Added first-class text shapes across canvas, client, and AI tooling. Refined zoom wheel interaction, replaced status bar zoom menu with a dial control, and applied theme consistency across light and dark modes.' },
        { name: 'CANVAS_ROTATION_AND_COMPASS', commits: 9, summary: 'Implemented viewport-centered camera rotation math and rotated canvas rendering with a compass view control. Synced rotation through presence and follow mode, added compass snapping with a QA matrix.' },
        { name: 'BOARD_MANAGEMENT_AND_SHARING', commits: 9, summary: 'Added board snapshot mini-previews, hover-delete with confirm dialogs, and broadcast of board deletes. Implemented board member ACLs, management routes, and a 6-character access code sharing flow.' },
        { name: 'PRESENCE_AND_STATION_LOG', commits: 8, summary: 'Unified cursor and camera presence by client connection. Built the station log roster with sort order, self-row styling, and follow/jump controls. Removed old header presence list in favor of station log.' },
        { name: 'CRATE_MIGRATION_AND_PROTOBUF', commits: 7, summary: 'Restructured workspace from two crates to three (client/server/canvas), removed the legacy React build pipeline. Migrated WebSocket transport to a shared protobuf frames crate with wire protocol docs.' },
        { name: 'FRAME_PARSING_AND_PERSISTENCE', commits: 5, summary: 'Refactored client-side frame parsing with expanded test coverage. Fixed protobuf numeric decoding for board join with regression tests. Refactored board WebSocket ops into dedicated handlers.' },
        { name: 'AUTH_AND_LOGIN_FLOW', commits: 5, summary: 'Added email-code authentication using Resend delivery with a template. Fixed OAuth navigation, resolved the dashboard auth flash, and simplified login title styling.' },
        { name: 'PERF_BENCHMARKS', commits: 3, summary: 'Added a perf crate with end-to-end, algorithmic complexity, and mass-user benchmarks. Configured auth bypass for local perf runs and improved output with count-matrix rows.' }
      ]
    },
    {
      day: 5, date: '2026-02-20', title: 'DIALS, REFACTORS & POLISH', commits: 68,
      fieldNote: 'Focused on code quality and UX refinement. Extracted reusable dial controls, decomposed the monolithic canvas host and frame client into smaller modules, split CSS into themed layers, and added multi-select, minimap, board import/export, and broad client test coverage.',
      clusters: [
        { name: 'DIAL_CONTROL_SYSTEM', commits: 10, summary: 'Extracted reusable dial primitives, migrated compass and zoom controls to the new system. Added object-level rotation, color, and text style dials with snap-click routing and reset controls.' },
        { name: 'MULTI_SELECT_AND_UX_POLISH', commits: 9, summary: 'Implemented multi-select canvas interactions with persisted grouping and a consistent bullseye placement ghost. Added public board toggle, follow controls, status-bar help modal, and default object colors.' },
        { name: 'CANVAS_HOST_REFACTOR', commits: 7, summary: 'Broke the canvas host into smaller modules for dial math, object prop helpers, frame emission, selection metrics, and shape placement presets. Removed unused transform code.' },
        { name: 'FRAME_CLIENT_REFACTOR', commits: 6, summary: 'Decomposed the frame client into dedicated submodules for parsing helpers, AI handlers, error handling, and chat/object/request concerns. Fixed hydrate recursion from the reorganization.' },
        { name: 'CLIENT_TESTING_AND_CLEANUP', commits: 6, summary: 'Added broad client test coverage for util, pages, state, and net helpers. Deduplicated redirect logic, shared hex color normalization, and extracted a shared request frame builder.' },
        { name: 'CSS_AND_THEME_CLEANUP', commits: 5, summary: 'Split monolithic CSS into theme, base, layout, and component modules. Extracted shared side-panel primitives, inlined CSS imports to eliminate runtime 404s.' },
        { name: 'MINIMAP_AND_VIEWPORT', commits: 5, summary: 'Replaced the station log with a bare top-right minimap overlay with draggable viewport controls. Made minimap read-only after drag UX proved too fragile.' },
        { name: 'OBJECT_TEXT_AND_PROMPTS', commits: 5, summary: 'Added object text edit modal triggered by canvas double-click. Refactored board prompt parsing, extracted prompt bar into a dedicated component with preview flow.' },
        { name: 'IMPORT_EXPORT_AND_INFRA', commits: 5, summary: 'Added board JSONL export and import endpoints with toolbar actions, web-sys file input for imports. Included traces crate in Docker build and added requirements checklist to README.' }
      ]
    },
    {
      day: 6, date: '2026-02-21', title: 'PERFORMANCE, CLI & AI TOOLS', commits: 52,
      fieldNote: 'Tackled large-board rendering performance with spatial indexing and viewport culling. Stood up a CLI crate, cleaned up AI tool schemas, added SVG rendering, and ran a full correctness audit with doc coverage push.',
      clusters: [
        { name: 'CANVAS_PERFORMANCE', commits: 8, summary: 'Tackled large-board rendering end to end: disabled auto savepoints during bulk loads, added join/render timing metrics, gated scene sync on revision numbers, and coalesced redraws. Introduced spatial bucket indexing with viewport culling.' },
        { name: 'AI_TOOL_SCHEMA_CLEANUP', commits: 8, summary: 'Refactored server LLM config, centralized provider wiring, and aligned AI tool schemas with canonical UI object properties. Removed legacy batch operations, shape aliases, and youtube_embed.' },
        { name: 'HOUSEKEEPING_AND_STATS', commits: 7, summary: 'Ran a correctness audit fixing clippy warnings and panic-capable code across the workspace. Added missing doc comments to all public items. Built project stats and code coverage scripts for the README.' },
        { name: 'TOOLBAR_AND_PROFILE_UI', commits: 7, summary: 'Added responsive toolbar toggles and a user profile modal with clipboard support. Iterated on the toolbar user area design across several refinement passes.' },
        { name: 'SVG_AI_TOOLS_AND_VIEWPORT', commits: 5, summary: 'Defined Phase 1 SVG AI tool schemas and implemented their execution path, plumbing svg objects through to the canvas renderer from inline path markup. Added streaming of ai:prompt updates and live viewport geometry in the AI system prompt.' },
        { name: 'CLI_AND_STRESS_TOOLING', commits: 4, summary: 'Stood up a CLI crate with clap subcommands covering REST board CRUD and WebSocket JSONL object streaming. Added a stress JSONL generator with a spiral pattern mode.' },
        { name: 'CANVAS_INPUT_FIXES', commits: 4, summary: 'Fixed a tool-switch pan jump bug, moved default viewport origin to top-left. Added a hand (pan) tool and preserved camera state across canvas host re-initialization.' },
        { name: 'TESTING_HARDENING', commits: 3, summary: 'Extracted pure server logic into testable functions. Added exhaustive edge-case tests across canvas, frames, and traces crates, then fixed compile errors surfaced by the surgeon pass.' }
      ]
    },
    {
      day: 7, date: '2026-02-22', title: 'TRACING & PORTFOLIO SITE', commits: 9,
      fieldNote: 'Final day. Promoted trace to a top-level frame field, built the portfolio website with vintage field-survey aesthetic, then iterated on design spec compliance and navigation. Replaced the timeline placeholder carousel with a commit-log view driven by actual git history.',
      clusters: [
        { name: 'FRAME_TRACE_PROMOTION', commits: 3, summary: 'Promoted the trace field from a nested data property to a top-level frame field with backend gating. Auto-enabled trace config before board list requests and removed legacy data.trace read fallbacks.' },
        { name: 'PORTFOLIO_POLISH_AND_NAV', commits: 3, summary: 'Closed visual gaps between the portfolio implementation and design spec \u2014 font sizes, carousel arrows, dot grid opacity, transitions, and missing CSS variables. Added hash-based URL routing so browser back/forward works.' },
        { name: 'PORTFOLIO_SITE_LAUNCH', commits: 2, summary: 'Built a vintage "Field Survey Terminal" portfolio site in plain HTML/CSS/JS and mounted it at the root path, relocating the Leptos SSR app to /app. Fixed routing issues and aligned live demo links.' },
        { name: 'TIMELINE_COMMIT_LOG', commits: 1, summary: 'Replaced the placeholder screenshot carousel with a vertically scrolling commit log panel showing themed clusters for each build day, driven by actual git history analysis.' }
      ]
    }
  ];

  /* --- Timeline Rendering --- */
  var tlPanel = document.getElementById('timeline-log-panel');

  if (tlPanel) {
    var tlTitle = document.getElementById('tl-title');
    var tlDate = document.getElementById('tl-date');
    var tlCommits = document.getElementById('tl-commits');
    var tlBody = document.getElementById('tl-body');
    var tlPagination = document.getElementById('tl-pagination');
    var tlFieldNotes = document.getElementById('tl-field-notes');
    var tlPrev = document.getElementById('tl-prev');
    var tlNext = document.getElementById('tl-next');
    var tlCurrent = 0;

    function renderTimelineDay(index) {
      var day = timelineDays[index];
      tlCurrent = index;

      /* Update header */
      tlTitle.textContent = 'DAY ' + day.day + ' \u2014 ' + day.title;
      tlDate.textContent = day.date;
      tlCommits.textContent = day.commits + ' COMMITS';
      tlPagination.textContent = 'DAY ' + day.day + ' OF 7';

      /* Render commit log (left panel) — most recent cluster first */
      var clusters = day.clusters.slice().reverse();
      if (clusters.length === 0) {
        tlBody.innerHTML = '<div class="tl-log-empty">No commits on this day.<br>Planning, research, and pre-search documentation only.</div>';
      } else {
        var html = '';
        for (var i = 0; i < clusters.length; i++) {
          var c = clusters[i];
          html += '<div class="tl-log-entry">'
            + '<div class="tl-log-marker"></div>'
            + '<div class="tl-log-content">'
            + '<div class="tl-log-name">' + c.name.replace(/_/g, '_') + '</div>'
            + '<div class="tl-log-commits">' + c.commits + ' COMMITS</div>'
            + '<div class="tl-log-summary">' + c.summary + '</div>'
            + '</div></div>';
        }
        tlBody.innerHTML = html;
      }
      tlBody.scrollTop = 0;

      /* Render field notes (right panel) */
      var notesHtml = '<div class="section-heading">FIELD_NOTES &mdash; DAY ' + day.day + '</div>'
        + '<p class="body-text">' + day.fieldNote + '</p>';
      if (clusters.length > 0) {
        notesHtml += '<div class="section-heading" style="margin-top:24px">ACTIVITY_SUMMARY</div>';
        /* Show clusters in original (chronological) order for the field notes */
        for (var j = 0; j < day.clusters.length; j++) {
          var cl = day.clusters[j];
          notesHtml += '<div class="timeline-entry">'
            + '<div class="inventory-header">'
            + '<span class="inventory-name">' + cl.name.replace(/_/g, '_') + '</span>'
            + '<span class="inventory-meta">' + cl.commits + ' COMMITS</span>'
            + '</div>'
            + '<p class="inventory-desc">' + cl.summary + '</p>'
            + '</div>';
        }
      }
      tlFieldNotes.innerHTML = notesHtml;
    }

    /* Initialize */
    renderTimelineDay(0);

    tlPrev.addEventListener('click', function () {
      renderTimelineDay((tlCurrent - 1 + timelineDays.length) % timelineDays.length);
    });

    tlNext.addEventListener('click', function () {
      renderTimelineDay((tlCurrent + 1) % timelineDays.length);
    });

    /* Keyboard navigation */
    document.addEventListener('keydown', function (e) {
      var timelinePage = document.querySelector('.page[data-page="timeline"]');
      if (!timelinePage || !timelinePage.classList.contains('active')) return;

      if (e.key === 'ArrowLeft') {
        tlPrev.click();
      } else if (e.key === 'ArrowRight') {
        tlNext.click();
      }
    });
  }
})();
