// Run lists, status and duration for the activity and repository views.
import type { IconName } from './Icon.svelte';
import { isHidden } from './hiddenRepos';
import { m } from './i18n/index.svelte';
import { fullRepo } from './model';
import { matchesTerms, searchTerms } from './search';
import type { Run, Snapshot } from './types';

export type ActivityFilter = 'all' | 'active' | 'failed' | 'recent';
/** Open runs: queued, running or waiting. A stuck run is history, not open. */
export const isActive = (run: Run) =>
  run.status === 'queued' ||
  run.status === 'in_progress' ||
  run.status === 'waiting';
export const HISTORY_LIMIT = 50;

/** Keep every open run visible, with a bounded chronological completed history. */
export function historyRuns(
  snapshot: Snapshot,
  accountId: string,
  query: string,
  filter: ActivityFilter,
) {
  let completed = 0;
  return visibleRuns(snapshot, accountId, query, filter).filter(
    (run) => isActive(run) || ++completed <= HISTORY_LIMIT,
  );
}

/** A repository's name without its owner, as run rows show it. */
export const shortRepo = (repo: string) =>
  repo.slice(repo.lastIndexOf('/') + 1);

/**
 * How a run row is named, repository first: every repository may call its
 * workflow „Release“, so the repository tells the rows apart, e.g.
 * `example-org/runner-tools-demo · Runner health #8403`.
 */
export const runLabel = (run: Run, repo: string) =>
  `${repo} · ${run.name} #${run.id}`;

export function statusLabel(run: Run): string {
  const status = m().runs.status;
  if (run.status !== 'completed') return status[run.status];
  return status[run.conclusion ?? 'cancelled'];
}

/**
 * How a run reads at a glance: one colour and one icon per class, in the
 * same classes as `statusLabel`. The service already folds GitHub's
 * timed-out, startup-failure and action-required outcomes into `failure`, and
 * skipped, neutral and stale ones into `cancelled`; a run queued for more
 * than 24 h arrives as `stuck`.
 */
export type RunTone =
  'run' | 'wait' | 'queue' | 'ok' | 'fail' | 'cancel' | 'stuck';

export function runTone(run: Run): RunTone {
  if (run.status === 'in_progress') return 'run';
  if (run.status === 'waiting') return 'wait';
  if (run.status === 'queued') return 'queue';
  if (run.status === 'stuck') return 'stuck';
  if (run.conclusion === 'success') return 'ok';
  return run.conclusion === 'failure' ? 'fail' : 'cancel';
}

/** The status icon of each tone, as the Verlauf rows draw it. */
export const TONE_ICON: Record<RunTone, IconName> = {
  run: 'activity',
  wait: 'pause',
  queue: 'clock',
  ok: 'check',
  fail: 'close',
  cancel: 'close',
  stuck: 'warning',
};

/**
 * The status line of a repository row. Open runs come first, so a running
 * build is never hidden behind a later update: more than one reads
 * „2 aktiv“, one names its own status; otherwise the latest run's status.
 */
export function repositoryStatus(
  runs: readonly Run[],
  latest: Run | undefined,
): { tone: RunTone; label: string } | null {
  const active = runs.filter(isActive);
  if (active.length > 1)
    return {
      tone: active.some((run) => run.status === 'in_progress')
        ? 'run'
        : runTone(active[0]!),
      label: m().runs.active(active.length),
    };
  const run = active[0] ?? latest;
  return run ? { tone: runTone(run), label: statusLabel(run) } : null;
}

/** Cells of the run strip: how many runs each repository row shows. */
export const STRIP_CELLS = 8;

/**
 * The run strip of one repository: its last `n` runs by start time, oldest
 * first and newest last, padded at the start with `null` while it has fewer.
 */
export function recentRuns(
  snapshot: Pick<Snapshot, 'runs'>,
  repoId: string,
  n = STRIP_CELLS,
): (Run | null)[] {
  const recent = snapshot.runs
    .filter((run) => run.repoId === repoId)
    .sort(
      (a, b) =>
        Date.parse(b.createdAt) - Date.parse(a.createdAt) || b.id - a.id,
    )
    .slice(0, n)
    .reverse();
  return [...Array<null>(Math.max(0, n - recent.length)).fill(null), ...recent];
}

/** The order in which the summary names the tones. */
const TONE_ORDER: readonly RunTone[] = [
  'ok',
  'fail',
  'cancel',
  'stuck',
  'run',
  'wait',
  'queue',
];

/**
 * What a run strip shows, in words: the strip is colour only, e.g.
 * „Letzte 8 Runs: 6 erfolgreich, 1 fehlgeschlagen, 1 läuft“.
 */
export function recentSummary(cells: readonly (Run | null)[]): string {
  const runs = cells.filter((run): run is Run => run !== null);
  const counts = new Map<RunTone, number>();
  for (const run of runs)
    counts.set(runTone(run), (counts.get(runTone(run)) ?? 0) + 1);
  const text = m().runs.strip;
  const tones = TONE_ORDER.filter((tone) => counts.has(tone));
  if (runs.length === 0) return text.none;
  if (runs.length === 1) return text.last(text.tones[tones[0]!][0]);
  return text.lastN(
    runs.length,
    tones
      .map((tone) => {
        const count = counts.get(tone)!;
        const [one, many] = text.tones[tone];
        return `${count} ${count === 1 ? one : many}`;
      })
      .join(', '),
  );
}

export function visibleRuns(
  snapshot: Snapshot,
  accountId: string,
  query: string,
  filter: ActivityFilter,
): Run[] {
  const terms = searchTerms(query);
  // A hidden repository's runs stay out of every run list (ADR-0008).
  const repos = new Map(
    snapshot.repos
      .filter(
        (r) =>
          r.selected &&
          (!accountId || r.accountId === accountId) &&
          !isHidden(snapshot.hiddenRepos, r),
      )
      .map((r) => [r.id, r]),
  );
  return snapshot.runs
    .filter((run) => {
      const repo = repos.get(run.repoId);
      if (
        !repo ||
        (filter === 'active' && !isActive(run)) ||
        (filter === 'failed' && run.conclusion !== 'failure')
      )
        return false;
      if (filter === 'recent' && isActive(run)) return false;
      return matchesTerms(`${fullRepo(repo)} ${run.name} ${run.branch}`, terms);
    })
    .sort(
      (a, b) =>
        Number(isActive(b)) - Number(isActive(a)) ||
        Date.parse(b.createdAt) - Date.parse(a.createdAt),
    );
}

/** Run time as `m:ss min` (the same in every language), e.g. `0:42 min`. */
export function duration(run: Run): string {
  const seconds = Math.max(
    0,
    Math.round((Date.parse(run.updatedAt) - Date.parse(run.createdAt)) / 1000),
  );
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')} min`;
}
