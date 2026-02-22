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
    if (validPages.indexOf(pageName) === -1) pageName = 'architecture';
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
    if (hash === 'stack' || hash === 'overview' || hash === 'demo') hash = 'architecture';
    return hash || 'architecture';
  }

  window.addEventListener('popstate', function () {
    switchPage(getPageFromHash(), false);
  });

  /* Load initial page from URL hash */
  var initialPage = getPageFromHash();
  if (initialPage !== 'architecture') {
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
  var REPO = 'https://github.com/ianzepp/gauntlet-week-1/commit/';

  var timelineDays = [
    {
      day: 1, date: '2026-02-16', title: 'FULL-STACK SCAFFOLD', commits: 40,
      fieldNote: 'Day one was about getting a working vertical slice as fast as possible. Started with the pre-search doc \u2014 budget, auth strategy, architecture outline, testing plan \u2014 then scaffolded the Rust/Axum backend and React/Konva frontend in the same afternoon. By evening, GitHub OAuth was working, WebSocket real-time sync was broadcasting cursor positions, and the AI tool loop was calling out to Claude with 9 tools. The persistence layer went through three iterations: first a buffered flush at 1-second intervals, then tightened to 100ms, then ripped out entirely in favor of direct DB writes per frame. Also wired up user field report popovers, structured logging, Docker Compose, and a static file server with SPA fallback. Ended the day with 40 commits and a system that actually worked end-to-end, even if the canvas was rough.',
      clusters: [
        { name: 'PRE_SEARCH_AND_DESIGN_DOCS', commits: 6, summary: 'Drafted and iterated on the CollabBoard pre-search document covering budget, auth, architecture, and testing strategy. Added the project brief PDF, design system spec, and organized all docs into a docs/ directory.',
          git: [
            {h:'cf05398',t:'13:56',m:'Add CollabBoard Pre-Search document'},
            {h:'3bf294a',t:'14:14',m:'Update Pre-Search after review: budget, auth, architecture, testing'},
            {h:'afb6ff3',t:'14:26',m:'Resolve PRE-SEARCH gaps: auth, sync, persistence, reconnect, and open questions'},
            {h:'9b8dc14',t:'14:29',m:'Add CollabBoard project brief PDF'},
            {h:'3e83cba',t:'16:30',m:'Add design system spec and move project docs to docs/'},
            {h:'dbe3ffb',t:'16:31',m:'Move PRE-SEARCH.MD to docs/'}
          ]},
        { name: 'BACKEND_AND_FRONTEND_SCAFFOLD', commits: 5, summary: 'Stood up the Rust/Axum backend scaffold and the React/Vite/Konva frontend scaffold. Moved backend into server/, added the AI agent loop and WebSocket dispatch entry points.',
          git: [
            {h:'f9a61b9',t:'16:34',m:'Add Rust + Axum backend scaffold for CollabBoard'},
            {h:'8adcfc1',t:'16:35',m:'Add .gitignore for Rust project'},
            {h:'7de815d',t:'16:57',m:'Add React + Vite + Konva frontend scaffold for CollabBoard'},
            {h:'107d8ee',t:'16:59',m:'Add README and move G4 spec to docs/'},
            {h:'ccfce3e',t:'17:44',m:'Move backend to server/, add AI agent loop and WS dispatch'}
          ]},
        { name: 'UI_LAYOUT_AND_FRONTEND', commits: 5, summary: 'Redesigned the UI with a left tool rail, tabbed right panel, and board stamp. Added the AI chat panel as a collapsible sidebar, placeholder drawing tools, and Transformer-based selection/resize.',
          git: [
            {h:'aa3b1c4',t:'18:17',m:'Add Transformer-based selection/resize and UI polish'},
            {h:'9cdd9da',t:'19:37',m:'Add frontend AI chat panel with collapsible sidebar (#26)'},
            {h:'da5f568',t:'20:35',m:'Redesign UI layout: left tool rail, tabbed right panel, board stamp'},
            {h:'62850ba',t:'20:41',m:'Add placeholder tools to left rail: line, arrow, text, draw, eraser'},
            {h:'1137120',t:'21:54',m:'Fix canvas object selection, resize, and transform bugs'}
          ]},
        { name: 'AI_SERVICE_AND_TOOLING', commits: 4, summary: 'Built the LLM multi-provider adapter with rate limiting and prompt injection defense. Replaced the consolidated tool set with 9 spec-matching tools.',
          git: [
            {h:'4bcebc7',t:'18:22',m:'Add rate limiting and prompt injection defense to AI service (#12)'},
            {h:'537a594',t:'18:28',m:'Add LLM multi-provider adapter, object service, and tool definitions (#12)'},
            {h:'a874cf0',t:'18:57',m:'Replace 7 consolidated AI tools with 9 spec-matching tools (#28)'},
            {h:'8f7e2eb',t:'19:02',m:'Merge branch \'issue-28\''}
          ]},
        { name: 'AUTH_AND_REALTIME_SYNC', commits: 4, summary: 'Implemented GitHub OAuth authentication with env var configuration. Wired up WebSocket real-time sync with cursor presence broadcasting across connected clients.',
          git: [
            {h:'363c343',t:'18:59',m:'Add GitHub OAuth authentication for collaboard'},
            {h:'67783d3',t:'19:01',m:'Add GitHub OAuth env vars to .env.example'},
            {h:'e33dec8',t:'19:30',m:'Wire up WebSocket real-time sync and cursor presence (#17)'},
            {h:'c29759b',t:'21:39',m:'Skip logging and persistence for cursor frames'}
          ]},
        { name: 'PERSISTENCE_LAYER', commits: 6, summary: 'Added buffered frame persistence to the frames table, then tightened the flush interval from 1s to 100ms and switched to sleep-after-flush. Later removed the buffer entirely in favor of direct DB persistence on each inbound frame.',
          git: [
            {h:'bc643c3',t:'19:49',m:'Add buffered frame persistence to frames table'},
            {h:'7313a7b',t:'19:52',m:'Tighten persistence flush interval from 1s to 100ms'},
            {h:'2de3864',t:'19:53',m:'Switch persistence loop from interval to sleep-after-flush'},
            {h:'72fd769',t:'21:01',m:'Remove frame buffer: persist frames directly to DB on send'},
            {h:'f1ebab5',t:'21:05',m:'Accept full Frame from client, persist inbound frames to DB'},
            {h:'b9c7def',t:'21:36',m:'Fix frame deserialization: demo board ID must be valid UUID'}
          ]},
        { name: 'USER_STATS_AND_PROFILES', commits: 3, summary: 'Added user field report popovers on status bar chips showing per-user activity. Fixed profile stats to stamp user_id on frames and query both in-memory boards and legacy frame data.',
          git: [
            {h:'ad8e3be',t:'20:48',m:'Add user field report popover on status bar user chips'},
            {h:'cec205b',t:'20:53',m:'Fix user field report: stamp user_id on frames, query board_objects'},
            {h:'2d47c3f',t:'20:56',m:'Fix profile stats: query in-memory boards and legacy frame data'}
          ]},
        { name: 'BUG_FIXES_AND_INFRA', commits: 7, summary: 'Fixed a stream of integration issues: Frame deserialization with null handling and valid UUIDs, AI "stuck on thinking" from unfiltered blocks, and canvas object selection/transform bugs. Added dotenvy for .env loading, structured logging, static file serving with SPA fallback, and a docker-compose file.',
          git: [
            {h:'f5036fc',t:'18:32',m:'Fix .gitignore to exclude server/target/ build artifacts'},
            {h:'1840a01',t:'20:05',m:'Load .env file at startup via dotenvy'},
            {h:'81f3cb9',t:'20:09',m:'Serve frontend static files from Axum with SPA fallback'},
            {h:'e42cda3',t:'21:25',m:'Fix Frame deserialization: use null instead of empty strings'},
            {h:'43b43a1',t:'21:29',m:'Fix AI stuck on "thinking": filter thinking blocks, guarantee item frame'},
            {h:'282d916',t:'21:32',m:'Add structured logging for frame and AI prompt debugging'},
            {h:'87cb6e1',t:'21:48',m:'Add docker-compose and update Cargo.lock'}
          ]}
      ]
    },
    {
      day: 2, date: '2026-02-17', title: 'CANVAS REBUILD & PANEL LAYOUT', commits: 76,
      fieldNote: 'Woke up, looked at the canvas code, and decided to gut it entirely. The previous day\u2019s implementation had too many layered hacks. Replaced it from scratch with a full-viewport rendering layer: grid overlay, pan/zoom, coordinate display. Rebuilt rectangle creation, selectable dragging, hit-testing, and inline text editing one piece at a time. In parallel, redesigned the right panel \u2014 replaced the toggle with an always-visible collapsed icon rail, then extracted it into tabbed sections for Boards, Chat, AI, and Inspector. Added a board dashboard with album grid layout, real-time chat messaging, markdown rendering in AI responses, and scoped AI history to the authenticated user. Also shipped the deployment pipeline (Docker Compose, Fly.io prep, then removed Fly), renamed the project to gauntlet-week-1, and isolated tests from the live database. 76 commits \u2014 the canvas rebuild alone was 10.',
      clusters: [
        { name: 'WEBSOCKET_ERROR_HANDLING', commits: 3, summary: 'Added error logging for failed WebSocket frames on both server and client sides. Centralized the outbound WebSocket frame path to reduce duplication.',
          git: [
            {h:'57f3c23',t:'05:46',m:'Log error code and message for failed WebSocket frames'},
            {h:'e9f09c2',t:'05:47',m:'Log error frames in FrameClient dispatch'},
            {h:'3a4ed86',t:'06:13',m:'Centralize WebSocket frame outbound path'}
          ]},
        { name: 'SERVER_TEST_INFRASTRUCTURE', commits: 7, summary: 'Moved server tests into dedicated *_test.rs files per project convention. Added integration suites for WebSocket AI flows, multi-user sync, board service syscalls, and chat/history.',
          git: [
            {h:'d9e7e1a',t:'06:27',m:'Add websocket AI integration tests for multi-tool flows'},
            {h:'27b46f1',t:'06:30',m:'Move server tests into dedicated *_test.rs files'},
            {h:'ce176db',t:'06:38',m:'Add multi-user websocket board synchronization tests'},
            {h:'0844caf',t:'07:21',m:'Add board service and ws board syscall tests'},
            {h:'432ff9f',t:'07:44',m:'Add websocket tests for chat message and list syscalls'},
            {h:'fd1807f',t:'07:45',m:'Trim leading blank lines in server test modules'},
            {h:'c6a8e04',t:'08:37',m:'Harden auth, board access, and WS reconnect behavior'}
          ]},
        { name: 'DASHBOARD_AND_BOARD_MANAGEMENT', commits: 5, summary: 'Added a board dashboard page with album grid layout displaying board names. Extracted a BoardCard component, fixed board list reload issues, and maintained full presence state from join/part broadcasts.',
          git: [
            {h:'c216b45',t:'06:40',m:'Add board dashboard page with album grid layout'},
            {h:'1c553bb',t:'06:48',m:'Maintain full presence list from board:join and board:part broadcasts'},
            {h:'e8f4a51',t:'06:51',m:'Display board names instead of UUIDs in toolbar, stamp, and status bar'},
            {h:'c5a1efb',t:'06:55',m:'Extract BoardCard component and add Mission Control board switcher'},
            {h:'43082fa',t:'07:14',m:'Fix board list not loading on dashboard reload'}
          ]},
        { name: 'RIGHT_PANEL_REDESIGN', commits: 12, summary: 'Replaced the right panel toggle with an always-visible collapsed icon rail, then iterated heavily \u2014 extracting tabs for Boards, Chat, AI, and Inspector. Added real-time chat, board switcher, and open/close chevron.',
          git: [
            {h:'3e20ec7',t:'06:29',m:'Replace rightPanelOpen with always-visible collapsed icon rail'},
            {h:'3dc037e',t:'07:09',m:'Refactor RightPanel: separate always-visible rail from expandable column'},
            {h:'8ba73f3',t:'07:15',m:'Fix MissionControl board list and remove stale "item" status'},
            {h:'1191ae4',t:'07:21',m:'Move MissionControl into RightPanel as a Boards tab'},
            {h:'47f14eb',t:'07:31',m:'Replace right panel tab row with static title header'},
            {h:'7dff38c',t:'07:32',m:'Add separator and toggle area to right rail'},
            {h:'f5663e3',t:'07:47',m:'Add Chat tab to RightPanel with real-time messaging'},
            {h:'6845d4a',t:'08:05',m:'Reorder right rail tabs: boards, chat, ai, inspector'},
            {h:'5740d22',t:'08:10',m:'Add open/close chevron icon to bottom of right rail'},
            {h:'dcbfabd',t:'08:15',m:'Move inspector from right panel to new left panel'},
            {h:'2b3deca',t:'18:04',m:'Update board UI controls and fix AI history duplication'}
          ]},
        { name: 'AI_AND_CHAT_FEATURES', commits: 8, summary: 'Added chat:history and ai:history syscalls, feeding recent conversation into LLM context scoped to the authenticated user. Rendered markdown in AI responses and fixed tool mutations to use current object versions.',
          git: [
            {h:'6243f0d',t:'07:52',m:'Use monospace font for AI responses and input in Field Notes'},
            {h:'d7aab4b',t:'07:56',m:'Rename chat:list to chat:history and add ai:history'},
            {h:'5802d0e',t:'07:58',m:'Feed recent AI conversation history into LLM context'},
            {h:'ef26d82',t:'08:02',m:'Clear AI and chat messages on board switch and reload history'},
            {h:'112de0e',t:'08:04',m:'Remove green border and reduce font size on AI assistant responses'},
            {h:'5b02d41',t:'08:20',m:'Fix AI tool updates to use current object version'},
            {h:'426d1f5',t:'08:23',m:'Scope AI history and context to authenticated user'},
            {h:'7b17768',t:'08:24',m:'Add empty state messages to Field Notes and Chat panels'},
            {h:'e291f65',t:'08:27',m:'Render markdown in AI assistant responses'}
          ]},
        { name: 'GRID_AND_VIEWPORT_LAYOUT', commits: 7, summary: 'Added Battleship-style grid coordinates to the viewport with labels on all four sides and a formal z-index layering system. After struggling with grid gutter attachment, reverted and removed the grid overlay UI entirely.',
          git: [
            {h:'13323fd',t:'07:29',m:'Center viewport so canvas (0,0) appears at screen center'},
            {h:'34c060e',t:'07:41',m:'Add Battleship-style grid coordinates to viewport and AI'},
            {h:'4981631',t:'08:19',m:'Wrap grid coordinate labels on all four sides of viewport'},
            {h:'9ed8d31',t:'08:33',m:'Fix grid overlay visibility: add z-index and opaque background'},
            {h:'dab7bc7',t:'08:36',m:'Add formal z-index layering system via CSS custom properties'},
            {h:'74a032b',t:'08:43',m:'Attach grid gutters to rails by extending over them'},
            {h:'904b6e9',t:'08:44',m:'Revert "Attach grid gutters to rails by extending over them"'}
          ]},
        { name: 'INSPECTOR_AND_VISUAL_POLISH', commits: 9, summary: 'Added inspector controls for font size and border width, moved presence chips to the top bar, merged the tool rail into a unified left panel. Added confirmed delete with keyboard shortcut and tuned selection ring animation.',
          git: [
            {h:'ad49d82',t:'08:08',m:'Move presence chips to top bar (left-aligned) and logged-in user to status bar'},
            {h:'2646878',t:'08:30',m:'Merge ToolRail into LeftPanel as a single unified rail'},
            {h:'fef3366',t:'08:31',m:'Fix left panel order: inspector panel, rail, then canvas'},
            {h:'9f1b2c4',t:'15:20',m:'Reduce left inspector panel width'},
            {h:'74a0a53',t:'15:25',m:'Fix tool strip anchoring to rail edge'},
            {h:'843e01b',t:'15:46',m:'Add inspector controls for font size and border width'},
            {h:'410d54e',t:'15:58',m:'Add confirmed delete action and keyboard delete shortcut'},
            {h:'e8ee298',t:'15:58',m:'Style inspector delete button'},
            {h:'984d65f',t:'18:17',m:'Refine selection ring animation and lighten canvas grid'},
            {h:'f99ac4a',t:'18:30',m:'Tune selection ring pulse and rebalance canvas grid contrast'}
          ]},
        { name: 'DEPLOYMENT_AND_DEVOPS', commits: 8, summary: 'Added run-dev.sh and switched to Docker Compose. Prepared Fly.io deployment, then removed it. Renamed project to gauntlet-week-1 and isolated tests from the live database.',
          git: [
            {h:'bc95d8a',t:'06:16',m:'Add run-dev.sh script'},
            {h:'4285f6f',t:'08:44',m:'Prepare Fly deployment image and release workflow'},
            {h:'233f161',t:'09:07',m:'Switch local dev to Docker Compose workflow'},
            {h:'22a3936',t:'11:29',m:'Update Fly config for ORD region and runtime env vars'},
            {h:'2441393',t:'12:08',m:'Rename collaboard to gauntlet-week-1 and isolate tests from live DB'},
            {h:'b59a057',t:'16:18',m:'Run DB migrations before server start in Dockerfile'},
            {h:'7d1bd06',t:'16:38',m:'Remove Fly deployment config'}
          ]},
        { name: 'MISC_CONFIG_AND_DOCS', commits: 7, summary: 'Added runtime tuning knobs, documented environment configuration, and logged sanitized startup config. Updated README, ran cargo fmt and clippy, fixed minor frontend selection bugs.',
          git: [
            {h:'c8fd88d',t:'06:14',m:'Fix selection, delete, ellipse transform, and formatting'},
            {h:'28bbef3',t:'06:59',m:'Add runtime tuning knobs and document env configuration'},
            {h:'42d3736',t:'07:45',m:'Log sanitized startup environment configuration'},
            {h:'7cf98ae',t:'08:50',m:'Remove battleship grid overlay UI, keep server-side grid utilities'},
            {h:'6e72a21',t:'09:05',m:'Update README to reflect current architecture and feature status'},
            {h:'2ab79b4',t:'11:41',m:'Delete .env.example'},
            {h:'f62bc10',t:'16:00',m:'Run rust fmt and fix clippy warnings'}
          ]},
        { name: 'CANVAS_GUTTING_AND_REBUILD', commits: 10, summary: 'Gutted the existing canvas and replaced it with a full-viewport layer: grid overlay, pan/zoom, coordinate display. Rebuilt rectangle creation, selectable dragging, hit-testing, inline text editing, and sticky notes from scratch.',
          git: [
            {h:'def568b',t:'12:14',m:'Gut canvas implementation, replace with placeholder'},
            {h:'d5ea2f9',t:'12:42',m:'Rebuild canvas as full-viewport layer with grid, pan/zoom, and coordinates'},
            {h:'b8fa7ac',t:'13:04',m:'Add rectangle tool strip and fix object creation dimensions'},
            {h:'4073947',t:'14:20',m:'Fix client/server object payload reconciliation'},
            {h:'bd6419f',t:'14:36',m:'Add selectable rectangle dragging on canvas'},
            {h:'bb27e7c',t:'14:45',m:'Stabilize canvas hit-testing and object drag behavior'},
            {h:'0ffef95',t:'15:15',m:'Add inline object text editing and centered handwriting display'},
            {h:'471809d',t:'15:37',m:'Add multi-user cursor tracking and rendering'},
            {h:'dc300cc',t:'15:53',m:'Add sticky note objects with title support'},
            {h:'9bd9b0f',t:'16:37',m:'Fix AI mutation broadcast status for frontend sync'}
          ]}
      ]
    },
    {
      day: 3, date: '2026-02-18', title: 'LEPTOS CLIENT FULL INTEGRATION', commits: 84,
      fieldNote: 'The biggest day of the sprint \u2014 84 commits. The morning started with the canvas engine crate: hit-testing with 99 geometry tests, an input state machine with 55 edge-case tests, and the full Canvas2D render pipeline. Then the big move: replacing React/Konva with Leptos 0.8 SSR across eight sequential phases. Scaffold, SSR integration, pages and auth, WebSocket frame client, toolbar and status bar, left panel, right panel, dark mode polish. Each phase landed as its own commit. By afternoon the new Rust client was rendering in the browser. Spent the evening on multiplayer: remote cursor rendering, adaptive drag interpolation, stale cursor expiry, and conflict guards for live move/resize/rotate. Then placement tools (click-to-place ghost preview, circles, lines, arrows with attachment points), frame grouping with savepoint/rewind timeline, and a YouTube TV embed as an easter egg. Also added 102 new server tests, centralized the Rust toolchain, and fought Docker build issues for hours.',
      clusters: [
        { name: 'DESIGN_DOCS_AND_SCAFFOLDING', commits: 6, summary: 'Refreshed the README, drafted the konva-to-rust design doc with a public API boundary section. Added the canvas crate scaffold with 131 passing tests.',
          git: [
            {h:'bf3a468',t:'07:16',m:'Refresh README to match current implementation'},
            {h:'5fb580b',t:'07:58',m:'Added very rough konva->rust idea doc'},
            {h:'234e43f',t:'08:12',m:'Revise konva-rust design doc after review'},
            {h:'5b17b34',t:'08:18',m:'Add canvas crate public API boundary to design doc'},
            {h:'f76a2df',t:'08:38',m:'Add canvas crate scaffolding with exhaustive tests (131 passing)'},
            {h:'e53bf58',t:'08:41',m:'Add hygiene tests and project ground rules'}
          ]},
        { name: 'CANVAS_ENGINE_CORE', commits: 11, summary: 'Implemented hit-testing with 99 geometry tests, the input state machine with 55 edge-case tests, and the render/draw pipeline with full Canvas2D output. Fixed resize accumulation and text action bugs.',
          git: [
            {h:'8801c4e',t:'08:49',m:'Implement set_viewport and full hit-testing with 99 geometry tests'},
            {h:'fa90536',t:'08:49',m:'Ratchet todo budget from 10 to 8 after hit-test implementation'},
            {h:'4a1027a',t:'08:51',m:'Extract shared constants into canvas/src/consts.rs'},
            {h:'836cd13',t:'09:02',m:'Implement input state machine for canvas engine'},
            {h:'d019a9f',t:'09:11',m:'Add 55 edge-case hardening tests for input state machine'},
            {h:'2618304',t:'09:18',m:'Fix resize accumulation bug: use total delta from start point'},
            {h:'0e3845b',t:'09:22',m:'Update design doc with implementation status and deviations'},
            {h:'d436d98',t:'09:29',m:'Add React-to-Leptos UI migration plan'},
            {h:'3640642',t:'09:49',m:'Fix resize/text action semantics and add regressions'},
            {h:'711de45',t:'09:50',m:'Silence test-only clippy float and copy-clone lints'},
            {h:'d2703b1',t:'10:02',m:'Add doc comments to all public items in canvas crate'},
            {h:'4ef3ad3',t:'10:15',m:'Implement render::draw() and Engine::render() with full Canvas2D pipeline'}
          ]},
        { name: 'LEPTOS_CLIENT_PHASES', commits: 12, summary: 'Built the Leptos 0.8 + Axum SSR client across eight sequential phases: scaffold, SSR integration, pages/auth/REST, WebSocket frame client, toolbar/statusbar, left panel, right panel, and dark mode polish.',
          git: [
            {h:'e06a095',t:'10:17',m:'Add client-rust crate scaffold (Phase 1)'},
            {h:'6415b58',t:'10:27',m:'Add tests for client-rust types and state modules'},
            {h:'38e93ff',t:'10:28',m:'Update react-to-leptos.md with Phase 1 scaffold status'},
            {h:'03e0936',t:'11:14',m:'Add Leptos 0.8 + Axum SSR integration (Phase 2)'},
            {h:'6d3078c',t:'11:16',m:'Create .gitkeep'},
            {h:'0b084b1',t:'11:16',m:'Update doc_test.rs'},
            {h:'ffb17b2',t:'11:18',m:'Implement pages, auth flow, and REST API client (Phase 3)'},
            {h:'cba1a47',t:'11:20',m:'Implement WebSocket frame client with reconnect and dispatch (Phase 4)'},
            {h:'b781d50',t:'11:23',m:'Implement Toolbar, StatusBar, and UserFieldReport components (Phase 5)'},
            {h:'5ae4a17',t:'11:25',m:'Implement left panel with tool rail, strip, and inspector (Phase 6)'},
            {h:'3a388e0',t:'11:27',m:'Implement right panel with chat, AI, and board switcher (Phase 7)'},
            {h:'eda362a',t:'11:29',m:'Add dark mode, board stamp, canvas host, and styling polish (Phase 8)'},
            {h:'af203ea',t:'11:34',m:'Merge branch \'leptos-ui\' into main'}
          ]},
        { name: 'SERVER_TESTS_AND_TOOLCHAIN', commits: 8, summary: 'Added 102 new server tests. Centralized Rust toolchain configuration, aligned rust-version across all crates and the Dockerfile, and resolved Docker build issues.',
          git: [
            {h:'5048cdf',t:'10:45',m:'Add 102 new server tests across 5 new test files and 3 expanded files'},
            {h:'e789a8c',t:'11:06',m:'Centralize Rust toolchain and harden tests against real backends'},
            {h:'863f9f7',t:'11:37',m:'Fix duplicate [features] table in server/Cargo.toml'},
            {h:'52fade9',t:'11:45',m:'Add dual-frontend Docker build and dual-port server'},
            {h:'93426f8',t:'11:48',m:'Bump Docker Rust image to 1.90 for cargo-leptos compatibility'},
            {h:'31ae79a',t:'11:50',m:'Align rust-version to 1.89 across all crates and Dockerfile'},
            {h:'2058523',t:'11:50',m:'Bump rust-version to 1.90 across all crates and Dockerfile'},
            {h:'decb7a6',t:'11:55',m:'Add perl and make to Docker build for openssl-sys compilation'},
            {h:'c76fb75',t:'12:34',m:'Moved PLAN.md'},
            {h:'dd12b45',t:'12:41',m:'Fix Docker build: use pre-built cargo-leptos binary and fix WASM compilation errors'}
          ]},
        { name: 'CLIENT_UI_RESTRUCTURE', commits: 10, summary: 'Rewrote the client stylesheet to a React-token-based UI system. Wired WebSocket board join/list/create flows, normalized frame parsing, polished toolbar and dashboard interactions.',
          git: [
            {h:'ea0cf60',t:'12:43',m:'Add missing public/ assets directory for cargo-leptos build'},
            {h:'0a6e44e',t:'13:23',m:'Rewrite client-rust stylesheet to React token-based UI system'},
            {h:'ba4b40f',t:'13:31',m:'Restructure left panel and implement tool rail, strip, and inspector'},
            {h:'54b528f',t:'13:35',m:'Restructure right panel and wire chat/ai history flows'},
            {h:'17f1a0e',t:'13:36',m:'Fix hydrate build for tool strip anchor and clean warning'},
            {h:'63b37e9',t:'13:45',m:'Send board join on mount and websocket reconnect'},
            {h:'8a40da8',t:'13:52',m:'Use websocket board:list and board:create in dashboard'},
            {h:'34c4d3f',t:'13:52',m:'Run single Leptos server port and align Rust toolchain'},
            {h:'64b2bed',t:'13:53',m:'Apply rustfmt normalization in left panel component files'},
            {h:'619f8a2',t:'13:53',m:'Update client-rust plan status and remaining tasks'},
            {h:'ac1cb60',t:'13:56',m:'Use untracked board read in websocket join helper'},
            {h:'94a803d',t:'14:00',m:'Polish toolbar/status UI and improve dashboard/login interactions'},
            {h:'5622a91',t:'14:02',m:'Normalize websocket chat/ai payload parsing in frame client'},
            {h:'0864a9b',t:'14:11',m:'Align frame ts to i64 and remove item status from protocol enums'},
            {h:'74f82a6',t:'14:17',m:'Route board list/create through shared frame client state'},
            {h:'ab8d0a0',t:'15:15',m:'Surface websocket board errors and normalize frame error parsing'},
            {h:'3f9992c',t:'15:17',m:'Update client-rust plan status after phase 3-8 completion'},
            {h:'fca1cc4',t:'15:25',m:'Refactor frame dispatch and add targeted client-rust unit coverage'}
          ]},
        { name: 'CANVAS_BROWSER_INTEGRATION', commits: 8, summary: 'Mounted the canvas engine in the browser and wired pointer, wheel, and keyboard events. Propagated canvas actions to WebSocket mutation frames, centered the world origin, and fixed rotated resize handle drift.',
          git: [
            {h:'59142bd',t:'15:34',m:'Fix board chrome ordering and viewport-safe panel grid sizing'},
            {h:'14ba1f3',t:'15:39',m:'Integrate canvas engine mount and state-to-canvas snapshot sync'},
            {h:'f07d7e9',t:'15:48',m:'Wire canvas pointer and wheel events for pan/zoom interaction'},
            {h:'0587ecc',t:'15:51',m:'Propagate canvas actions to websocket object mutation frames'},
            {h:'02e20bd',t:'15:54',m:'Center canvas world origin at viewport center on mount'},
            {h:'6ff07e9',t:'16:21',m:'Add canvas keyboard handling and safe selection sync'},
            {h:'ab09071',t:'17:02',m:'Fix rotated resize handle drift by resizing in local space'},
            {h:'9b51c2f',t:'17:02',m:'Add canvas telemetry state for live status bar updates'}
          ]},
        { name: 'MULTIPLAYER_PRESENCE', commits: 7, summary: 'Implemented remote cursor rendering with server cursor frame support. Built adaptive drag interpolation, stale cursor expiry, and conflict guards for live move/resize/rotate broadcasting.',
          git: [
            {h:'f83e653',t:'19:09',m:'Render remote cursors and support server cursor frame shape'},
            {h:'08c0a21',t:'19:18',m:'Add ephemeral object:drag sync for live move/resize/rotate'},
            {h:'098bb29',t:'19:25',m:'Add drag lifecycle end/timeout handling and conflict guards'},
            {h:'6273d44',t:'19:28',m:'Make drag interpolation adaptive to frame timing'},
            {h:'25e01df',t:'19:31',m:'Add cursor clear and stale cursor expiry lifecycle'},
            {h:'67ac134',t:'19:43',m:'Broadcast cursor updates during ghost placement mode'},
            {h:'e480f9a',t:'19:47',m:'Open inspector on object double-click'}
          ]},
        { name: 'PLACEMENT_TOOLS_AND_SHAPES', commits: 7, summary: 'Replaced the tool flyout with click-to-place ghost preview workflow. Enabled circle, line, and arrow placement tools with shape attachment points and endpoint markers.',
          git: [
            {h:'5854f98',t:'19:41',m:'Replace tool flyout with click-to-place ghost preview'},
            {h:'625e53c',t:'19:48',m:'Enable circle line and arrow placement tools'},
            {h:'bf025f9',t:'19:51',m:'Avoid duplicate local object on create'},
            {h:'d125333',t:'20:11',m:'Add shape attachment points for line and arrow endpoints'},
            {h:'138ba10',t:'20:17',m:'Render attached endpoint markers in normal edge view'}
          ]},
        { name: 'FRAMES_AND_POLISH', commits: 5, summary: 'Added frames grouping with persistent rail tooltips and savepoint/rewind timeline. Implemented grouped transform rotation for frame contents and shipped a YouTube TV embed as an easter egg.',
          git: [
            {h:'8aa97f4',t:'20:38',m:'Astley: add YouTube TV object and non-blocking video overlay'},
            {h:'a61bef2',t:'20:52',m:'Add frames grouping and persistent rail tooltips'},
            {h:'73ec4a8',t:'21:28',m:'Add savepoints and record-shelf rewind timeline'},
            {h:'9803783',t:'21:36',m:'Rotate frame contents as grouped transform'},
            {h:'fe48dfa',t:'21:53',m:'Unify canvas background grid rendering'}
          ]}
      ]
    },
    {
      day: 4, date: '2026-02-19', title: 'OBSERVABILITY, AI & ROTATION', commits: 104,
      fieldNote: 'Highest commit count of the sprint: 104. The day split across five major fronts. First, migrated the entire WebSocket transport from JSON to a shared protobuf frames crate with binary encoding and wire protocol docs. Second, built the traces crate for observability \u2014 derivation helpers, client-side trace view UI, linked AI tool calls into prompt trace trees, per-round LLM spans with timing metrics. Third, implemented camera rotation with a compass view control, snapping behavior, and a QA matrix. Fourth, rebuilt the AI integration: session-scoped context memory, strict quoted-YAML grammar for LLM I/O, an applyChangesYaml parser, and routing all tool calls through a shared syscall dispatcher. Fifth, shipped real features: email-code authentication via Resend, board member ACLs with management routes, 6-character access code sharing, board snapshot mini-previews, hover-delete with confirm dialogs, and the perf crate with end-to-end benchmarks. Also added first-class text shapes, replaced the zoom menu with a dial control, and themed the UI for light/dark consistency.',
      clusters: [
        { name: 'PRESENCE_AND_STATION_LOG', commits: 8, summary: 'Unified cursor and camera presence by client connection. Built the station log roster with sort order, self-row styling, and follow/jump controls. Removed old header presence list in favor of station log.',
          git: [
            {h:'9e443c7',t:'08:36',m:'Unify board presence by client connection and add station log roster'},
            {h:'54861e2',t:'08:44',m:'Sort station log with current connection first'},
            {h:'ac91f50',t:'08:56',m:'Add shared viewport follow/jump and harden join/telemetry flow'},
            {h:'bf3d890',t:'09:12',m:'Unify cursor+camera presence and improve station log controls'},
            {h:'cb80c25',t:'09:20',m:'Polish camera lock UX and include board name on join'},
            {h:'178efa8',t:'09:24',m:'Refine status bar lock indicator and remove user chip'},
            {h:'56a7ea2',t:'11:41',m:'Adjust station log self row and toolbar identity label'},
            {h:'b8cbd8e',t:'11:42',m:'Shift station log left when right panel is expanded'}
          ]},
        { name: 'FRAME_PARSING_AND_PERSISTENCE', commits: 5, summary: 'Refactored client-side frame parsing with expanded test coverage. Fixed protobuf numeric decoding for board join with regression tests. Refactored board WebSocket ops into dedicated handlers.',
          git: [
            {h:'f0b42c7',t:'09:29',m:'Refactor client-rust frame parsing and expand test coverage'},
            {h:'761968d',t:'09:50',m:'Fix board membership transitions and harden persistence/token correctness'},
            {h:'f86020f',t:'11:28',m:'Fix protobuf numeric decoding for board join and add regression tests'},
            {h:'87aa126',t:'12:04',m:'Stream board join objects via item frames and preserve client identity'},
            {h:'b24a93d',t:'12:12',m:'Broadcast board part immediately on leave and board switch'}
          ]},
        { name: 'CRATE_MIGRATION_AND_PROTOBUF', commits: 7, summary: 'Restructured workspace from two crates to three (client/server/canvas), removed the legacy React build pipeline. Migrated WebSocket transport to a shared protobuf frames crate with wire protocol docs.',
          git: [
            {h:'61d5318',t:'10:01',m:'Apply technical-writer documentation rubric across non-test Rust modules'},
            {h:'c93b824',t:'10:16',m:'Migrate to 3-crate layout: client/server/canvas and remove legacy React build'},
            {h:'734843c',t:'10:17',m:'Rewrite README for current Rust-only client/server/canvas architecture'},
            {h:'8e0776e',t:'10:43',m:'Migrate WS frame transport to shared protobuf frames crate'},
            {h:'53339bc',t:'10:44',m:'Document frames wire protocol and clean WS wording'},
            {h:'569f5bf',t:'10:47',m:'Harden frames codec docs and edge-case tests'},
            {h:'1467fcc',t:'12:15',m:'Refactor board websocket ops into dedicated handlers'},
            {h:'ab2c68f',t:'15:39',m:'Fix Docker workspace copies for cargo leptos build'},
            {h:'4f703fb',t:'15:47',m:'Rewrite README with dedicated crate sections and project narrative'},
            {h:'8721984',t:'15:49',m:'Add total test count (681) to README testing section'},
            {h:'a280658',t:'16:25',m:'Update env example for Leptos static paths'}
          ]},
        { name: 'PERF_BENCHMARKS', commits: 3, summary: 'Added a perf crate with end-to-end, algorithmic complexity, and mass-user benchmarks. Configured auth bypass for local perf runs and improved output with count-matrix rows.',
          git: [
            {h:'412f88e',t:'10:53',m:'Add perf crate with e2e, complexity, and mass-user benchmarks'},
            {h:'65ad403',t:'11:09',m:'Add perf auth bypass and fix local server startup config'},
            {h:'7249952',t:'11:20',m:'Improve perf output with count-matrix rows and JSON summaries'}
          ]},
        { name: 'CANVAS_ROTATION_AND_COMPASS', commits: 9, summary: 'Implemented viewport-centered camera rotation math and rotated canvas rendering with a compass view control. Synced rotation through presence and follow mode, added compass snapping with a QA matrix.',
          git: [
            {h:'a3aa375',t:'12:26',m:'Document canvas rotation compass UI behavior'},
            {h:'5da2a1c',t:'12:36',m:'Add viewport-centered camera view rotation math'},
            {h:'2f796c2',t:'12:43',m:'Add rotated canvas rendering and compass view control'},
            {h:'54495e4',t:'12:44',m:'Moving docs around'},
            {h:'f75d2be',t:'12:49',m:'Sync camera rotation in presence and follow mode'},
            {h:'939aa6a',t:'12:51',m:'Polish compass snapping and add rotation QA matrix'},
            {h:'99be8e9',t:'12:53',m:'Document rotation features and fix canvas hygiene violations'},
            {h:'4cc8e60',t:'12:57',m:'Enable web-sys DOM rect APIs for compass pointer math'},
            {h:'2dc37ec',t:'13:05',m:'Fix compass visibility and remove duplicate canvas telemetry'},
            {h:'dbee54b',t:'13:06',m:'Emit presence updates during compass rotation interactions'},
            {h:'0aaa370',t:'13:07',m:'Fix reused compass pointer-up handler closure'}
          ]},
        { name: 'BOARD_MANAGEMENT_AND_SHARING', commits: 9, summary: 'Added board snapshot mini-previews, hover-delete with confirm dialogs, and broadcast of board deletes. Implemented board member ACLs, management routes, and a 6-character access code sharing flow.',
          git: [
            {h:'ddaca25',t:'13:28',m:'Add board snapshot mini-previews with resilient list parsing'},
            {h:'9753280',t:'13:37',m:'Add hover delete action and confirm dialog for dashboard boards'},
            {h:'4774e92',t:'13:38',m:'Broadcast board deletes and eject clients from deleted boards'},
            {h:'880c7b7',t:'13:43',m:'Broadcast board list refresh events to all websocket clients'},
            {h:'440bbb3',t:'13:44',m:'Refactor board frame handling into thinner op dispatcher'},
            {h:'51078f8',t:'13:46',m:'Add tests for board list refresh fanout and snapshot parsing'},
            {h:'084204b',t:'13:54',m:'Add board:list noop via revision tokens'},
            {h:'068056c',t:'13:59',m:'Poll dashboard board list every 10s using since_rev'},
            {h:'cfb2187',t:'17:32',m:'Add board_members ACL with member management routes'},
            {h:'150c819',t:'18:39',m:'Add board sharing via 6-character access codes'},
            {h:'6dfc24f',t:'22:40',m:'Unify dashboard actions and add board input overlay'}
          ]},
        { name: 'CANVAS_AND_UI_POLISH', commits: 11, summary: 'Added first-class text shapes across canvas, client, and AI tooling. Refined zoom wheel interaction, replaced status bar zoom menu with a dial control, and applied theme consistency across light and dark modes.',
          git: [
            {h:'d313396',t:'08:28',m:'Move Field Records into right panel and sync workspace changes'},
            {h:'d997394',t:'11:47',m:'Add home viewport reset and unify rail tooltip behavior'},
            {h:'5932d4d',t:'15:53',m:'Apply formatting cleanup'},
            {h:'fb8780b',t:'16:38',m:'Fix board overlay edge spacing against rails'},
            {h:'664ca62',t:'17:41',m:'Stabilize board overlay positioning across rails and viewport'},
            {h:'daeebe6',t:'18:12',m:'Remove canvas grid-dot background rendering'},
            {h:'a00b1a9',t:'18:56',m:'Fix board view color mapping fallback for circle/shape props'},
            {h:'9206810',t:'20:08',m:'Refine canvas rendering and UI state handling'},
            {h:'dbb0816',t:'20:59',m:'Reorder board header and add segmented mode controls'},
            {h:'0e7e147',t:'21:57',m:'Toolbar: move share beside view toggle with spacing'},
            {h:'1edddac',t:'22:03',m:'Revert floating inspector experiment'},
            {h:'7f80555',t:'22:10',m:'Theme: make nav chrome consistent across light/dark'},
            {h:'2d5e4a6',t:'22:12',m:'Theme: use earthy highlight accents in light mode'},
            {h:'260de5a',t:'23:27',m:'Add first-class text shape across canvas, client, and AI tooling'},
            {h:'757902f',t:'23:54',m:'Wrap canvas text rendering and anchor prompt bar to canvas'},
            {h:'a2dc33b',t:'00:19',m:'Refine zoom wheel interaction and center-reset behavior'},
            {h:'167ec53',t:'00:22',m:'Remove status bar zoom menu in favor of dial control'}
          ]},
        { name: 'AUTH_AND_LOGIN_FLOW', commits: 5, summary: 'Added email-code authentication using Resend delivery with a template. Fixed OAuth navigation, resolved the dashboard auth flash, and simplified login title styling.',
          git: [
            {h:'64d9654',t:'15:53',m:'Update login page headings'},
            {h:'48e6260',t:'16:04',m:'Fix login OAuth navigation and dashboard auth flash'},
            {h:'472afd4',t:'16:21',m:'Simplify login title and remove unused style'},
            {h:'4acef11',t:'17:09',m:'Add email code auth with Resend delivery and template'},
            {h:'17f4afb',t:'17:24',m:'Show dynamic auth method in UI headers'}
          ]},
        { name: 'OBSERVABILITY_AND_TRACING', commits: 14, summary: 'Added a traces crate with derivation helpers and client-side trace view UI. Linked AI tool calls and object frames into a prompt trace tree, emitted per-round LLM spans with metrics, and iterated heavily on the trace UI.',
          git: [
            {h:'d3c5f1c',t:'12:46',m:'Create collabboard-observability-design.md'},
            {h:'bad0148',t:'16:51',m:'Add traces crate with derivation helpers and coverage'},
            {h:'b2f58af',t:'16:53',m:'Document observability implementation status'},
            {h:'b2c7b75',t:'20:22',m:'Add observability trace view to client'},
            {h:'14e2b8c',t:'20:37',m:'Link AI tool and object frames into prompt trace tree'},
            {h:'9bdb0db',t:'20:40',m:'Adapt trace UI semantics for tool syscalls and tree depth'},
            {h:'fc552eb',t:'20:43',m:'Emit per-round ai:llm_request trace spans with metrics'},
            {h:'671d713',t:'20:45',m:'Add trace envelope metadata across AI spans and mutations'},
            {h:'f69604c',t:'20:49',m:'Populate trace labels for AI, tool, and mutation frames'},
            {h:'f519c21',t:'20:51',m:'Drop legacy trace fallbacks and require trace envelope'},
            {h:'d9fffee',t:'20:54',m:'Enforce trace envelope on ai prompt responses and add span persistence test'},
            {h:'52f17b8',t:'21:18',m:'Trace UI fixes'},
            {h:'4f1702b',t:'21:35',m:'Trace UI: improve index signal and reduce board:list noise'},
            {h:'bcb22f9',t:'21:39',m:'Trace UI: filter empty sessions and stop users-list polling'},
            {h:'7041dc6',t:'21:42',m:'Trace UI: simplify context panel and allow unselecting session'},
            {h:'a5bcc28',t:'21:50',m:'Trace mode: hide left rail'},
            {h:'289646a',t:'21:54',m:'Right panel: close trace tab when leaving trace mode'},
            {h:'40fe768',t:'22:17',m:'Trace log: add sortable column headers'}
          ]},
        { name: 'AI_TOOLS_AND_YAML_GRAMMAR', commits: 12, summary: 'Built AI assistant integration with markdown rendering, session-scoped context, and a strict quoted-YAML grammar for LLM I/O. Added an applyChangesYaml parser/executor and routed tool calls through a shared syscall dispatcher.',
          git: [
            {h:'6da8b56',t:'17:53',m:'Refine AI transcript rendering and prompt reconciliation'},
            {h:'a3b77f3',t:'17:59',m:'Render AI assistant messages as markdown'},
            {h:'2963b2e',t:'18:04',m:'Extract AI system prompt to markdown and refine layout guidance'},
            {h:'8c95c95',t:'18:10',m:'Add parallel batch tool for LLM orchestration'},
            {h:'d96ca3c',t:'18:07',m:'Remove header presence list in favor of station log'},
            {h:'6c2d35d',t:'18:58',m:'Reset AI prompt context per refresh and trim tool-call carryover'},
            {h:'c671fa7',t:'19:13',m:'Revise AI style tools and session-scoped context memory'},
            {h:'e58534c',t:'19:29',m:'Define strict quoted YAML shape grammar for LLM I/O'},
            {h:'8927d13',t:'19:33',m:'Add applyChangesYaml parser and executor for AI mutations'},
            {h:'bdbae01',t:'19:49',m:'Temporarily force YAML-only AI tool mode via toggle'},
            {h:'4f4c73b',t:'20:09',m:'Route AI tool calls through tool syscall frames'},
            {h:'75d5bbd',t:'20:11',m:'Add shared tool syscall dispatcher for AI and WS'},
            {h:'d53815f',t:'20:29',m:'Minor formatting cleanup in server ws route and tool syscall'},
            {h:'ae0765c',t:'22:49',m:'Wire board prompt bar to AI with inline request status'},
            {h:'7079b32',t:'23:12',m:'Align board prompt status icon to input row'}
          ]}
      ]
    },
    {
      day: 5, date: '2026-02-20', title: 'DIALS, REFACTORS & POLISH', commits: 68,
      fieldNote: 'Shifted focus from features to quality. The monolithic canvas host file was getting unwieldy, so I decomposed it into submodules: dial math, object prop helpers, frame emission, selection metrics, shape placement presets, viewport and presence helpers, input mapping. Did the same for the frame client \u2014 parsing helpers, AI handlers, error handling, chat/object/request concerns all got their own files. Split the CSS into theme, base, layout, and component modules. Extracted reusable dial primitives and migrated the compass and zoom controls, then built new dials for object rotation, color (with base picker and lightness rotation), and text style. Added a minimap overlay with draggable viewport controls (later made read-only after the drag UX proved too fragile). Shipped multi-select canvas interactions with persisted grouping, board JSONL import/export with web-sys file input, and broad client test coverage across util, pages, state, and net helpers. 68 commits, mostly refactoring \u2014 the kind of day that doesn\u2019t look dramatic but makes everything after it faster.',
      clusters: [
        { name: 'CSS_AND_THEME_CLEANUP', commits: 5, summary: 'Split monolithic CSS into theme, base, layout, and component modules. Extracted shared side-panel primitives, inlined CSS imports to eliminate runtime 404s.',
          git: [
            {h:'2956389',t:'07:20',m:'Split CSS into theme, base, layout, and component modules'},
            {h:'076cca5',t:'07:21',m:'Extract shared side-panel CSS primitives'},
            {h:'d1b7b6d',t:'07:25',m:'Inline CSS imports to avoid runtime 404 stylesheets'},
            {h:'566c0bb',t:'11:10',m:'Collapse unused split CSS duplicates into stubs'},
            {h:'845206e',t:'21:56',m:'Reduce cursor backlog and exclude cursor events from trace'}
          ]},
        { name: 'OBJECT_TEXT_AND_PROMPTS', commits: 5, summary: 'Added object text edit modal triggered by canvas double-click. Refactored board prompt parsing, extracted prompt bar into a dedicated component with preview flow.',
          git: [
            {h:'858b7da',t:'07:12',m:'Add text content fallback for canvas rendering'},
            {h:'a88c814',t:'07:37',m:'Refine prompt log icons, spacing, and neutral text colors'},
            {h:'2f2c4e6',t:'07:48',m:'Refine prompt preview flow and typography with inline read-more'},
            {h:'ce87d24',t:'09:40',m:'Add object text edit modal on canvas double-click'},
            {h:'d8b640e',t:'09:44',m:'Close object text modal when selection clears'},
            {h:'85ca0b2',t:'11:47',m:'Refactor board page prompt parsing and object text dialog'},
            {h:'a90450a',t:'11:49',m:'Extract board prompt bar into dedicated page component'}
          ]},
        { name: 'DIAL_CONTROL_SYSTEM', commits: 10, summary: 'Extracted reusable dial primitives, migrated compass and zoom controls to the new system. Added object-level rotation, color, and text style dials with snap-click routing and reset controls.',
          git: [
            {h:'00643f7',t:'07:25',m:'Add click-to-reset center hitbox for compass rotation'},
            {h:'17fabb7',t:'07:58',m:'Extract reusable dial primitives and migrate compass/zoom'},
            {h:'59d4eb8',t:'07:59',m:'Stack right-side dials with compass below zoom'},
            {h:'6e23910',t:'08:07',m:'Add always-visible object rotation dial and fix dial snap click routing'},
            {h:'ee56e95',t:'08:28',m:'Align object zoom dial with scale model and resize baseline reset'},
            {h:'816bd5f',t:'08:46',m:'Unify board and object controls on shared compass and zoom dials'},
            {h:'c8e10ce',t:'08:56',m:'Add object ColorDial with base-color picker and lightness rotation'},
            {h:'4e5d145',t:'09:29',m:'Set default border width to 0px and add dial reset controls'},
            {h:'9746ba9',t:'09:53',m:'Add text style dial and adaptive text color fallback'},
            {h:'c25b3fc',t:'16:29',m:'Refine dial markers and add zoom reset controls'},
            {h:'578281b',t:'20:05',m:'Refine dial handles to circular edge-centered knobs'}
          ]},
        { name: 'MINIMAP_AND_VIEWPORT', commits: 5, summary: 'Replaced the station log with a bare top-right minimap overlay with draggable viewport controls. Made minimap read-only after drag UX proved too fragile.',
          git: [
            {h:'b1687d6',t:'10:08',m:'Replace station log with bare top-right minimap overlay'},
            {h:'e0f72bc',t:'10:12',m:'Temporarily disable inspector panel and left rail expand control'},
            {h:'448e55f',t:'10:46',m:'Add draggable minimap viewport controls'},
            {h:'4b451b5',t:'17:16',m:'Fix unauth redirect and minimap drag capture'},
            {h:'a23b571',t:'17:18',m:'Stabilize minimap drag with pointer snapshot mapping'},
            {h:'3ea3cc6',t:'17:21',m:'Fix minimap click-vs-drag interaction'},
            {h:'f20f021',t:'21:23',m:'Make minimap read-only and non-interactive'}
          ]},
        { name: 'CLIENT_TESTING_AND_CLEANUP', commits: 6, summary: 'Added broad client test coverage for util, pages, state, and net helpers. Deduplicated redirect logic, shared hex color normalization, and extracted a shared request frame builder.',
          git: [
            {h:'eec383f',t:'10:57',m:'Refactor dashboard view into smaller components'},
            {h:'0875b9d',t:'11:08',m:'Extract shared request frame builder for client syscalls'},
            {h:'f2c09ad',t:'11:09',m:'Deduplicate unauthenticated redirect logic'},
            {h:'27b4819',t:'11:10',m:'Share hex color normalization across canvas and inspector'},
            {h:'38968ba',t:'11:35',m:'Fix board test import for FrameStatus'},
            {h:'f5af209',t:'18:00',m:'Add broad client test coverage for util, pages, state, and net helpers'},
            {h:'a3d227c',t:'19:39',m:'Refactor client helpers for testability and expand unit coverage'},
            {h:'bec4f84',t:'21:26',m:'Fix all-features test compatibility for validation runs'}
          ]},
        { name: 'FRAME_CLIENT_REFACTOR', commits: 6, summary: 'Decomposed the frame client into dedicated submodules for parsing helpers, AI handlers, error handling, and chat/object/request concerns. Fixed hydrate recursion from the reorganization.',
          git: [
            {h:'445c20b',t:'11:53',m:'Extract frame client parsing helpers into submodule'},
            {h:'d391b92',t:'11:56',m:'Extract frame client AI handlers into submodule'},
            {h:'a790ac1',t:'11:56',m:'Extract frame client error handling into submodule'},
            {h:'595351e',t:'12:04',m:'Refactor frame client into chat/object/request modules'},
            {h:'ae1ae16',t:'15:29',m:'Apply rustfmt updates from refactor validation'},
            {h:'6cd98a3',t:'16:00',m:'Fix hydrate recursion and color normalization callsites'}
          ]},
        { name: 'IMPORT_EXPORT_AND_INFRA', commits: 5, summary: 'Added board JSONL export and import endpoints with toolbar actions, web-sys file input for imports. Included traces crate in Docker build and added requirements checklist to README.',
          git: [
            {h:'8def4ea',t:'10:49',m:'Include traces crate in Docker build context'},
            {h:'b8a16cd',t:'11:45',m:'Add requirements checklist and update README with deployment URL'},
            {h:'d0c511d',t:'11:52',m:'Rewrite README with accurate crate list, features, and env vars'},
            {h:'b86acbe',t:'16:36',m:'Add board JSONL export endpoint and toolbar action'},
            {h:'ed37d39',t:'16:45',m:'Add board JSONL import endpoint and toolbar import action'},
            {h:'0d53d2b',t:'16:48',m:'Enable web-sys file input features for import'}
          ]},
        { name: 'CANVAS_HOST_REFACTOR', commits: 7, summary: 'Broke the canvas host into smaller modules for dial math, object prop helpers, frame emission, selection metrics, and shape placement presets. Removed unused transform code.',
          git: [
            {h:'d8ce5be',t:'16:09',m:'Reduce board page view nesting for hydrate recursion'},
            {h:'bd57c9e',t:'17:04',m:'Refactor canvas host dial math and object prop helpers'},
            {h:'b56c3f2',t:'17:08',m:'Refactor canvas host object update frame emission'},
            {h:'a4aee72',t:'17:11',m:'Refactor canvas host selection representative metrics'},
            {h:'5663ca7',t:'17:12',m:'Refactor canvas host shape placement presets'},
            {h:'7e8a7d7',t:'17:31',m:'Refactor canvas viewport and presence helpers'},
            {h:'a9c0411',t:'17:44',m:'Refactor canvas selection actions into shared module'},
            {h:'abc0b66',t:'17:46',m:'Refactor canvas input mapping helpers'},
            {h:'571814a',t:'20:59',m:'Remove unused transform helper in canvas host'}
          ]},
        { name: 'MULTI_SELECT_AND_UX_POLISH', commits: 9, summary: 'Implemented multi-select canvas interactions with persisted grouping and a consistent bullseye placement ghost. Added public board toggle, follow controls, status-bar help modal, and default object colors.',
          git: [
            {h:'fe42c49',t:'07:15',m:'Add canvas-edge drag resize for left rail'},
            {h:'f39dd56',t:'07:31',m:'Remove board name from status bar'},
            {h:'4cd1a18',t:'16:30',m:'Use router navigation for toolbar back action'},
            {h:'61b0953',t:'20:30',m:'Add public board visibility toggle and refine login code UX'},
            {h:'e3fde8d',t:'20:34',m:'Use a consistent bullseye placement ghost for shape tools'},
            {h:'bf77e30',t:'20:58',m:'Implement multi-select canvas interactions with persisted grouping'},
            {h:'79cc1fc',t:'21:17',m:'Add users panel follow controls and status-bar help modal'},
            {h:'67bed4c',t:'21:53',m:'Set sticky notes yellow and circles blue by default'},
            {h:'ec19efd',t:'21:53',m:'Move Help control to left side of status bar'}
          ]}
      ]
    },
    {
      day: 6, date: '2026-02-21', title: 'PERFORMANCE, CLI & AI TOOLS', commits: 52,
      fieldNote: 'Performance day. Large boards (1,000+ objects from the stress generator) were sluggish, so I worked through the rendering pipeline end-to-end: disabled auto savepoints during bulk loads, gated scene sync on revision numbers, coalesced redraws with requestAnimationFrame, avoided large board clones in canvas host effects, and added spatial bucket indexing with viewport culling. Also fixed a tool-switch pan jump, moved the default viewport origin to top-left, and added a hand (pan) tool. Stood up the CLI crate with clap subcommands for REST board CRUD, WebSocket JSONL object streaming, and a spiral-pattern stress generator. Ran a full correctness audit: fixed all clippy warnings and panic-capable code across the workspace, added missing doc comments to every public item. Cleaned up AI tool schemas \u2014 removed legacy batch operations, shape aliases, and youtube_embed, then aligned tool definitions with canonical UI object properties. Added Phase 1 SVG AI tools and implemented their execution path, plumbing svg objects through to the canvas renderer from inline path markup. Also added streaming AI prompt updates as item frames and included live viewport geometry in the AI system prompt context. 52 commits.',
      clusters: [
        { name: 'HOUSEKEEPING_AND_STATS', commits: 7, summary: 'Ran a correctness audit fixing clippy warnings and panic-capable code across the workspace. Added missing doc comments to all public items. Built project stats and code coverage scripts for the README.',
          git: [
            {h:'e43e214',t:'08:06',m:'README and requirements update'},
            {h:'32c07d0',t:'08:40',m:'Fix correctness issues from housekeeping audit'},
            {h:'dfb2c24',t:'09:02',m:'Add tests and documentation from housekeeping audit'},
            {h:'c07e4f5',t:'09:08',m:'Add project stats scripts for mechanical metrics collection'},
            {h:'ed04f95',t:'09:09',m:'Add project stats tables to README'},
            {h:'aa1639d',t:'09:15',m:'Add code coverage stats script and README table'},
            {h:'08871c4',t:'11:41',m:'Rust code surgeon pass: fix all clippy warnings and panic-capable code'},
            {h:'1b9be6e',t:'11:48',m:'Add missing doc comments to public and pub(crate) items across workspace'},
            {h:'71a2a48',t:'15:27',m:'Update README stats tables with latest metrics'}
          ]},
        { name: 'TESTING_HARDENING', commits: 3, summary: 'Extracted pure server logic into testable functions. Added exhaustive edge-case tests across canvas, frames, and traces crates, then fixed compile errors surfaced by the surgeon pass.',
          git: [
            {h:'79ae5d0',t:'09:30',m:'Extract pure server logic for testing and add comprehensive unit tests'},
            {h:'9d75380',t:'09:39',m:'Add exhaustive edge-case tests across canvas, frames, and traces crates'},
            {h:'f6b6e64',t:'15:28',m:'Fix compile errors from surgeon pass in toolbar and canvas_host'}
          ]},
        { name: 'TOOLBAR_AND_PROFILE_UI', commits: 7, summary: 'Added responsive toolbar toggles and a user profile modal with clipboard support. Iterated on the toolbar user area design across several refinement passes.',
          git: [
            {h:'6159381',t:'15:44',m:'Add responsive toolbar toggles and user profile modal'},
            {h:'ff17a82',t:'15:45',m:'Enable web-sys Navigator and Clipboard features for profile modal'},
            {h:'6f01261',t:'15:47',m:'Fix clipboard() call \u2014 returns Clipboard directly, not Option'},
            {h:'f739d87',t:'15:52',m:'Redesign toolbar user area: name, (method) badge, info icon, logout'},
            {h:'5507955',t:'15:53',m:'Replace info unicode char with SVG circle-i icon, make button square'},
            {h:'648772e',t:'15:54',m:'Use standard .btn class on info button to match other toolbar buttons'},
            {h:'a88e1cb',t:'15:57',m:'Fix toolbar info button square centering'}
          ]},
        { name: 'CLI_AND_STRESS_TOOLING', commits: 4, summary: 'Stood up a CLI crate with clap subcommands covering REST board CRUD and WebSocket JSONL object streaming. Added a stress JSONL generator with a spiral pattern mode.',
          git: [
            {h:'38f1590',t:'16:05',m:'Add initial CLI crate with clap subcommands'},
            {h:'21640ca',t:'16:33',m:'Add REST board CRUD and WS JSONL object streaming'},
            {h:'cdaa5f0',t:'16:43',m:'Add stress JSONL generator and reduce WS frame log verbosity'},
            {h:'204f179',t:'21:02',m:'Add spiral stress pattern to JSONL generator'}
          ]},
        { name: 'CANVAS_PERFORMANCE', commits: 8, summary: 'Tackled large-board rendering end to end: disabled auto savepoints during bulk loads, added join/render timing metrics, gated scene sync on revision numbers, and coalesced redraws. Introduced spatial bucket indexing with viewport culling.',
          git: [
            {h:'b64983d',t:'16:59',m:'Improve bulk board load performance and disable auto savepoints'},
            {h:'8d25692',t:'17:08',m:'Add join/render timing metrics and optimize board load updates'},
            {h:'d4d66b7',t:'17:37',m:'Render selected objects in a dedicated second pass'},
            {h:'ffff884',t:'19:16',m:'Gate canvas scene sync on scene revision and reset board telemetry on route change'},
            {h:'a23403a',t:'19:19',m:'Avoid large board clones in canvas host effects and coalesce redraws with RAF'},
            {h:'eaab1e9',t:'19:54',m:'Add bulk streaming status for board join frames'},
            {h:'ebaad49',t:'19:56',m:'Add spatial buckets and viewport culling in canvas'},
            {h:'5adaab4',t:'19:56',m:'Disable minimap rendering for large boards'},
            {h:'3026c44',t:'20:11',m:'Update canvas_viewport.rs'}
          ]},
        { name: 'CANVAS_INPUT_FIXES', commits: 4, summary: 'Fixed a tool-switch pan jump bug, moved default viewport origin to top-left. Added a hand (pan) tool and preserved camera state across canvas host re-initialization.',
          git: [
            {h:'658c427',t:'19:32',m:'Fix tool-switch pan jump and move default viewport origin to top-left'},
            {h:'e42a8bd',t:'19:41',m:'Preserve camera state when canvas host re-initializes'},
            {h:'9a5eb7f',t:'19:56',m:'Add hand tool across UI and canvas input'},
            {h:'44d5878',t:'19:56',m:'Keep canvas host mounted when toggling trace view'}
          ]},
        { name: 'AI_TOOL_SCHEMA_CLEANUP', commits: 8, summary: 'Refactored server LLM config, centralized provider wiring, and aligned AI tool schemas with canonical UI object properties. Removed legacy batch operations, shape aliases, and youtube_embed.',
          git: [
            {h:'a50dccb',t:'21:10',m:'Refactor server LLM config and centralize provider wiring'},
            {h:'be83571',t:'21:12',m:'Restore standard LLM tool definitions and prompt guidance'},
            {h:'7c93fae',t:'21:16',m:'Remove batch and applyChangesYaml from AI tool path'},
            {h:'72c7095',t:'21:25',m:'Align AI tools with UI object properties and shape support'},
            {h:'1203a16',t:'21:35',m:'Enforce canonical prop schema with strokeWidth'},
            {h:'c0dc1cc',t:'22:02',m:'Remove youtube_embed shape and tool path'},
            {h:'4731736',t:'22:05',m:'Remove remaining shape alias fixtures'},
            {h:'2b52f07',t:'22:07',m:'Canonicalize presence payload to user_name and user_color'},
            {h:'f989ae2',t:'22:33',m:'Align AI tool schemas with canonical UI props'}
          ]},
        { name: 'SVG_AI_TOOLS_AND_VIEWPORT', commits: 5, summary: 'Defined Phase 1 SVG AI tool schemas and implemented their execution path, plumbing svg objects through to the canvas renderer from inline path markup. Added streaming of ai:prompt updates and live viewport geometry in the AI system prompt.',
          git: [
            {h:'6443d94',t:'22:46',m:'Add Phase 1 SVG AI tool definitions'},
            {h:'0bc3836',t:'22:52',m:'Implement SVG tool execution and svg object plumbing'},
            {h:'0c9b3e5',t:'23:18',m:'Render svg objects from inline path markup'},
            {h:'52278a7',t:'23:26',m:'Stream ai:prompt tool and text updates as item frames'},
            {h:'9ae8e33',t:'23:27',m:'Format svg render test helpers'},
            {h:'cec2741',t:'23:44',m:'Cache per-client viewport state on server and keep AI prompt compact'},
            {h:'6536110',t:'23:48',m:'Include live viewport geometry in AI system prompt context'}
          ]}
      ]
    },
    {
      day: 7, date: '2026-02-22', title: 'TRACING & PORTFOLIO SITE', commits: 9,
      fieldNote: 'Final day of the sprint. Morning started with a small but important cleanup: promoted the trace field from a nested data property to a top-level frame field with backend gating, which simplified the entire observability pipeline. Then built the portfolio website you\u2019re looking at now \u2014 a vintage "Field Survey Terminal" aesthetic in plain HTML, CSS, and vanilla JS. Mounted it at the root path and relocated the Leptos app to /app. Iterated on design spec compliance (font sizes, transitions, carousel arrows, CSS variables), added hash-based URL routing so browser back/forward works, and replaced the timeline placeholder carousel with this vertically scrolling commit-log view driven by actual git history analysis. 9 commits on the final day \u2014 the lightest of the sprint, but the one that ties everything together for presentation.',
      clusters: [
        { name: 'FRAME_TRACE_PROMOTION', commits: 3, summary: 'Promoted the trace field from a nested data property to a top-level frame field with backend gating. Auto-enabled trace config before board list requests and removed legacy data.trace read fallbacks.',
          git: [
            {h:'3eec4fc',t:'08:35',m:'Promote trace to top-level frame field with backend gating'},
            {h:'8a240d5',t:'08:38',m:'Auto-enable trace config before board list requests'},
            {h:'bd22d41',t:'08:40',m:'Remove legacy data.trace read fallbacks'},
            {h:'7c5de4a',t:'08:55',m:'Fix client Frame initializers after trace field addition'}
          ]},
        { name: 'PORTFOLIO_SITE_LAUNCH', commits: 2, summary: 'Built a vintage "Field Survey Terminal" portfolio site in plain HTML/CSS/JS and mounted it at the root path, relocating the Leptos SSR app to /app. Fixed routing issues and aligned live demo links.',
          git: [
            {h:'de9a937',t:'08:54',m:'Add public portfolio website at / and move Leptos app to /app'},
            {h:'0f17396',t:'09:05',m:'Fix /app migration routing and align live demo links'}
          ]},
        { name: 'PORTFOLIO_POLISH_AND_NAV', commits: 3, summary: 'Closed visual gaps between the portfolio implementation and design spec \u2014 font sizes, carousel arrows, dot grid opacity, transitions, and missing CSS variables. Added hash-based URL routing so browser back/forward works.',
          git: [
            {h:'744f0f8',t:'09:08',m:'Close style gaps between portfolio implementation and design spec'},
            {h:'9fda662',t:'09:09',m:'Add hash-based URL routing to portfolio navigation'},
            {h:'75721f8',t:'09:23',m:'Correct sprint dates to Mon Feb 16 - Sun Feb 22'}
          ]},
        { name: 'TIMELINE_COMMIT_LOG', commits: 1, summary: 'Replaced the placeholder screenshot carousel with a vertically scrolling commit log panel showing themed clusters for each build day, driven by actual git history analysis.',
          git: [
            {h:'10e617d',t:'09:20',m:'Replace timeline left panel with vertical scrolling commit log'}
          ]}
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

      /* Render commit log (left panel) — oldest cluster first */
      var clusters = day.clusters;
      if (clusters.length === 0) {
        tlBody.innerHTML = '<div class="tl-log-empty">No commits on this day.<br>Planning, research, and pre-search documentation only.</div>';
      } else {
        var html = '';
        for (var i = 0; i < clusters.length; i++) {
          var c = clusters[i];
          html += '<div class="tl-log-entry">'
            + '<div class="tl-log-marker"></div>'
            + '<div class="tl-log-content">'
            + '<div class="tl-log-name">' + c.name + '</div>'
            + '<div class="tl-log-commits">' + c.commits + ' COMMITS</div>'
            + '<div class="tl-log-summary">' + c.summary + '</div>';
          if (c.git && c.git.length > 0) {
            html += '<div class="tl-commit-list">';
            for (var j = 0; j < c.git.length; j++) {
              var g = c.git[j];
              html += '<div class="tl-commit">'
                + '<div class="tl-commit-dot"></div>'
                + '<a class="tl-commit-hash" href="' + REPO + g.h + '" target="_blank" rel="noopener">' + g.h + '</a>'
                + '<span class="tl-commit-time">' + g.t + '</span>'
                + '<span class="tl-commit-msg">' + g.m + '</span>'
                + '</div>';
            }
            html += '</div>';
          }
          html += '</div></div>';
        }
        tlBody.innerHTML = html;
      }
      tlBody.scrollTop = 0;

      /* Render field notes (right panel) */
      var notesHtml = '<div class="section-heading">FIELD_NOTES &mdash; DAY ' + day.day + '</div>'
        + '<p class="body-text">' + day.fieldNote + '</p>';
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
  /* --- Transcripts Page --- */
  var txPanel = document.getElementById('tx-log-panel');

  if (txPanel) {
    var txTitle = document.getElementById('tx-title');
    var txDate = document.getElementById('tx-date');
    var txCount = document.getElementById('tx-count');
    var txList = document.getElementById('tx-list');
    var txPagination = document.getElementById('tx-pagination');
    var txViewer = document.getElementById('tx-viewer');
    var txPrev = document.getElementById('tx-prev');
    var txNext = document.getElementById('tx-next');
    var txCurrent = 0;
    var txManifest = null;

    /* Day titles from timeline data */
    var txDayTitles = [];
    for (var d = 0; d < timelineDays.length; d++) {
      txDayTitles.push(timelineDays[d].title);
    }

    function renderTranscriptDay(index) {
      if (!txManifest) return;
      var dayData = txManifest.days[index];
      txCurrent = index;

      txTitle.textContent = 'DAY ' + dayData.day + ' \u2014 ' + (txDayTitles[index] || '');
      txDate.textContent = dayData.date;
      txCount.textContent = dayData.sessions.length + ' SESSIONS';
      txPagination.textContent = 'DAY ' + dayData.day + ' OF 7';

      /* Split sessions into interactive vs agent tasks */
      var interactive = [];
      var agentTasks = [];
      for (var i = 0; i < dayData.sessions.length; i++) {
        var s = dayData.sessions[i];
        var userCount = parseUserCount(s.messages);
        if (userCount === 1) {
          agentTasks.push(s);
        } else {
          interactive.push(s);
        }
      }

      var html = '';
      if (interactive.length > 0) {
        html += '<div class="tx-section-header">INTERACTIVE &mdash; ' + interactive.length + ' SESSIONS</div>';
        html += renderSessionList(interactive, dayData.day);
      }
      if (agentTasks.length > 0) {
        html += '<div class="tx-section-header">AGENT_TASKS &mdash; ' + agentTasks.length + ' SESSIONS</div>';
        html += renderSessionList(agentTasks, dayData.day);
      }
      txList.innerHTML = html;
      txList.scrollTop = 0;

      /* Attach click handlers */
      var entries = txList.querySelectorAll('.tx-session');
      entries.forEach(function (entry) {
        entry.addEventListener('click', function () {
          entries.forEach(function (e) { e.classList.remove('active'); });
          entry.classList.add('active');
          loadTranscript(entry.getAttribute('data-day'), entry.getAttribute('data-file'));
        });
      });

      /* Reset viewer */
      txViewer.innerHTML = '<div class="tx-empty-state">SELECT A SESSION FROM THE LOG</div>';
    }

    function parseUserCount(messages) {
      if (!messages) return 0;
      var match = messages.match(/^(\d+)\s+user/);
      return match ? parseInt(match[1], 10) : 0;
    }

    function renderSessionList(sessions, dayNum) {
      var html = '';
      for (var i = 0; i < sessions.length; i++) {
        var s = sessions[i];
        var estTime = formatTranscriptTime(s.started);
        var agentClass = s.agent === 'codex' ? ' codex' : '';
        var durationPill = s.duration ? '<span class="tx-session-duration">' + escapeHtml(s.duration) + '</span>' : '';
        var metaParts = [];
        if (s.model) metaParts.push(s.model);
        if (s.messages) metaParts.push(s.messages);

        html += '<div class="tx-session" data-day="' + dayNum + '" data-file="' + s.file + '">'
          + '<div class="tx-session-time">' + estTime + '</div>'
          + '<div class="tx-session-body">'
          + '<div class="tx-session-header"><span class="tx-session-agent' + agentClass + '">' + s.agent.toUpperCase() + '</span>' + durationPill + '</div>'
          + '<div class="tx-session-summary">' + escapeHtml(s.summary || '(no summary)') + '</div>'
          + '<div class="tx-session-meta">' + escapeHtml(metaParts.join(' \u00b7 ')) + '</div>'
          + '</div></div>';
      }
      return html;
    }

    function formatTranscriptTime(started) {
      if (!started) return '--:--';
      /* Parse UTC, convert to EST (UTC-5), show HH:MM */
      var parts = started.match(/(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/);
      if (!parts) return '--:--';
      var h = parseInt(parts[4], 10) - 5;
      var m = parts[5];
      if (h < 0) h += 24;
      var ampm = h >= 12 ? 'PM' : 'AM';
      var h12 = h % 12;
      if (h12 === 0) h12 = 12;
      return h12 + ':' + m + ' ' + ampm;
    }

    function loadTranscript(dayNum, filename) {
      var url = 'transcripts/day' + dayNum + '/' + filename;
      txViewer.innerHTML = '<div class="tx-empty-state">LOADING...</div>';

      fetch(url)
        .then(function (res) { return res.text(); })
        .then(function (text) {
          renderTranscript(text);
        })
        .catch(function () {
          txViewer.innerHTML = '<div class="tx-empty-state">FAILED TO LOAD TRANSCRIPT</div>';
        });
    }

    function renderTranscript(text) {
      var lines = text.split('\n');
      var metaLines = [];
      var bodyLines = [];
      var inMeta = true;

      for (var i = 0; i < lines.length; i++) {
        var line = lines[i];
        if (inMeta && line.indexOf('\ud83d\udccb') === 0) {
          metaLines.push(line);
        } else {
          inMeta = false;
          bodyLines.push(line);
        }
      }

      var html = '';

      /* Meta block */
      if (metaLines.length > 0) {
        html += '<div class="tx-meta-block">';
        for (var m = 0; m < metaLines.length; m++) {
          html += '<div>' + escapeHtml(metaLines[m]) + '</div>';
        }
        html += '</div>';
      }

      /* Body — group lines into blocks by leading emoji.
         A line without a leading emoji is a continuation of the previous block. */
      html += '<div class="tx-body">';
      var blocks = [];
      var currentBlock = null;

      for (var b = 0; b < bodyLines.length; b++) {
        var line = bodyLines[b];
        var cls = classifyLine(line);

        if (cls !== null) {
          /* New emoji-prefixed line starts a new block */
          if (currentBlock) blocks.push(currentBlock);
          currentBlock = { cls: cls, lines: [line] };
        } else if (currentBlock) {
          /* Continuation line — append to current block */
          currentBlock.lines.push(line);
        } else {
          /* Orphan line before any emoji — treat as assistant */
          currentBlock = { cls: 'tx-line-assistant', lines: [line] };
        }
      }
      if (currentBlock) blocks.push(currentBlock);

      for (var k = 0; k < blocks.length; k++) {
        var blk = blocks[k];
        /* Trim trailing blank lines */
        while (blk.lines.length > 0 && blk.lines[blk.lines.length - 1].trim() === '') {
          blk.lines.pop();
        }
        if (blk.lines.length === 0) continue;
        var raw = blk.lines.join('\n');
        if (blk.cls === 'tx-line-user' || blk.cls === 'tx-line-assistant') {
          html += '<div class="tx-line ' + blk.cls + '">' + renderMarkdown(raw) + '</div>';
        } else {
          html += '<div class="tx-line ' + blk.cls + '">' + escapeHtml(raw) + '</div>';
        }
      }
      html += '</div>';

      txViewer.innerHTML = html;
      txViewer.scrollTop = 0;
    }

    function classifyLine(line) {
      if (line.indexOf('\ud83d\udc64') === 0) return 'tx-line-user';       /* 👤 */
      if (line.indexOf('\ud83e\udd16') === 0) return 'tx-line-assistant';  /* 🤖 */
      if (line.indexOf('\u2705') === 0) return 'tx-line-tool';             /* ✅ */
      if (line.indexOf('\u274c') === 0) return 'tx-line-tool-fail';        /* ❌ */
      if (line.indexOf('\ud83d\udccb') === 0) return 'tx-line-meta';       /* 📋 */
      if (line.indexOf('---') === 0) return 'tx-line-summary';
      return null; /* continuation of previous block */
    }

    function renderMarkdown(text) {
      if (typeof marked !== 'undefined' && marked.parse) {
        try { return marked.parse(text, { breaks: true }); } catch (e) { /* fall through */ }
      }
      return escapeHtml(text);
    }

    function escapeHtml(str) {
      if (!str) return '';
      return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    /* Load manifest and init */
    fetch('transcripts/index.json')
      .then(function (res) { return res.json(); })
      .then(function (data) {
        txManifest = data;
        renderTranscriptDay(0);
      });

    txPrev.addEventListener('click', function () {
      if (!txManifest) return;
      renderTranscriptDay((txCurrent - 1 + txManifest.days.length) % txManifest.days.length);
    });

    txNext.addEventListener('click', function () {
      if (!txManifest) return;
      renderTranscriptDay((txCurrent + 1) % txManifest.days.length);
    });

    /* Keyboard navigation for transcripts page */
    document.addEventListener('keydown', function (e) {
      var txPage = document.querySelector('.page[data-page="transcripts"]');
      if (!txPage || !txPage.classList.contains('active')) return;

      if (e.key === 'ArrowLeft') {
        txPrev.click();
      } else if (e.key === 'ArrowRight') {
        txNext.click();
      }
    });
  }
  /* --- Visuals Page --- */
  var visPanel = document.getElementById('vis-panel');

  if (visPanel) {
    var visTitle = document.getElementById('vis-title');
    var visDate = document.getElementById('vis-date');
    var visCount = document.getElementById('vis-count');
    var visGallery = document.getElementById('vis-gallery');
    var visPagination = document.getElementById('vis-pagination');
    var visPrev = document.getElementById('vis-prev');
    var visNext = document.getElementById('vis-next');
    var visCurrent = 0;

    /* Media files organized by day (dates from filenames) */
    var visualDays = [
      { day: 2, date: '2026-02-17', files: [
        { name: 'screenshot-2026-02-17-0859.png', type: 'screenshot', time: '08:59' },
        { name: 'screenshot-2026-02-17-1527a.png', type: 'screenshot', time: '15:27' },
        { name: 'screenshot-2026-02-17-1527b.png', type: 'screenshot', time: '15:27' },
        { name: 'screenshot-2026-02-17-1527c.png', type: 'screenshot', time: '15:27' },
        { name: 'screenshot-2026-02-17-1527d.png', type: 'screenshot', time: '15:27' },
        { name: 'screenshot-2026-02-17-1640.png', type: 'screenshot', time: '16:40' }
      ]},
      { day: 3, date: '2026-02-18', files: [
        { name: 'screenshot-2026-02-18-1549.png', type: 'screenshot', time: '15:49' },
        { name: 'screenshot-2026-02-18-1605.png', type: 'screenshot', time: '16:05' },
        { name: 'screenshot-2026-02-18-1912.png', type: 'screenshot', time: '19:12' },
        { name: 'recording-2026-02-18-1913.mov', type: 'recording', time: '19:13' },
        { name: 'recording-2026-02-18-2000.mov', type: 'recording', time: '20:00' },
        { name: 'recording-2026-02-18-2033.mov', type: 'recording', time: '20:33' },
        { name: 'recording-2026-02-18-2036.mov', type: 'recording', time: '20:36' },
        { name: 'recording-2026-02-18-2134.mov', type: 'recording', time: '21:34' }
      ]},
      { day: 4, date: '2026-02-19', files: [
        { name: 'recording-2026-02-19-1403.mov', type: 'recording', time: '14:03' },
        { name: 'screenshot-2026-02-19-2033.png', type: 'screenshot', time: '20:33' },
        { name: 'screenshot-2026-02-19-2034.png', type: 'screenshot', time: '20:34' },
        { name: 'screenshot-2026-02-19-2233.png', type: 'screenshot', time: '22:33' },
        { name: 'screenshot-2026-02-19-2302.png', type: 'screenshot', time: '23:02' },
        { name: 'screenshot-2026-02-19-2344.png', type: 'screenshot', time: '23:44' },
        { name: 'recording-2026-02-19-2253.mov', type: 'recording', time: '22:53' }
      ]},
      { day: 5, date: '2026-02-20', files: [
        { name: 'screenshot-2026-02-20-0042.png', type: 'screenshot', time: '00:42' },
        { name: 'screenshot-2026-02-20-0043.png', type: 'screenshot', time: '00:43' },
        { name: 'screenshot-2026-02-20-0748.png', type: 'screenshot', time: '07:48' },
        { name: 'screenshot-2026-02-20-0927.png', type: 'screenshot', time: '09:27' },
        { name: 'screenshot-2026-02-20-1012.png', type: 'screenshot', time: '10:12' },
        { name: 'screenshot-2026-02-20-1143.png', type: 'screenshot', time: '11:43' },
        { name: 'recording-2026-02-20-1017.mov', type: 'recording', time: '10:17' },
        { name: 'recording-2026-02-20-1651.mov', type: 'recording', time: '16:51' }
      ]},
      { day: 6, date: '2026-02-21', files: [
        { name: 'screenshot-2026-02-21-1709.png', type: 'screenshot', time: '17:09' },
        { name: 'screenshot-2026-02-21-1711.png', type: 'screenshot', time: '17:11' },
        { name: 'screenshot-2026-02-21-1718.png', type: 'screenshot', time: '17:18' },
        { name: 'screenshot-2026-02-21-2042.png', type: 'screenshot', time: '20:42' }
      ]}
    ];

    /* Day titles from timeline data */
    var visDayTitles = {};
    for (var vd = 0; vd < timelineDays.length; vd++) {
      visDayTitles[timelineDays[vd].day] = timelineDays[vd].title;
    }

    function renderVisualDay(index) {
      var dayData = visualDays[index];
      visCurrent = index;

      var dayTitle = visDayTitles[dayData.day] || '';
      visTitle.textContent = 'DAY ' + dayData.day + ' \u2014 ' + dayTitle;
      visDate.textContent = dayData.date;
      visCount.textContent = dayData.files.length + ' FILES';
      visPagination.textContent = 'DAY ' + dayData.day + ' OF ' + visualDays[visualDays.length - 1].day;

      if (dayData.files.length === 0) {
        visGallery.innerHTML = '<div class="vis-empty">NO VISUAL MEDIA FOR THIS DAY</div>';
        return;
      }

      var html = '';
      for (var i = 0; i < dayData.files.length; i++) {
        var f = dayData.files[i];
        var src = 'media/' + f.name;
        html += '<div class="vis-item" data-src="' + src + '" data-type="' + f.type + '">';
        if (f.type === 'screenshot') {
          html += '<img src="' + src + '" alt="Screenshot from ' + dayData.date + ' at ' + f.time + '" loading="lazy">';
        } else {
          html += '<video src="' + src + '" preload="metadata" muted></video>';
        }
        html += '<div class="vis-item-label">'
          + '<span class="vis-item-type">' + f.type + '</span>'
          + '<span>' + f.time + '</span>'
          + '</div></div>';
      }
      visGallery.innerHTML = html;
      visGallery.scrollTop = 0;

      /* Attach click handlers for lightbox */
      var items = visGallery.querySelectorAll('.vis-item');
      items.forEach(function (item) {
        item.addEventListener('click', function () {
          openLightbox(item.getAttribute('data-src'), item.getAttribute('data-type'));
        });
      });
    }

    function openLightbox(src, type) {
      var overlay = document.createElement('div');
      overlay.className = 'vis-lightbox';

      if (type === 'screenshot') {
        var img = document.createElement('img');
        img.src = src;
        overlay.appendChild(img);
      } else {
        var vid = document.createElement('video');
        vid.src = src;
        vid.controls = true;
        vid.autoplay = true;
        overlay.appendChild(vid);
      }

      overlay.addEventListener('click', function (e) {
        if (e.target === overlay) {
          if (vid) vid.pause();
          document.body.removeChild(overlay);
        }
      });

      document.addEventListener('keydown', function handler(e) {
        if (e.key === 'Escape') {
          if (vid) vid.pause();
          if (overlay.parentNode) document.body.removeChild(overlay);
          document.removeEventListener('keydown', handler);
        }
      });

      document.body.appendChild(overlay);
    }

    renderVisualDay(0);

    visPrev.addEventListener('click', function () {
      renderVisualDay((visCurrent - 1 + visualDays.length) % visualDays.length);
    });

    visNext.addEventListener('click', function () {
      renderVisualDay((visCurrent + 1) % visualDays.length);
    });

    /* Keyboard navigation for visuals page */
    document.addEventListener('keydown', function (e) {
      var visPage = document.querySelector('.page[data-page="visuals"]');
      if (!visPage || !visPage.classList.contains('active')) return;

      if (e.key === 'ArrowLeft') {
        visPrev.click();
      } else if (e.key === 'ArrowRight') {
        visNext.click();
      }
    });
  }
})();
