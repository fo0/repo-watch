/**
 * The words of a search query, lower-cased; whitespace alone yields none.
 * Every page filters with the same rule, so a query reads the same everywhere.
 */
export function searchTerms(query: string): string[] {
  return query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
}

/**
 * Whether `text` contains every word of the query, in any order and anywhere,
 * e.g. „release site“ finds „demo/site-demo Release“. No words match all.
 */
export function matchesTerms(text: string, terms: readonly string[]): boolean {
  const haystack = text.toLocaleLowerCase();
  return terms.every((term) => haystack.includes(term));
}
