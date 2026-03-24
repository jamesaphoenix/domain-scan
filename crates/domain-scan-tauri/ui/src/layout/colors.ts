const STATIC_PALETTE = [
  "#3b82f6", // blue
  "#22c55e", // green
  "#f97316", // orange
  "#a855f7", // purple
  "#ef4444", // red
  "#eab308", // yellow
  "#06b6d4", // cyan
  "#ec4899", // pink
  "#14b8a6", // teal
  "#f59e0b", // amber
  "#6366f1", // indigo
  "#84cc16", // lime
];

/** Convert HSL values to hex color string. */
function hslToHex(h: number, s: number, l: number): string {
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
    return Math.round(255 * color)
      .toString(16)
      .padStart(2, "0");
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

/**
 * Assign colors to domains.
 *
 * Priority:
 * 1. Use manifest-specified color if available
 * 2. Fall back to static 12-color palette
 * 3. If > 12 domains without manifest colors, cycle with HSL
 */
export function assignDomainColors(
  manifestDomains: Record<string, { label: string; color: string }>,
  allDomainIds: string[],
): Map<string, string> {
  const colors = new Map<string, string>();
  let paletteIndex = 0;

  for (const id of allDomainIds) {
    const def = manifestDomains[id];
    if (def?.color) {
      colors.set(id, def.color);
    } else if (paletteIndex < STATIC_PALETTE.length) {
      colors.set(id, STATIC_PALETTE[paletteIndex]!);
      paletteIndex++;
    } else {
      const hue = (paletteIndex / Math.max(allDomainIds.length, 1)) * 360;
      colors.set(id, hslToHex(hue, 0.65, 0.55));
      paletteIndex++;
    }
  }

  return colors;
}
