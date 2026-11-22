import { Channel, invoke } from '@tauri-apps/api/core';
import type { Locale } from './i18n/locale';
import type {
  BrowserConfig,
  BrowserReply,
  DispatchOutcome,
  DispatchRequest,
  DownloadProgress,
  LogLevel,
  LocalDiscovery,
  Pin,
  Snapshot,
  StartupStatus,
  UncertainStart,
  UpdateInfo,
} from './types';

/**
 * Every command the native shell registers in `generate_handler!`
 * (src-tauri/src/desktop.rs), with its camelCase arguments and its result.
 * `args: undefined` means the command takes none; a Rust `Result<(), String>`
 * resolves as `void` and every `Err` rejects with its message in the language set by `set_locale`.
 */
export interface Commands {
  snapshot: { args: undefined; result: Snapshot };
  refresh_snapshot: { args: undefined; result: Snapshot };
  refresh_repositories: { args: undefined; result: Snapshot };
  /** Seconds between idle refreshes; an open run still polls every 15 s. */
  refresh_interval: { args: undefined; result: number };
  /** Validates (15–3600 s) and stores the interval; resolves with the saved value. */
  set_refresh_interval: { args: { seconds: number }; result: number };
  /** The level of the local log file; read without the engine lock. */
  log_level: { args: undefined; result: LogLevel };
  /** Stores the level; resolves with the saved value. */
  set_log_level: { args: { level: LogLevel }; result: LogLevel };
  /** Creates the log folder if it is missing and shows it in the file manager. */
  open_log_folder: { args: undefined; result: void };
  /** The language in effect for native texts: errors, notifications, tray. */
  set_locale: { args: { locale: Locale }; result: void };
  save_pin: { args: { pin: Pin }; result: Snapshot };
  delete_pin: { args: { pinId: string }; result: Snapshot };
  dispatch_workflow: {
    args: { request: DispatchRequest };
    result: DispatchOutcome;
  };
  /** Releases exactly this claim after the user checked GitHub; never dispatches. */
  release_uncertain_start: {
    args: { start: UncertainStart };
    result: Snapshot;
  };
  /** Asks GitHub to cancel an open run once; the next poll reports the outcome. */
  cancel_run: { args: { repoId: string; runId: number }; result: Snapshot };
  /** Loads a repository found by the search, so its workflows can be used. */
  open_repository: { args: { repoId: string }; result: Snapshot };
  /** Hides a repository from the lists or shows it again; the selection applies at once. */
  set_repo_hidden: {
    args: { accountId: string; repoId: string; hidden: boolean };
    result: Snapshot;
  };
  connect_token: { args: { token: string }; result: Snapshot };
  browser_auth_config: { args: undefined; result: BrowserConfig };
  start_browser_auth: { args: undefined; result: BrowserReply };
  poll_browser_auth: { args: { flowId: number }; result: BrowserReply };
  open_browser_auth: { args: { flowId: number }; result: void };
  open_browser_installation: { args: undefined; result: void };
  discover_local_auth: { args: undefined; result: LocalDiscovery };
  connect_local_auth: {
    args: { login: string; confirmImport: boolean };
    result: Snapshot;
  };
  cancel_connect: { args: undefined; result: void };
  disconnect_account: { args: { accountId: string }; result: Snapshot };
  autostart_status: { args: undefined; result: StartupStatus };
  set_autostart: { args: { enabled: boolean }; result: StartupStatus };
  open_startup_settings: { args: undefined; result: void };
  /** `null` when the installed version is current. */
  check_update: { args: undefined; result: UpdateInfo | null };
  /** Downloads, verifies and starts the checked update; RepoWatch then exits and restarts. */
  install_update: {
    args: { onProgress: Channel<DownloadProgress> };
    result: void;
  };
  open_run: { args: { url: string }; result: void };
  /** Opens a repository page (`safeRepoUrl`); the shell re-validates it. */
  open_repo: { args: { url: string }; result: void };
  /** Registered for the shell; the panel itself never hides the window. */
  hide_panel: { args: undefined; result: void };
}

export type Command = keyof Commands;
type Args<K extends Command> = Commands[K]['args'];

/** Runtime list of the map's keys; the record type keeps it exhaustive and exact. */
const REGISTERED: Record<Command, true> = {
  snapshot: true,
  refresh_snapshot: true,
  refresh_repositories: true,
  refresh_interval: true,
  set_refresh_interval: true,
  set_locale: true,
  log_level: true,
  set_log_level: true,
  open_log_folder: true,
  save_pin: true,
  delete_pin: true,
  dispatch_workflow: true,
  release_uncertain_start: true,
  cancel_run: true,
  open_repository: true,
  set_repo_hidden: true,
  connect_token: true,
  browser_auth_config: true,
  start_browser_auth: true,
  poll_browser_auth: true,
  open_browser_auth: true,
  open_browser_installation: true,
  discover_local_auth: true,
  connect_local_auth: true,
  cancel_connect: true,
  disconnect_account: true,
  autostart_status: true,
  set_autostart: true,
  open_startup_settings: true,
  check_update: true,
  install_update: true,
  open_run: true,
  open_repo: true,
  hide_panel: true,
};
export const COMMAND_NAMES: readonly string[] = Object.keys(REGISTERED);

/** The only way the panel reaches the native shell: one typed Tauri command. */
export function call<K extends Command>(
  command: K,
  ...[args]: Args<K> extends undefined ? [] : [args: Args<K>]
): Promise<Commands[K]['result']> {
  return invoke<Commands[K]['result']>(command, args);
}

/** A channel for a command's streamed messages, e.g. `install_update` progress. */
export function channel<T>(onMessage: (message: T) => void): Channel<T> {
  return new Channel<T>(onMessage);
}
