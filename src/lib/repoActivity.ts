// Repository rows: activity, search reach and when each last ran.
import { isHidden } from './hiddenRepos';
import { fullRepo } from './model';
import { isActive, recentRuns, repositoryStatus } from './runs';
import { matchesTerms, searchTerms } from './search';
import type { Repo, Run, Snapshot, Workflow } from './types';

/**
 * The repository browser includes selected repos even before their first
 * run, most recently active first (`lastActivity`), recomputed with every
 * snapshot. A search also reaches the account's other repositories by
 * `owner/name`, after every selected match and newest push first: they carry
 * no workflows or runs until opened (`loaded: false`). A hidden repository
 * shows only in a search (ADR-0008), marked `hidden`.
 */
export function repositoryActivity(
  snapshot: Snapshot,
  accountId: string,
  query: string,
) {
  const terms = searchTerms(query);
  const searching = terms.length > 0;
  const ofAccount = snapshot.repos.filter(
    (repo) => !accountId || repo.accountId === accountId,
  );
  const hidden = (repo: Repo) => isHidden(snapshot.hiddenRepos, repo);
  const loaded = ofAccount
    .filter((repo) => repo.selected && (searching || !hidden(repo)))
    .map((repo) => {
      const runs = snapshot.runs.filter((run) => run.repoId === repo.id);
      const latest = [...runs].sort(
        (a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt),
      )[0];
      const all = snapshot.workflows.filter(
        (workflow) => workflow.repoId === repo.id,
      );
      const workflows = all.filter((workflow) =>
        matchesTerms(`${fullRepo(repo)} ${workflow.name}`, terms),
      );
      return {
        repo,
        hidden: hidden(repo),
        loaded: true,
        runs,
        workflows,
        latest,
        last: lastRun({ runs, workflows: all }, repo.id),
        status: repositoryStatus(runs, latest),
        recent: recentRuns({ runs }, repo.id),
      };
    })
    .filter(
      ({ repo, workflows }) =>
        matchesTerms(fullRepo(repo), terms) || workflows.length > 0,
    )
    .sort(
      (a, b) =>
        lastActivity(b) - lastActivity(a) ||
        fullRepo(a.repo).localeCompare(fullRepo(b.repo)),
    );
  if (!searching) return loaded;
  const found = ofAccount
    .filter((repo) => !repo.selected && matchesTerms(fullRepo(repo), terms))
    .sort(
      (a, b) =>
        (Date.parse(b.pushedAt ?? '') || 0) -
          (Date.parse(a.pushedAt ?? '') || 0) ||
        fullRepo(a).localeCompare(fullRepo(b)),
    )
    .map((repo) => ({
      repo,
      hidden: hidden(repo),
      loaded: false,
      runs: [] as Run[],
      workflows: [] as Workflow[],
      latest: undefined as Run | undefined,
      last: null,
      status: null,
      recent: [] as (Run | null)[],
    }));
  return [...loaded, ...found];
}

/**
 * A loaded repository's most recent activity (ms since the epoch): the
 * newest of its latest run's update, its last run start (`lastRun`, which
 * knows `Workflow.lastRunAt`) and its last push. Missing or unreadable
 * times count as absent; a repository with none sorts last.
 */
export function lastActivity(group: {
  repo: Pick<Repo, 'pushedAt'>;
  latest: Pick<Run, 'updatedAt'> | undefined;
  last: LastRun | null;
}): number {
  return Math.max(
    -Infinity,
    ...[group.latest?.updatedAt, group.last?.at, group.repo.pushedAt]
      .map((value) => Date.parse(value ?? ''))
      .filter((time) => !Number.isNaN(time)),
  );
}

/** When a repository or one workflow last ran, and whether it runs now. */
export interface LastRun {
  /** Start (ISO 8601) of the latest known run. */
  at: string;
  /** A run of this scope is open right now. */
  active: boolean;
}

/**
 * The latest run start of a repository, or of one of its workflows: the
 * newer of the snapshot's runs and `Workflow.lastRunAt`, which the core
 * looks up for workflows the run history no longer holds. Null while no run
 * is known.
 */
export function lastRun(
  snapshot: Pick<Snapshot, 'runs' | 'workflows'>,
  repoId: string,
  workflowId?: number,
): LastRun | null {
  const inScope = (itemRepo: string, itemWorkflow: number) =>
    itemRepo === repoId &&
    (workflowId === undefined || itemWorkflow === workflowId);
  let at: string | null = null;
  let newest = -Infinity;
  let active = false;
  const consider = (value: string | null | undefined) => {
    const time = Date.parse(value ?? '');
    if (time > newest) [newest, at] = [time, value!];
  };
  for (const run of snapshot.runs)
    if (inScope(run.repoId, run.workflowId)) {
      consider(run.createdAt);
      active ||= isActive(run);
    }
  for (const workflow of snapshot.workflows)
    if (inScope(workflow.repoId, workflow.id)) consider(workflow.lastRunAt);
  return at === null ? null : { at, active };
}
