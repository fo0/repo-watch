import fixture from '../../fixtures/demo.json';
import { autoSelect } from './autoRepos';
import { demoHidden } from './hiddenRepos';
import { m } from './i18n/index.svelte';
import { demoLog } from './logLevel';
import { safePin, targetKey, validateDispatch } from './model';
import { demoRefresh } from './refresh';
import type { Bridge, DemoControls, PreferenceStorage } from './bridgeTypes';
import type {
  DemoScenario,
  DispatchOutcome,
  DispatchRequest,
  HiddenRepo,
  Pin,
  Snapshot,
  UncertainStart,
} from './types';

export type { Bridge, DemoControls, PreferenceStorage } from './bridgeTypes';
export const PREFERENCE_KEY = 'repowatch.demo.preferences.v1';
const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const fingerprint = (request: DispatchRequest) =>
  JSON.stringify([
    targetKey(request),
    Object.entries(request.inputs).sort(([a], [b]) => a.localeCompare(b)),
  ]);

/** The fixture under the native rule: each account's newest repositories. */
function initial(pins?: Pin[], hidden: HiddenRepo[] = []): Snapshot {
  const data = copy(fixture) as Snapshot;
  if (pins) data.pins = pins;
  data.repos = autoSelect(data.repos, data.pins, hidden);
  return data;
}

