/**
 * Pure edits of `reader.translations`, the ordered list of the pack ids shown
 * in the reader. Each returns a new list and leaves the input untouched.
 */

/** Shows `id` (appended last) or hides it. */
export function toggleTranslation(
  selected: readonly string[],
  id: string,
  shown: boolean,
): string[] {
  if (shown) return selected.includes(id) ? [...selected] : [...selected, id];
  return selected.filter((item) => item !== id);
}

/** Moves `id` one place earlier (`-1`) or later (`1`); a no-op at either end. */
export function moveTranslation(selected: readonly string[], id: string, by: -1 | 1): string[] {
  const from = selected.indexOf(id);
  const to = from + by;
  if (from < 0 || to < 0 || to >= selected.length) return [...selected];
  const next = [...selected];
  [next[from], next[to]] = [selected[to] ?? id, id];
  return next;
}
