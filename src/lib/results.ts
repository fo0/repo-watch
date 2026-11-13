/**
 * The main button of each listed result: a live run, a favorite, a
 * repository row or a Verlauf run, in the order the page shows them.
 */
export const RESULT =
  '.content :is(.live-run, .pin-open, .repository-toggle, .run-button)';

/**
 * Where ArrowDown or ArrowUp on the result at `index` (of `count`) moves
 * focus: the neighbouring result, the search field from the first one, or
 * null to leave the key alone (the last result, any other key).
 */
export function resultStep(
  key: string,
  index: number,
  count: number,
): number | 'search' | null {
  if (index < 0 || index >= count) return null;
  if (key === 'ArrowDown') return index + 1 < count ? index + 1 : null;
  if (key === 'ArrowUp') return index > 0 ? index - 1 : 'search';
  return null;
}