export function createDemoBridge(
  storage?: PreferenceStorage,
  latency = 300,
): Bridge & DemoControls {
  const hidden = demoHidden(storage);
  let data = initial(undefined, hidden.list);
  let scenario: DemoScenario = 'normal';
  let nextRun = 9000;
  let generation = 0;
  const pending = new Set<string>();
  const uncertain = new Map<string, UncertainStart>();
  const claims = new Map<string, string>();
  const outcomes = new Map<string, DispatchOutcome>();
  const simulated = new Map<number, number>();
  const delay = () =>
    new Promise<void>((resolve) => setTimeout(resolve, latency));
  try {
    const stored = JSON.parse(storage?.getItem(PREFERENCE_KEY) ?? 'null');
    if (stored && Array.isArray(stored.pins)) {
      // A saved pin may belong to any fixture repository; it keeps it shown.
      const validation = copy(data);
      validation.repos.forEach((r) => {
        r.selected = true;
      });
      data = initial(
        stored.pins.flatMap((pin: Pin) => {
          try {
            return [safePin(validation, pin)];
          } catch {
            return [];
          }
        }),
        hidden.list,
      );
    }
  } catch {
    /* Corrupt or unavailable preferences never prevent startup. */
  }
  const persist = () => {
    try {
      storage?.setItem(PREFERENCE_KEY, JSON.stringify({ pins: data.pins }));
    } catch {
      /* An in-memory demo remains usable with storage disabled. */
    }
  };
  const snapshot = (): Snapshot => {
    if (scenario !== 'offline') {
      for (const run of data.runs) {
        const started = simulated.get(run.id);
        if (started === undefined) continue;
        const elapsed = Date.now() - started;
        const status =
          elapsed >= 2000
            ? 'completed'
            : elapsed >= 1000
              ? 'in_progress'
              : 'queued';
        if (run.status !== status) {
          run.status = status;
          run.conclusion = status === 'completed' ? 'success' : null;
          run.updatedAt = new Date().toISOString();
          data.updatedAt = run.updatedAt;
        }
        if (status === 'completed') simulated.delete(run.id);
      }
    }
    return copy({
      ...data,
      connectivity:
        scenario === 'offline'
          ? 'offline'
          : scenario === 'forbidden'
            ? 'forbidden'
            : 'online',
      stale: scenario === 'offline',
      uncertainStarts: [...uncertain.values()],
      ...(hidden.list.length ? { hiddenRepos: hidden.list } : {}),
      message:
        scenario === 'offline'
          ? m().demo.offline
          : scenario === 'forbidden'
            ? m().demo.forbidden
            : undefined,
    });
  };
  const reject = (message: string): DispatchOutcome => ({
    state: 'rejected',
    message,
  });
  return {
    async snapshot() {
      return snapshot();
    },
    async refreshSnapshot() {
      await delay();
      if (scenario !== 'offline') data.updatedAt = new Date().toISOString();
      return snapshot();
    },
    async savePin(pin) {
      const safe = safePin(data, copy(pin));
      data.pins = [...data.pins.filter((p) => p.id !== safe.id), safe];
      persist();
      return snapshot();
    },
    async deletePin(pinId) {
      data.pins = data.pins.filter((p) => p.id !== pinId);
      persist();
      return snapshot();
    },
    async dispatchWorkflow(input) {
      const request = copy(input);
      const key = targetKey(request);
      const signature = fingerprint(request);
      const claim = claims.get(request.requestId);
      if (claim && claim !== signature) return reject(m().demo.requestReused);
      const previous = outcomes.get(request.requestId);
      if (previous) return copy(previous);
      const validation = validateDispatch(data, request);
      if (validation) return reject(validation);
      if (uncertain.has(key))
        return {
          state: 'uncertain',
          message: m().demo.stillUncertain,
        };
      if (pending.has(key)) return reject(m().demo.pending);
      claims.set(request.requestId, signature);
      pending.add(key);
      const ticket = generation;
      try {
        await delay();
        if (ticket !== generation) return reject(m().demo.wasReset);
        const changed = validateDispatch(data, request);
        if (changed) return reject(changed);
        if (scenario === 'offline') return reject(m().demo.offlineStart);
        if (scenario === 'forbidden') return reject(m().demo.forbiddenStart);
        if (scenario === 'uncertain') {
          const { accountId, repoId, workflowId, ref } = request;
          uncertain.set(key, {
            accountId,
            repoId,
            workflowId,
            ref,
            claimedAt: new Date().toISOString(),
          });
          const outcome: DispatchOutcome = {
            state: 'uncertain',
            message: m().demo.uncertain,
          };
          outcomes.set(request.requestId, outcome);
          return copy(outcome);
        }
        const workflow = data.workflows.find(
          (w) => w.id === request.workflowId && w.repoId === request.repoId,
        )!;
        const runId = ++nextRun;
        const now = new Date().toISOString();
        data.runs.unshift({
          id: runId,
          repoId: request.repoId,
          workflowId: request.workflowId,
          name: workflow.name,
          branch: request.ref,
          status: 'queued',
          conclusion: null,
          createdAt: now,
          updatedAt: now,
          htmlUrl: '',
        });
        simulated.set(runId, Date.now());
        data.updatedAt = now;
        const outcome: DispatchOutcome = {
          state: 'accepted',
          runId,
          message: m().demo.accepted(runId),
        };
        outcomes.set(request.requestId, outcome);
        return copy(outcome);
      } finally {
        if (ticket === generation) pending.delete(key);
      }
    },
    async cancelRun(repoId, runId) {
      await delay();
      const run = data.runs.find((r) => r.id === runId && r.repoId === repoId);
      const repo = data.repos.find((r) => r.id === repoId && r.selected);
      if (!run || !repo) throw new Error(m().demo.runNotFound);
      if (run.status === 'completed') throw new Error(m().demo.runFinished);
      if (repo.permission !== 'write') throw new Error(m().demo.noWriteAccess);
      if (scenario === 'offline') throw new Error(m().demo.offlineCancel);
      if (scenario === 'forbidden') throw new Error(m().demo.forbiddenCancel);
      simulated.delete(run.id);
      run.status = 'completed';
      run.conclusion = 'cancelled';
      run.updatedAt = new Date().toISOString();
      data.updatedAt = run.updatedAt;
      return snapshot();
    },
    async openRepository(repoId) {
      await delay();
      const repo = data.repos.find((r) => r.id === repoId);
      if (!repo) throw new Error(m().demo.repoNotFound);
      repo.selected = true;
      return snapshot();
    },
    async setRepoHidden(accountId, repoId, hide) {
      data.repos = hidden.change(data.repos, accountId, repoId, hide, (list) =>
        autoSelect(data.repos, data.pins, list),
      );
      return snapshot();
    },
    async releaseUncertainStart(start) {
      const key = targetKey(start);
      const current = uncertain.get(key);
      if (current && current.claimedAt !== start.claimedAt)
        throw new Error(m().demo.newerLock);
      uncertain.delete(key);
      return snapshot();
    },
    ...demoRefresh(storage),
    ...demoLog(storage),
    // The demo has no native texts; the panel switches by itself.
    setLocale: () => Promise.resolve(),
    async setDemoScenario(next) {
      scenario = next;
      return snapshot();
    },
    async resetDemo() {
      generation++;
      hidden.reset();
      data = initial();
      scenario = 'normal';
      nextRun = 9000;
      pending.clear();
      uncertain.clear();
      outcomes.clear();
      claims.clear();
      simulated.clear();
      try {
        storage?.removeItem(PREFERENCE_KEY);
      } catch {
        /* Storage may be unavailable. */
      }
      return snapshot();
    },
  };
}
