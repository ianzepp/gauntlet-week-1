# AI Development Log — Field Board

**Project:** Real-time collaborative whiteboard, full-stack Rust (Axum + Leptos SSR/WASM)
**Sprint:** 6 days (Feb 16–22), 501 commits, ~50K lines of Rust across 7 crates, 945 tests

---

## Tools & Workflow

| Tool | Role |
|---|---|
| **Claude Code (Opus)** | Architecture, design decisions, correctness analysis, documentation |
| **Codex** | Iterative implementation, feature build-out, bulk of commit volume |
| **CLAUDE.md** | Enforced workflow rules for both agents: `cargo fmt → clippy → test → commit`, no `.unwrap()` in prod, dedicated test files |

Commit breakdown: ~320 Codex commits (~64%), 177 Claude commits (~35%), <5 manual commits (<1%). Both AI tools auto-committed after passing all checks.

## MCP Usage

None. The project used native Rust tooling exclusively (SQLx compile-time queries, Leptos SSR/WASM, Prost protobuf codegen).

## Effective Prompts

The most effective prompts were reusable **custom agents** (saved as markdown skill files):

1. **Rust Correctness Surgeon** (`rust-correctness-surgeon.md`) — An Opus-model agent prompted as a Staff Engineer / HFT systems programmer. Instruction: *"Every line of code is guilty until proven correct."* Systematically checks soundness, error handling integrity, data flow invariants, logic correctness, and resource management. Reports findings by severity with concrete fix code.

2. **Technical Writer** (`technical-writer.md`) — A Sonnet-model agent that documents WHY over WHAT. Adds module-level architecture overviews, phase markers for complex operations, and trade-off documentation. Run after large change sets before final commit.

3. **CLAUDE.md as a persistent prompt** — Rather than repeating constraints per-conversation, rules like "no `.unwrap()` in non-test code" and "tests go in dedicated `*_test.rs` files" were codified once and enforced automatically across all agent sessions.

## Code Analysis

| Category | % |
|---|---|
| AI-generated (Codex implementation + Claude architecture) | ~99% |
| Hand-written | <1% |

Virtually all code was AI-generated. The human role was directing *what* to build and *how* to structure it, not writing code directly.

## Strengths & Limitations

**Where AI excelled:**

- Codex was effective at iterative implementation — grinding through feature work, test writing, and fixing clippy/test failures in a loop
- Claude was strong at high-level design, architectural review, and correctness analysis
- The two-tool split (Claude for thinking, Codex for doing) worked well in practice

**Where AI struggled:**

- Claude tends to gloss over technical requirements during implementation — it would produce code that looked right but missed edge cases or subtleties, making it less reliable for hands-on coding compared to Codex
- Neither tool was good at performance tuning or making judgment calls about real-time system trade-offs (flush intervals, queue capacities, deadband thresholds)

## Key Learnings

- **Rust is amazing when you don't have to write it yourself.** The language's strict compiler means AI-generated code that compiles is already reasonably correct — the type system and borrow checker catch entire categories of bugs before tests even run.
- **Split your AI tools by strength.** Claude for design/review, Codex for implementation volume. Trying to use one tool for everything produced worse results.
- **Custom agents are reusable prompts.** The Rust Correctness Surgeon and Technical Writer agents provided consistent, high-quality analysis across dozens of invocations without re-explaining the task each time.
