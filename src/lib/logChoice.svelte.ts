import type { Bridge } from './bridgeTypes';
import { LOG_DEFAULT, validLogLevel } from './logLevel';
import type { LogLevel } from './types';

type LogPort = Pick<Bridge, 'logLevel' | 'setLogLevel'>;

/**
 * The log level while the settings dialog is open. The choice only stages a
 * value; „Speichern“ writes it with `apply`.
 */
export class LogChoice {
  /** The level in effect; null until the first read finished. */
  saved = $state<LogLevel | null>(null);
  /** The staged level; null while untouched. */
  draft = $state<LogLevel | null>(null);
  #port: LogPort;

  constructor(port: LogPort) {
    this.#port = port;
  }

  /** What the control shows. */
  get value() {
    return this.draft ?? this.saved ?? LOG_DEFAULT;
  }
  get pending() {
    return this.draft !== null && this.draft !== this.saved;
  }

  stage = (level: LogLevel) => {
    this.draft = level === this.saved ? null : level;
  };

  /** Reads once; an unreadable or foreign value shows the default. */
  read = async () => {
    if (this.saved !== null) return;
    let level = LOG_DEFAULT;
    try {
      const value = await this.#port.logLevel();
      if (validLogLevel(value)) level = value;
    } catch {
      /* The default shows; saving still writes the chosen level. */
    }
    this.saved ??= level;
    if (this.draft === this.saved) this.draft = null;
  };

  /** Writes the staged level; a refusal rejects with the shell's user-facing message. */
  apply = async () => {
    const level = this.draft;
    if (level === null) return;
    this.saved = await this.#port.setLogLevel(level);
    this.draft = null;
  };
}
