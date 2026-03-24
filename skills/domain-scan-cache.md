---
name: domain-scan-cache
version: 1.0.0
description: Cache management — use cache stats before clearing, use --no-cache for debugging, prefer cache prune over cache clear.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Cache Management

## When to use

Use `domain-scan cache` to inspect, prune, or clear the content-addressed parse cache. The cache stores tree-sitter parse results keyed by SHA-256(path + content), so unchanged files are never re-parsed.

## Key commands

```bash
# Check cache size and entry count
domain-scan cache stats

# Preview what prune would remove (stale entries for deleted/changed files)
domain-scan cache prune --dry-run

# Remove stale cache entries
domain-scan cache prune

# Preview what clear would delete
domain-scan cache clear --dry-run

# Delete entire cache
domain-scan cache clear

# Bypass cache for a single scan (without clearing it)
domain-scan scan --no-cache
```

## Rules

- Always run `cache stats` before `cache clear` to understand what you're deleting. The cache may contain results for thousands of files.
- Prefer `cache prune` over `cache clear`. Prune only removes entries for files that have been deleted or changed — it preserves valid cached results.
- **Always `--dry-run` before `cache clear` or `cache prune`**. The dry-run output shows each entry that would be removed as structured JSON with `action`, `target`, and `reason` fields.
- Use `--no-cache` on individual commands to bypass the cache for debugging, rather than clearing the entire cache.
- The cache invalidates automatically when file content changes (content-addressed by SHA-256). You rarely need to manually manage it.

## When to clear cache

- After upgrading domain-scan (query files may have changed, producing different IR)
- After modifying `.scm` query files during development
- When cache disk size is unreasonably large (check with `cache stats`)

## When to prune cache

- After deleting many files from the scanned codebase
- After a large branch switch that changes many files
- As routine maintenance to reclaim disk space

## Common mistakes

- Running `cache clear` when `cache prune` would suffice → destroys valid cached results, causing the next scan to re-parse everything.
- Not using `--dry-run` before clearing → deleting more than expected.
- Manually clearing the cache because results seem stale → the issue is more likely a stale file on disk or a `--build-status` override. Use `--no-cache` on the specific command first to diagnose.
- Clearing cache to "fix" incorrect extractions → cache stores correct parse results. If extractions are wrong, the issue is in the `.scm` query files, not the cache.
