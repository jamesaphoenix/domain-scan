---
name: domain-scan-init
version: 1.0.0
description: How to build and refine system.json manifests — always start with --bootstrap, use sub-agents per domain, never auto-confirm built status.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Building & Refining System Manifests

## When to use

Use `domain-scan init` to create and validate system.json manifests that map a codebase's structural entities to logical subsystems and domains. This is the first step before viewing the Subsystem Tube Map.

## Key commands

```bash
# Bootstrap a starter manifest from heuristics
domain-scan init --bootstrap -o system.json

# Bootstrap with a custom project name
domain-scan init --bootstrap --name "my-project" -o system.json

# Validate an existing manifest (dry-run shows errors without writing)
domain-scan init --apply-manifest system.json --dry-run --output json

# Apply (load + validate) a manifest
domain-scan init --apply-manifest system.json

# Get the JSON Schema for system.json
domain-scan schema init
```

## Workflow

1. **Always start with `--bootstrap`** — never write system.json from scratch. The heuristics produce a reasonable starter manifest from directory structure and import analysis.
2. **Review the bootstrap output** — check domains, subsystems, and connections. The bootstrap sets all subsystems to `status: "new"` by default.
3. **Refine with sub-agents per domain** — each sub-agent focuses on one domain's subsystem boundaries, names, descriptions, and entity placement.
4. **Validate after every edit** — run `domain-scan init --apply-manifest system.json --dry-run` to catch structural errors.
5. **Check coverage** — run `domain-scan match --manifest system.json --output json --fields coverage_percent` to see how many entities are matched.
6. **Confirm with the user** before marking any subsystem as `built`.

## Rules

- **Never auto-confirm `built` status.** `built` means "source code is authoritative, high confidence." Only the user can upgrade a subsystem from `new` to `built`. The agent proposes `new` for everything.
- **Always `--dry-run` before writing.** Preview validation errors and coverage before committing changes.
- **Use kebab-case for subsystem IDs** (e.g., `auth-jwt`, `billing-stripe`).
- **Use verb-first connection labels** (e.g., `reads from`, `triggers`, `authenticates via`).
- **3-8 subsystems per domain** is the sweet spot. Fewer = too coarse. More = too granular.
- **Schemas anchor subsystems** — if a subsystem has Drizzle/Zod/Prisma schemas, those schemas define its data boundary.
- **No utility domains** — "utils", "shared", "common" are not domains. Distribute those entities to the domains that use them.
- **No duplicate subsystems** — if two subsystems share >50% of their entities, merge them.
- **No test-only connections** — connections represent runtime dependencies, not test imports.

## Connection semantics

| Type | Meaning | Example |
|------|---------|---------|
| `depends_on` | Hard runtime dependency (won't work without it) | `billing-stripe` depends_on `auth-sessions` |
| `uses` | Soft dependency (calls but could be replaced) | `notifications` uses `email-provider` |
| `triggers` | Async/event-driven (fires and forgets) | `order-processor` triggers `inventory-update` |

## Editing manifests

The agent edits `system.json` directly — no special patch API needed:
1. Read `system.json`
2. Edit the JSON (split, merge, rename, move entities between subsystems)
3. Validate: `domain-scan init --apply-manifest system.json --dry-run`
4. Check coverage: `domain-scan match --manifest system.json --output json --fields coverage_percent`
5. Repeat until coverage is satisfactory

## Nested subsystems (children)

Use the `children` array when a single package has distinct sub-modules that deserve separate tracking. For example, `packages/platform/` might contain `auth`, `billing`, and `notifications` as independent concerns.

### When to use

- A directory contains 3+ logically distinct modules (not just files)
- Each child maps to a sub-directory with its own interfaces/schemas
- The parent subsystem would be too coarse without splitting

### Format

Children use the same structure as top-level subsystems, nested inside the parent's `children` array:

```json
{
  "id": "platform-core",
  "name": "Platform Core",
  "domain": "platform",
  "status": "new",
  "filePath": "packages/platform/",
  "children": [
    {
      "id": "platform-auth",
      "name": "Platform Auth",
      "domain": "platform",
      "status": "new",
      "filePath": "packages/platform/src/auth/",
      "interfaces": [],
      "operations": [],
      "tables": [],
      "events": [],
      "dependencies": ["platform-sessions"]
    },
    {
      "id": "platform-billing",
      "name": "Platform Billing",
      "domain": "platform",
      "status": "new",
      "filePath": "packages/platform/src/billing/",
      "interfaces": [],
      "operations": [],
      "tables": [],
      "events": [],
      "dependencies": ["platform-auth"]
    }
  ]
}
```

### Matching behavior

- Children inherit the parent's domain
- Children override `filePath` with a more specific path
- The matching engine resolves children by path specificity — a child's `filePath` is always more specific than the parent's, so entities match to the most precise subsystem

### Rules

- Children must have a more specific `filePath` than the parent
- Do not nest deeper than one level (children of children) — flatten instead
- Prefer 2-5 children per parent. If you need more, the parent is probably a domain, not a subsystem

## Common mistakes

- Writing system.json from scratch instead of using `--bootstrap` — the heuristics save significant time and produce reasonable starting points.
- Marking subsystems as `built` without user confirmation — this causes the tube map to treat source code as authoritative and skip LLM enrichment on entities that may need it.
- Creating "utility" or "shared" domains — these are anti-patterns. Distribute utilities to the domains that use them.
- Forgetting to validate after edits — structural errors (dangling references, orphan domains) will cause the tube map to render incorrectly.
- Using `depends_on` for everything — distinguish between hard dependencies, soft usage, and async triggers.
