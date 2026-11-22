import type { PreferenceStorage } from './bridgeTypes';
import type { LogLevel } from './types';

/** Mirrors `Level` in src-tauri/src/app_log.rs; info is the default. */
export const LOG_LEVELS = ['info', 'debug'] as const;
export const LOG_DEFAULT: LogLevel = 'info';
export const LOG_KEY = 'repowatch.demo.log.v1';

export function validLogLevel(level: unknown): level is LogLevel {
  return LOG_LEVELS.some((known) => known === level);
}

/**
 * The browser demo's level: kept in memory and, when allowed, in storage.
 * The demo writes no log file, so there is no folder to open.
 */
export function demoLog(storage?: PreferenceStorage) {
  let level = LOG_DEFAULT;
  try {
    const stored: unknown = JSON.parse(storage?.getItem(LOG_KEY) ?? 'null');
    if (validLogLevel(stored)) level = stored;
  } catch {
    /* Corrupt or unavailable storage keeps the default. */
  }
  return {
    logLevel: () => Promise.resolve(level),
    setLogLevel(next: LogLevel): Promise<LogLevel> {
      level = validLogLevel(next) ? next : LOG_DEFAULT;
      try {
        storage?.setItem(LOG_KEY, JSON.stringify(level));
      } catch {
        /* The value still applies for this session. */
      }
      return Promise.resolve(level);
    },
    openLogFolder: () => Promise.resolve(),
  };
}
