// The bridge contract: what the native shell and the browser demo both serve.
import type { Locale } from './i18n/locale';
import type {
  DemoScenario,
  DispatchOutcome,
  DispatchRequest,
  LogLevel,
  Pin,
  Snapshot,
  UncertainStart,
} from './types';

/** The snapshot and start commands both the native shell and the demo serve. */
export interface Bridge {
  snapshot(): Promise<Snapshot>;
  refreshSnapshot(): Promise<Snapshot>;
  savePin(pin: Pin): Promise<Snapshot>;
  deletePin(pinId: string): Promise<Snapshot>;
  dispatchWorkflow(request: DispatchRequest): Promise<DispatchOutcome>;
  /** Explicit user action after checking GitHub; releases exactly this claim, never dispatches. */
  releaseUncertainStart(start: UncertainStart): Promise<Snapshot>;
  /** Asks to cancel an open run; the outcome arrives with a later snapshot. */
  cancelRun(repoId: string, runId: number): Promise<Snapshot>;
  /** Loads a repository the search found outside the automatic set. */
  openRepository(repoId: string): Promise<Snapshot>;
  /** Hides a repository from the lists or shows it again. */
  setRepoHidden(
    accountId: string,
    repoId: string,
    hidden: boolean,
  ): Promise<Snapshot>;
  /** Seconds between idle refreshes. */
  refreshInterval(): Promise<number>;
  /** Stores a validated interval (15–3600 s); rejects with a localized message. */
  setRefreshInterval(seconds: number): Promise<number>;
  /** The level of the local log file. */
  logLevel(): Promise<LogLevel>;
  /** Stores the level; it applies at once. */
  setLogLevel(level: LogLevel): Promise<LogLevel>;
  /** Shows the log folder in the file manager; the demo has none. */
  openLogFolder(): Promise<void>;
  /** Tells the native side the language in effect for its own texts. */
  setLocale(locale: Locale): Promise<void>;
}
/** Demo-only controls; the native shell registers no matching command. */
export interface DemoControls {
  setDemoScenario(scenario: DemoScenario): Promise<Snapshot>;
  resetDemo(): Promise<Snapshot>;
}
export interface PreferenceStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}
