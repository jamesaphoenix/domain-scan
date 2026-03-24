---
name: domain-scan-prompt
version: 1.0.0
description: LLM prompt generation — use --focus to scope, --include-scan for self-contained prompts, choose agent count based on codebase size.
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# LLM Prompt Generation

## When to use

Use `domain-scan prompt` to generate structured LLM sub-agent prompts that partition a codebase analysis task across multiple agents. Each agent gets a scoped assignment with file lists, entity counts, and build-status-aware instructions.

## Key commands

```bash
# Generate a prompt with 5 agents (default)
domain-scan prompt

# Customize agent count based on codebase size
domain-scan prompt --agents 3

# Focus on a specific domain area
domain-scan prompt --focus "auth"

# Include full scan data in the prompt (self-contained)
domain-scan prompt --include-scan

# Combine focus with scan embedding
domain-scan prompt --focus "payment" --agents 4 --include-scan

# Output as JSON for programmatic processing
domain-scan prompt --output json

# Use JSON input
domain-scan prompt --json '{"agents": 4, "focus": "auth", "include_scan": true}'
```

## Rules

- Choose agent count based on codebase size: 2-3 for small (<500 files), 4-5 for medium (500-2000), 6-8 for large (>2000).
- Use `--focus` to scope prompts to a specific domain area (regex matched against entity names). This dramatically reduces prompt size and improves agent focus.
- Use `--include-scan` only when agents need the full structural data. Without it, prompts include file lists and entity counts but not the full IR — which is usually sufficient.
- The prompt automatically adapts its partitioning strategy based on codebase size:
  - **< 500 files:** ByConcern — 5 categories (Interface Audit, Service Architecture, Method Census, Cross-Cutting Concerns, Implementation Audit)
  - **500-2000 files:** Hybrid — concern + directory partitioning
  - **> 2000 files:** ByDirectory — directory-based with concern sub-partitions
- Built files get "verify and catalog" instructions. Unbuilt/Rebuild files get "analyze and infer" instructions. This is automatic — don't override it.

## Common mistakes

- Setting `--agents` too high for small codebases → agents get trivially small scopes with not enough work. Match agent count to codebase size.
- Not using `--focus` → generating a prompt for the entire codebase when you only need one domain area analyzed.
- Always using `--include-scan` → embedding the full scan inflates the prompt significantly. Only use when agents genuinely need entity-level detail, not just file lists.
- Ignoring build status in agent output → agents working on `unbuilt` files will produce lower-confidence results. Review these with extra scrutiny.
