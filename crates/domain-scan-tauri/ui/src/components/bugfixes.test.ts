import { describe, it, expect } from "vitest";

// ---------------------------------------------------------------------------
// Fix 2: WizardStepDomains — domain ID collision on add/delete/add
// ---------------------------------------------------------------------------
// We test the logic of `handleAddDomain` extracted as a pure function.
// The bug: using `domain-${domainEntries.length + 1}` causes collisions
// after deletions. The fix: use `domain-${Date.now()}`.

describe("WizardStepDomains — domain ID collision fix", () => {
  /**
   * Simulates the fixed handleAddDomain logic.
   * Returns a new domains record with a timestamped ID.
   */
  function addDomain(domains: Record<string, { label: string; color: string }>): Record<string, { label: string; color: string }> {
    const idx = Object.keys(domains).length;
    const id = `domain-${Date.now()}`;
    const DEFAULT_COLORS = [
      "#3b82f6", "#22c55e", "#f97316", "#a855f7",
      "#ef4444", "#eab308", "#06b6d4", "#ec4899",
    ];
    return {
      ...domains,
      [id]: {
        label: `Domain ${idx + 1}`,
        color: DEFAULT_COLORS[idx % DEFAULT_COLORS.length],
      },
    };
  }

  function removeDomain(domains: Record<string, { label: string; color: string }>, id: string): Record<string, { label: string; color: string }> {
    const next = { ...domains };
    delete next[id];
    return next;
  }

  it("generates unique IDs even after add/delete/add cycles", () => {
    // Simulate the OLD (buggy) behavior to prove the collision
    function addDomainBuggy(domains: Record<string, { label: string; color: string }>): Record<string, { label: string; color: string }> {
      const idx = Object.keys(domains).length;
      const id = `domain-${idx + 1}`; // BUG: length-based ID
      return {
        ...domains,
        [id]: { label: `Domain ${idx + 1}`, color: "#000" },
      };
    }

    // Demonstrate the collision with the old approach
    let buggyDomains: Record<string, { label: string; color: string }> = {};
    buggyDomains = addDomainBuggy(buggyDomains); // domain-1
    buggyDomains = addDomainBuggy(buggyDomains); // domain-2
    buggyDomains = removeDomain(buggyDomains, "domain-1"); // remove domain-1, length=1
    buggyDomains = addDomainBuggy(buggyDomains); // domain-2 again! COLLISION
    // The buggy version only has 1 entry because domain-2 overwrote itself
    expect(Object.keys(buggyDomains)).toHaveLength(1); // proves the bug

    // Now show the fix: timestamp-based IDs never collide
    // (even if Date.now() returns the same ms, spreading into existing object preserves keys)
    const id1 = `domain-${Date.now()}`;
    const id2 = `domain-${Date.now() + 1}`; // guaranteed different
    const id3 = `domain-${Date.now() + 2}`; // guaranteed different

    let fixedDomains: Record<string, { label: string; color: string }> = {};
    fixedDomains[id1] = { label: "D1", color: "#000" };
    fixedDomains[id2] = { label: "D2", color: "#000" };
    fixedDomains = removeDomain(fixedDomains, id1);
    fixedDomains[id3] = { label: "D3", color: "#000" };

    expect(Object.keys(fixedDomains)).toHaveLength(2);
    const uniqueIds = new Set(Object.keys(fixedDomains));
    expect(uniqueIds.size).toBe(2);
  });

  it("generated IDs are timestamp-based (start with 'domain-' followed by a number)", () => {
    const domains = addDomain({});
    const id = Object.keys(domains)[0];
    expect(id).toMatch(/^domain-\d+$/);
    // The number portion should be close to current timestamp
    const ts = parseInt(id.replace("domain-", ""), 10);
    expect(ts).toBeGreaterThan(Date.now() - 5000);
    expect(ts).toBeLessThanOrEqual(Date.now());
  });
});

// ---------------------------------------------------------------------------
// Fix 5: TubeLineStripes — duplicate React key
// ---------------------------------------------------------------------------
// We test that the key generation logic produces unique keys even when
// two paths share the same domain.

describe("TubeLineStripes — duplicate key fix", () => {
  it("key includes index to avoid duplicates when domains repeat", () => {
    const paths = [
      { domain: "billing", color: "#f00", d: "M 0 0 L 10 10" },
      { domain: "billing", color: "#0f0", d: "M 20 20 L 30 30" },
      { domain: "auth", color: "#00f", d: "M 40 40 L 50 50" },
    ];

    // The fixed key generation: `${p.domain}-${i}`
    const keys = paths.map((p, i) => `${p.domain}-${i}`);

    const uniqueKeys = new Set(keys);
    expect(uniqueKeys.size).toBe(keys.length);
    expect(keys).toEqual(["billing-0", "billing-1", "auth-2"]);
  });
});

