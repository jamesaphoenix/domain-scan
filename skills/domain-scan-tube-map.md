---
name: domain-scan-tube-map
version: 1.0.0
description: How to view and interact with the Subsystem Tube Map — load manifests, check coverage, trace dependencies, fix unmatched entities.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Subsystem Tube Map Interaction

## When to use

Use the tube map workflow to visualize a codebase's domain architecture as a London Underground-style map. Subsystems are stations, domains are colored lines, and dependencies are edges between stations.

## Key commands

```bash
# Check entity-to-subsystem coverage
domain-scan match --manifest system.json --output json --fields coverage_percent,unmatched_count

# List unmatched entities
domain-scan match --manifest system.json --unmatched-only --output json --fields "name,kind,file"

# Generate prompts for unmatched items
domain-scan match --manifest system.json --prompt-unmatched --agents 3

# Validate manifest structure
domain-scan init --apply-manifest system.json --dry-run --output json

# Get subsystem detail
domain-scan match --manifest system.json --output json --fields "subsystems.id,subsystems.matched_entity_count"
```

## Workflow: From scan to tube map

1. **Scan** the codebase: `domain-scan scan --root . --output json --fields stats`
2. **Bootstrap** a manifest: `domain-scan init --bootstrap -o system.json`
3. **Refine** the manifest (see `domain-scan-init` skill)
4. **Match** entities: `domain-scan match --manifest system.json --output json`
5. **Fix unmatched**: iterate on the manifest until `coverage_percent` is satisfactory
6. **View** in the Tauri app: load the manifest in the Subsystem Tube Map tab

## Interpreting coverage

| Coverage | Meaning | Action |
|----------|---------|--------|
| >90% | Excellent | Minor cleanup — check remaining unmatched items |
| 70-90% | Good | Review unmatched items, may need new subsystems or adjusted file paths |
| 50-70% | Fair | Significant gaps — likely missing domains or subsystem boundaries are wrong |
| <50% | Poor | Manifest needs major rework — re-bootstrap or restructure domains |

## Fixing unmatched entities

1. Run `domain-scan match --manifest system.json --unmatched-only --output json --fields "name,kind,file"`
2. Group unmatched entities by directory/module
3. For each group, either:
   - Expand an existing subsystem's `filePath` to include the directory
   - Create a new subsystem in the appropriate domain
   - Add entity names to a subsystem's `interfaces`, `operations`, `tables`, or `events` arrays
4. Re-validate: `domain-scan init --apply-manifest system.json --dry-run`
5. Re-match: `domain-scan match --manifest system.json --output json --fields coverage_percent`

## Dependency tracing

In the Tauri app, clicking "trace" on a station highlights the full dependency chain:
- **Upstream**: all stations that this station depends on (transitively)
- **Downstream**: all stations that depend on this station (transitively)
- **Both**: full transitive closure in both directions

To check dependencies programmatically:
```bash
domain-scan match --manifest system.json --output json --fields "connections"
```

## Common mistakes

- Loading a manifest without scanning first — the tube map needs both scan data and manifest data to show entity counts and coverage.
- Ignoring low coverage — a tube map with <70% coverage will have many stations with zero matched entities, making the visualization misleading.
- Not using `--unmatched-only` — dumping the full match output into context when you only need to see what's missing.
- Editing the manifest without re-validating — structural errors (dangling subsystem references in connections, orphan domains) will cause rendering issues.
