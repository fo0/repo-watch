import { describe, expect, it } from 'vitest';
import { matchesTerms, searchTerms } from './search';

describe('searchTerms', () => {
  it('splits the query into lower-cased words', () => {
    expect(searchTerms('  Release   SITE\tmain ')).toEqual([
      'release',
      'site',
      'main',
    ]);
  });
  it('yields no words for an empty or blank query', () => {
    expect(searchTerms('')).toEqual([]);
    expect(searchTerms('   ')).toEqual([]);
  });
});

describe('matchesTerms', () => {
  const text = 'example-org/site-demo Release main';
  it('matches when every word occurs, in any order', () => {
    expect(matchesTerms(text, searchTerms('release site'))).toBe(true);
    expect(matchesTerms(text, searchTerms('MAIN example-org/site'))).toBe(true);
    expect(matchesTerms(text, searchTerms('site-demo release'))).toBe(true);
  });
  it('fails when one word is missing', () => {
    expect(matchesTerms(text, searchTerms('release develop'))).toBe(false);
  });
  it('matches everything without words', () => {
    expect(matchesTerms(text, [])).toBe(true);
  });
});
