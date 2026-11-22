import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createDemoBridge, type Bridge, type DemoControls } from './demo';
import { call } from './ipc';
import type { Snapshot } from './types';

export const native = isTauri();
let storage: Storage | undefined;
try {
  storage = globalThis.localStorage;
} catch {
  /* Private browser modes can deny storage. */
}

/** Native commands; the shell has no demo scenario or reset command. */
const nativeBridge: Bridge = {
  snapshot: () => call('snapshot'),
  refreshSnapshot: () => call('refresh_snapshot'),
  savePin: (pin) => call('save_pin', { pin }),
  deletePin: (pinId) => call('delete_pin', { pinId }),
  dispatchWorkflow: (request) => call('dispatch_workflow', { request }),
  releaseUncertainStart: (start) => call('release_uncertain_start', { start }),
  cancelRun: (repoId, runId) => call('cancel_run', { repoId, runId }),
  openRepository: (repoId) => call('open_repository', { repoId }),
  setRepoHidden: (accountId, repoId, hidden) =>
    call('set_repo_hidden', { accountId, repoId, hidden }),
  refreshInterval: () => call('refresh_interval'),
  setRefreshInterval: (seconds) => call('set_refresh_interval', { seconds }),
  logLevel: () => call('log_level'),
  setLogLevel: (level) => call('set_log_level', { level }),
  openLogFolder: () => call('open_log_folder'),
  setLocale: (locale) => call('set_locale', { locale }),
};

export const bridge: Bridge & Partial<DemoControls> = native
  ? nativeBridge
  : createDemoBridge(storage);

/** Native owns polling; browser polls its local simulation without network traffic. */
export async function observeSnapshots(
  callback: (snapshot: Snapshot) => void,
): Promise<() => void> {
  if (native) {
    try {
      return await listen<Snapshot>('repowatch:snapshot', (event) =>
        callback(event.payload),
      );
    } catch {
      /* A shell without event permissions can still read native snapshots. */
    }
  }
  const timer = setInterval(() => {
    void bridge
      .snapshot()
      .then(callback)
      .catch(() => {
        /* Explicit refresh reports connection errors. */
      });
  }, 1000);
  return () => clearInterval(timer);
}
