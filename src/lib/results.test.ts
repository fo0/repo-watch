import { describe, expect, it } from 'vitest';
import { resultStep } from './results';

describe('resultStep', () => {
  it('moves to the next and previous result', () => {
    expect(resultStep('ArrowDown', 0, 3)).toBe(1);
    expect(resultStep('ArrowDown', 1, 3)).toBe(2);
    expect(resultStep('ArrowUp', 2, 3)).toBe(1);
  });
  it('returns to the search from the first result', () => {
    expect(resultStep('ArrowUp', 0, 3)).toBe('search');
    expect(resultStep('ArrowUp', 0, 1)).toBe('search');
  });
  it('leaves the key alone on the last result, other keys and stray indices', () => {
    expect(resultStep('ArrowDown', 2, 3)).toBeNull();
    expect(resultStep('Home', 1, 3)).toBeNull();
    expect(resultStep('ArrowLeft', 1, 3)).toBeNull();
    expect(resultStep('ArrowDown', -1, 3)).toBeNull();
    expect(resultStep('ArrowUp', 3, 3)).toBeNull();
  });
});
