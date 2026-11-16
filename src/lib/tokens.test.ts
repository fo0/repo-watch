import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

/*
 * Reads the design tokens straight from the CSS, resolves both themes and
 * checks WCAG AA contrast for text and status colours, then checks that no
 * other stylesheet or component hard-codes a colour.
 */
const STYLES = join(process.cwd(), 'src/styles');
const LIB = join(process.cwd(), 'src/lib');
const TOKENS = readFileSync(join(STYLES, '00-tokens.css'), 'utf8').replace(
  /\/\*[\s\S]*?\*\//g,
  '',
);

/** Hex, rgb()/hsl() and the other colour functions; not `#each` and friends. */
const COLOUR_LITERAL =
  /(?<![\w&{])#(?:[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{3,4})\b|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch)\(/i;

type Decls = Record<string, string>;
type Rgba = [number, number, number, number];

/** The declarations of the first rule whose selector line matches `head`. */
function block(head: RegExp): Decls {
  const start = TOKENS.search(head);
  if (start < 0) throw new Error(`no token block ${head}`);
  const open = TOKENS.indexOf('{', start);
  const body = TOKENS.slice(open + 1, TOKENS.indexOf('}', open));
  const decls: Decls = {};
  for (const [, name, value] of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g))
    decls[name!] = value!.replace(/\s+/g, ' ').trim();
  return decls;
}

