# AI Development Cost — Gauntlet Week 1 (Feb 16–22, 2026)

Token usage and API spend for the six-day Field Board sprint, measured across all
AI coding sessions recorded by [cassio](https://github.com/ianzepp/cassio).

## Daily breakdown

| Date | Sessions | Tokens in | Tokens out | Cost | Wall time |
|------|----------|-----------|------------|------|-----------|
| Feb 16 | 95 | 1.8M | 228.7K | $201 | 17h 41m |
| Feb 17 | 68 | 89.7M | 820.7K | $277 | 30h 28m |
| Feb 18 | 60 | 144.8M | 804.9K | $350 | 29h 28m |
| Feb 19 | 22 | 345.9M | 1.3M | $630 | 31h 4m |
| Feb 20 | 13 | 58.9M | 271.0K | $111 | 5h 8m |
| Feb 21 | 34 | 132.1M | 537.9K | $343 | 35h 14m |
| Feb 22 | 55 | 82.0M | 517.8K | $221 | 9h 43m |
| **Week** | **347** | **855.2M** | **4.5M** | **$2,133** | **158h 46m** |

Wall time exceeds 24 h/day on several days because agent sessions ran in parallel
across multiple Claude Code worktrees simultaneously.

## Project breakdown (gauntlet-week-1 only)

| Project path | Sessions | Cost |
|---|---|---|
| `github/ianzepp/gauntlet-week-1` | 155 | $1,607 |
| `github/gauntlet/collaboard` | 70 | $206 |
| `ianzepp/github/gauntlet-week-1` | 12 | $106 |
| `ianzepp/github/gauntlet` | 30 | $65 |
| Other gauntlet paths | 6 | $6 |
| **Gauntlet total** | **273** | **$1,990** |

The remaining ~$143 of the week's spend came from non-gauntlet sessions that ran
in parallel during the same period.

## Notes on token counting

- **Tokens in** counts only non-cached input tokens (the uncached portion billed at
  the full input rate).
- Cache-read tokens (billed at ~10% of the input rate) and cache-write tokens
  (billed at ~125%) are included in the cost calculation but are not shown in the
  table above — they dominated volume on the heavier days (e.g., Feb 19 processed
  ~346M raw input tokens largely through prompt-cache reads).
- Pricing used: Claude Sonnet $3.00/$15.00 input/output per MTok; Claude Opus
  $5.00/$25.00; cache read 10% of input rate, cache write 125% of input rate.
- Tool: [cassio](https://github.com/ianzepp/cassio) v0.4.0.