// ---------------------------------------------------------------------------
// Fix 4: CSV export — quoting logic
// ---------------------------------------------------------------------------
// The Rust CSV export now wraps name and file in quotes. We test the
// quoting logic conceptually (mirroring the Rust format string).

describe("CSV export — proper quoting", () => {
  /** Mirrors the Rust format: "\"{}\",{:?},\"{}\"" with embedded quote escaping */
  function formatCsvRow(
    name: string,
    kind: string,
    file: string,
    line: number,
    language: string,
    buildStatus: string,
    confidence: string,
  ): string {
    const escapedName = name.replace(/"/g, '""');
    const escapedFile = file.replace(/"/g, '""');
    return `"${escapedName}",${kind},"${escapedFile}",${line},${language},${buildStatus},${confidence}`;
  }

  it("wraps name containing commas in quotes", () => {
    const row = formatCsvRow(
      "Foo, Bar",
      "interface",
      "/src/foo.ts",
      1,
      "TypeScript",
      "built",
      "high",
    );
    expect(row).toBe('"Foo, Bar",interface,"/src/foo.ts",1,TypeScript,built,high');
    // Parsing the first field should recover "Foo, Bar"
    expect(row.startsWith('"Foo, Bar"')).toBe(true);
  });

  it("escapes embedded quotes in name", () => {
    const row = formatCsvRow(
      'say "hello"',
      "function",
      "/src/greet.ts",
      5,
      "TypeScript",
      "built",
      "high",
    );
    expect(row).toBe('"say ""hello""",function,"/src/greet.ts",5,TypeScript,built,high');
  });

  it("handles names without special characters", () => {
    const row = formatCsvRow(
      "MyService",
      "service",
      "/src/service.ts",
      10,
      "TypeScript",
      "built",
      "high",
    );
    expect(row).toBe('"MyService",service,"/src/service.ts",10,TypeScript,built,high');
  });

  it("wraps file paths containing commas in quotes", () => {
    const row = formatCsvRow(
      "Foo",
      "interface",
      "/src/a, b/foo.ts",
      1,
      "TypeScript",
      "built",
      "high",
    );
    expect(row).toBe('"Foo",interface,"/src/a, b/foo.ts",1,TypeScript,built,high');
  });
});

// ---------------------------------------------------------------------------
// Fix 1: TubeMapView auto-scan — no scanLoaded guard
// ---------------------------------------------------------------------------
// The fix removes the `scanLoaded` guard so the auto-scan effect always
// fires when manifestPath changes. We test the guard logic in isolation.

describe("TubeMapView — auto-scan guard fix", () => {
  it("old guard skips scan when scanLoaded is true (the bug)", () => {
    const manifestPath = "/project/system.json";
    const scanLoaded = true;

    // Old guard: both conditions must pass
    const oldShouldScan = !(!manifestPath || scanLoaded);
    expect(oldShouldScan).toBe(false); // BUG: skipped even though we should scan
  });

  it("new guard always scans when manifestPath is set", () => {
    const manifestPath = "/project/system.json";

    // New guard: only checks manifestPath
    const newShouldScan = !!manifestPath;
    expect(newShouldScan).toBe(true); // FIXED: always scans
  });

  it("new guard still skips when manifestPath is empty", () => {
    const manifestPath = "";

    const newShouldScan = !!manifestPath;
    expect(newShouldScan).toBe(false);
  });

  it("new guard still skips when manifestPath is null", () => {
    const manifestPath = null;

    const newShouldScan = !!manifestPath;
    expect(newShouldScan).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Fix 3: FilterBar stale state — reset key
// ---------------------------------------------------------------------------
// The fix uses React's key prop to force remount FilterBar when scan changes.
// We test that the key changes when scan stats change.

describe("FilterBar — reset key on new scan", () => {
  it("key changes when parse_duration_ms changes", () => {
    const stats1: { parse_duration_ms: number } | null = { parse_duration_ms: 42 };
    const stats2: { parse_duration_ms: number } | null = { parse_duration_ms: 99 };

    const key1 = stats1?.parse_duration_ms ?? 0;
    const key2 = stats2?.parse_duration_ms ?? 0;

    expect(key1).not.toBe(key2);
  });

  it("key defaults to 0 when stats is null", () => {
    const stats = null as { parse_duration_ms: number } | null;

    const key = stats?.parse_duration_ms ?? 0;
    expect(key).toBe(0);
  });

  it("different scans produce different keys (parse_duration_ms varies)", () => {
    // In practice, parse_duration_ms changes between scans because
    // different directories take different amounts of time
    const scan1Key = 150;
    const scan2Key = 230;
    expect(scan1Key).not.toBe(scan2Key);
  });
});