const dark = block(/^:root\s*\{/m);
const lightSystem = block(/:root:not\(\[data-theme='dark'\]\)\s*\{/);
const lightChosen = block(/^:root\[data-theme='light'\]\s*\{/m);
const THEMES: Record<'dark' | 'light', Decls> = {
  dark,
  light: { ...dark, ...lightSystem },
};

/** Splits on top-level commas, so nested functions stay whole. */
function args(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let current = '';
  for (const char of text) {
    if (char === '(') depth++;
    if (char === ')') depth--;
    if (char === ',' && depth === 0) {
      parts.push(current.trim());
      current = '';
    } else current += char;
  }
  return [...parts, current.trim()];
}

/** Resolves hex, rgb(a), var() and srgb color-mix() to straight RGBA. */
function resolve(value: string, theme: Decls): Rgba {
  const text = value.trim();
  const variable = /^var\((--[\w-]+)\)$/.exec(text);
  if (variable) {
    const next = theme[variable[1]!];
    if (!next) throw new Error(`undefined token ${variable[1]}`);
    return resolve(next, theme);
  }
  if (text === 'transparent') return [0, 0, 0, 0];
  const hex = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(text);
  if (hex) {
    const n = parseInt(hex[1]!, 16);
    const alpha = hex[2] ? parseInt(hex[2], 16) / 255 : 1;
    return [n >> 16, (n >> 8) & 255, n & 255, alpha];
  }
  const rgb = /^rgba?\((.*)\)$/.exec(text);
  if (rgb) {
    const [r, g, b, a = '1'] = args(rgb[1]!);
    return [Number(r), Number(g), Number(b), Number(a)];
  }
  const mix = /^color-mix\(in srgb,(.*)\)$/.exec(text);
  if (mix) {
    const [first, second] = args(mix[1]!).map((part) => {
      const weighted = /^(.*?)\s+([\d.]+)%$/.exec(part);
      return weighted
        ? { color: weighted[1]!, weight: Number(weighted[2]) / 100 }
        : { color: part, weight: NaN };
    });
    const p = Number.isNaN(first!.weight) ? 1 - second!.weight : first!.weight;
    const a = resolve(first!.color, theme);
    const b = resolve(second!.color, theme);
    // Premultiplied interpolation, as CSS Color 5 specifies.
    const alpha = a[3] * p + b[3] * (1 - p);
    const channel = (i: number) =>
      alpha === 0 ? 0 : (a[i]! * a[3] * p + b[i]! * b[3] * (1 - p)) / alpha;
    return [channel(0), channel(1), channel(2), alpha];
  }
  throw new Error(`unsupported colour ${text}`);
}

/** `top` painted over `bottom`, layer after layer, on an opaque base. */
function paint(theme: Decls, ...layers: string[]): Rgba {
  return layers
    .map((layer) => resolve(`var(${layer})`, theme))
    .reduce((bottom, top) => {
      const a = top[3];
      return [0, 1, 2]
        .map((i) => top[i]! * a + bottom[i]! * (1 - a))
        .concat(1) as Rgba;
    });
}

function luminance([r, g, b]: Rgba): number {
  const linear = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

function contrast(foreground: Rgba, background: Rgba): number {
  const [high, low] = [luminance(foreground), luminance(background)].sort(
    (x, y) => y - x,
  );
  return (high! + 0.05) / (low! + 0.05);
}

const TEXT = ['--text', '--text-2', '--text-3', '--accent'];
const STATUS = ['--run', '--ok', '--fail', '--wait', '--stuck'];
const SURFACES = ['--surface', '--raised', '--hover'];
/**
 * Text on a tint: alerts, outcomes, confirmations, the danger button's hover,
 * accented buttons. Each tint is measured over an opaque surface; a tint drawn
 * inside another tint lays its own opaque base (`linear-gradient(tint, tint),
 * var(--raised)`), so these pairs are what the panel actually shows.
 */
const TINTED: [string, string][] = [
  ['--ok', '--ok-soft'],
  ['--run', '--run-soft'],
  ['--wait', '--wait-soft'],
  ['--fail', '--fail-soft'],
  ['--fail', '--fail-soft-hover'],
  // The unresolved-start warning inside a danger confirmation.
  ['--wait', '--fail-soft'],
  ['--text-2', '--accent-soft'],
  ['--accent', '--accent-soft'],
];

describe.each(Object.entries(THEMES))('%s theme contrast', (_, theme) => {
  it.each([...TEXT, ...STATUS])('%s reaches 4.5:1 on every surface', (fg) => {
    for (const surface of SURFACES) {
      const ratio = contrast(paint(theme, fg), paint(theme, surface));
      expect(ratio, `${fg} on ${surface}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it.each(['--accent', '--accent-hover'])(
    '--on-accent reaches 4.5:1 on %s',
    (fill) => {
      const ratio = contrast(paint(theme, '--on-accent'), paint(theme, fill));
      expect(ratio).toBeGreaterThanOrEqual(4.5);
    },
  );

  it.each(TINTED)('%s reaches 4.5:1 on %s', (fg, tint) => {
    for (const surface of ['--surface', '--raised']) {
      const ratio = contrast(paint(theme, fg), paint(theme, surface, tint));
      expect(ratio, `${fg} on ${tint} over ${surface}`).toBeGreaterThanOrEqual(
        4.5,
      );
    }
  });
});

describe('theme blocks', () => {
  it('the OS-light and the chosen-light blocks are identical', () => {
    expect(lightChosen).toEqual(lightSystem);
  });

  it('light overrides only tokens the dark set defines', () => {
    for (const name of Object.keys(lightSystem))
      expect(dark, name).toHaveProperty([name]);
  });

  it('light gives every literal dark colour a value of its own', () => {
    // Derived tokens (var(), color-mix()) follow by themselves; a literal one
    // without a light value would show its dark colour in the light theme.
    const literal = Object.keys(dark).filter((name) =>
      COLOUR_LITERAL.test(dark[name]!),
    );
    expect(literal.length).toBeGreaterThan(15);
    for (const name of literal)
      expect(lightSystem, name).toHaveProperty([name]);
  });
});

describe('colour literals live only in the tokens', () => {
  // 14-forced-colors.css is scanned too: it may use system colours only.
  const files = [
    ...readdirSync(STYLES)
      .filter((name) => name.endsWith('.css') && name !== '00-tokens.css')
      .map((name) => `src/styles/${name}`),
    ...readdirSync(LIB)
      .filter((name) => name.endsWith('.svelte'))
      .map((name) => `src/lib/${name}`),
    ...readdirSync(join(process.cwd(), 'src'))
      .filter((name) => name.endsWith('.svelte'))
      .map((name) => `src/${name}`),
  ];

  it('finds the style files and components', () => {
    expect(files).toContain('src/styles/14-forced-colors.css');
    expect(files).toContain('src/App.svelte');
    expect(files.length).toBeGreaterThan(60);
  });

  it.each(files)('%s reads tokens only', (file) => {
    const offending = readFileSync(join(process.cwd(), file), 'utf8')
      .split('\n')
      .filter((line) => COLOUR_LITERAL.test(line));
    expect(offending).toEqual([]);
  });
});
