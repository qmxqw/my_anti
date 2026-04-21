export interface RelativeTimeTextPart {
  value: number;
  text: string;
}

export function formatLargestRelativeTimeParts(
  parts: RelativeTimeTextPart[],
  fallback: string,
  maxUnits = 2,
): string {
  if (maxUnits <= 0) {
    return fallback;
  }

  const visible = parts
    .filter((part) => Number.isFinite(part.value) && part.value > 0)
    .slice(0, maxUnits)
    .map((part) => part.text.trim())
    .filter(Boolean);

  return visible.length > 0 ? visible.join(' ') : fallback;
}
