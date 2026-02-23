# Production Cost Projections

## Assumptions

| Parameter | Value |
|-----------|-------|
| Sessions per user per month | 3 |
| AI commands per session | 8 |
| AI commands per user per month | 24 |
| LLM calls per command (average) | 2.7 |
| Input tokens per LLM call | ~3,900 |
| Output tokens per LLM call | ~165 |

Token counts are derived from production trace data. Input tokens reflect post-fix baseline
(tool call inputs stripped from history to prevent context bloat on agentic loops).

## Pricing

| Model | Input | Output |
|-------|-------|--------|
| `moonshotai/kimi-k2-0905` | $0.60 / 1M tokens | $2.50 / 1M tokens |
| `google/gemini-2.5-flash-lite-preview-09-2025` | $0.10 / 1M tokens | $0.40 / 1M tokens |

Effective cost per command:
- **Kimi K2**: ~$0.0075/command
- **Gemini Flash-lite**: ~$0.0009/command
- **Blended (50/50)**: ~$0.0042/command

## Monthly Cost Projections

| Scale | 100 Users | 1,000 Users | 10,000 Users | 100,000 Users |
|-------|----------:|------------:|-------------:|--------------:|
| Kimi K2 only | $18/mo | $180/mo | $1,800/mo | $18,000/mo |
| Gemini Flash-lite only | $2/mo | $22/mo | $216/mo | $2,160/mo |
| Blended 50/50 | $10/mo | $100/mo | $1,000/mo | $10,000/mo |

## Notes

- No caching or volume discounts applied — actual costs likely lower at scale.
- P90 command cost is ~3x the average due to power users running long agentic loops
  (10+ LLM calls/command observed in production traces). Budget accordingly.
- Output tokens dominate Kimi K2 cost despite being a small fraction of volume.
- Gemini Flash-lite output cost is negligible; input dominates.
