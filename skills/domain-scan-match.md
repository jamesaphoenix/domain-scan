---
name: domain-scan-match
version: 1.0.0
description: Entity-to-subsystem matching workflow — always --dry-run before --write-back, use --prompt-unmatched for follow-up prompts.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Entity-to-Subsystem Matching

## When to use

Use `domain-scan match` to map extracted structural entities to known subsystems defined in a manifest file (e.g., `system.json`). This is the core workflow for building a complete domain architecture map.

## Key commands

```bash
# Match entities to subsystems
domain-scan match --manifest system.json --output json

# Show only unmatched entities
domain-scan match --manifest system.json --unmatched-only --output json

# Preview what write-back would do (ALWAYS do this first)
domain-scan match --manifest system.json --write-back --dry-run --output json

# Write matched entities back to manifest (after reviewing dry-run)
domain-scan match --manifest system.json --write-back

# Fail if any entities are unmatched (for CI)
domain-scan match --manifest system.json --fail-on-unmatched

# Generate LLM prompts for unmatched items
domain-scan match --manifest system.json --prompt-unmatched --agents 3

# Use JSON input for complex match configuration
domain-scan match --json '{"manifest": "system.json", "unmatched_only": true, "fail_on_unmatched": true}'
```

## Matching workflow

1. **Scan** the codebase: `domain-scan scan --output json --fields stats` to understand size
2. **Match** against manifest: `domain-scan match --manifest system.json --output json`
3. **Review** coverage: check `coverage_percent` and `unmatched` array in output
4. **Dry-run** write-back: `domain-scan match --manifest system.json --write-back --dry-run`
5. **Write back** after review: `domain-scan match --manifest system.json --write-back`
6. **Prompt** for unmatched: `domain-scan match --manifest system.json --prompt-unmatched` to generate LLM sub-agent prompts
7. **Repeat** until `unmatched` count is zero

## Rules

- **Always `--dry-run` before `--write-back`**. The dry-run shows exactly what will be written as structured JSON. Never write back without reviewing first.
- Check `match_strategy` on each matched entity: `file_path` matches are high confidence, `name_match` matches may need human review.
- Use `--fail-on-unmatched` in CI to ensure complete subsystem coverage.
- For `built` modules, matches are deterministic and authoritative. For `unbuilt`/`rebuild` modules, treat matches as proposals requiring human review.

## Write-back format preservation

`--write-back` auto-detects whether the manifest is a full `SystemManifest` (with
meta/domains/connections) or a plain `Manifest` (subsystems only). It preserves all
existing fields. Requires domain-scan >= 0.3.0.

## Match strategies (in priority order)

1. **file_path** — Entity's file path is a prefix of subsystem's `filePath`. Deepest match wins.
2. **import_graph** — Entity is connected to a subsystem via import chains.
3. **name_match** — Entity name appears in subsystem's `interfaces`, `operations`, `tables`, or `events` arrays.

## Common mistakes

- Running `--write-back` without `--dry-run` first → unexpected manifest changes. Always preview first.
- Ignoring `match_strategy` field → trusting `name_match` results as much as `file_path` results. Name matches are heuristic and may be wrong.
- Not checking `coverage_percent` → assuming all entities are matched. Always check coverage before proceeding.
- Using `--fail-on-unmatched` during initial matching → it will always fail on first run. Use it only after iterating to near-complete coverage.
