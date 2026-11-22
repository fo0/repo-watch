import { describe, expect, it } from 'vitest';
import { createDemoBridge, type PreferenceStorage } from './demo';
import { LogChoice } from './logChoice.svelte';
import { demoLog, LOG_KEY, validLogLevel } from './logLevel';
import type { LogLevel } from './types';

function memory(seed: Record<string, string> = {}): PreferenceStorage & {
  items: Map<string, string>;
} {
  const items = new Map(Object.entries(seed));
  return {
    items,
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => void items.set(key, value),
    removeItem: (key) => void items.delete(key),
  };
}

describe('log level', () => {
  it('knows info and debug only', () => {
    expect(validLogLevel('info') && validLogLevel('debug')).toBe(true);
    for (const value of ['trace', 'INFO', '', null, 1])
      expect(validLogLevel(value)).toBe(false);
  });

  it('defaults to info in the demo and keeps a saved level', async () => {
    const storage = memory();
    const demo = demoLog(storage);
    expect(await demo.logLevel()).toBe('info');
    expect(await demo.setLogLevel('debug')).toBe('debug');
    expect(storage.items.get(LOG_KEY)).toBe('"debug"');
    expect(await demoLog(storage).logLevel()).toBe('debug');
    // A corrupt or foreign stored value keeps the default.
    expect(await demoLog(memory({ [LOG_KEY]: '"trace"' })).logLevel()).toBe(
      'info',
    );
    expect(await demoLog(memory({ [LOG_KEY]: '{' })).logLevel()).toBe('info');
    await expect(demo.openLogFolder()).resolves.toBeUndefined();
  });

  it('is served by the demo bridge', async () => {
    const bridge = createDemoBridge(memory(), 0);
    expect(await bridge.logLevel()).toBe('info');
    expect(await bridge.setLogLevel('debug')).toBe('debug');
    expect(await bridge.logLevel()).toBe('debug');
  });
});

describe('staged log level', () => {
  const port = (level: unknown, written: LogLevel[] = []) => ({
    written,
    logLevel: () => Promise.resolve(level as LogLevel),
    setLogLevel: (next: LogLevel) => {
      written.push(next);
      return Promise.resolve(next);
    },
  });

  it('stages a choice and writes it only on apply', async () => {
    const shell = port('info');
    const choice = new LogChoice(shell);
    await choice.read();
    expect([choice.value, choice.pending]).toEqual(['info', false]);
    choice.stage('debug');
    expect([choice.value, choice.pending]).toEqual(['debug', true]);
    expect(shell.written).toEqual([]);
    // Back on the saved level, nothing is pending.
    choice.stage('info');
    expect(choice.pending).toBe(false);
    choice.stage('debug');
    await choice.apply();
    expect(shell.written).toEqual(['debug']);
    expect([choice.saved, choice.value, choice.pending]).toEqual([
      'debug',
      'debug',
      false,
    ]);
    await choice.apply();
    expect(shell.written).toEqual(['debug']);
  });

  it('shows info for an unreadable or foreign level', async () => {
    const foreign = new LogChoice(port('trace'));
    await foreign.read();
    expect(foreign.saved).toBe('info');
    const failing = new LogChoice({
      logLevel: () => Promise.reject(new Error('fixture')),
      setLogLevel: (next) => Promise.resolve(next),
    });
    await failing.read();
    expect(failing.saved).toBe('info');
  });

  it('keeps the choice staged when the shell refuses it', async () => {
    const choice = new LogChoice({
      logLevel: () => Promise.resolve('info'),
      setLogLevel: () => Promise.reject(new Error('fixture refusal')),
    });
    await choice.read();
    choice.stage('debug');
    await expect(choice.apply()).rejects.toThrow('fixture refusal');
    expect([choice.saved, choice.pending]).toEqual(['info', true]);
  });
});
