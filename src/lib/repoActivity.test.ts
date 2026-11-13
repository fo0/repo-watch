import { describe, expect, it } from 'vitest';
import fixture from '../../fixtures/demo.json';
import { lastActivity, lastRun, repositoryActivity } from './repoActivity';
import type { Snapshot } from './types';

// Synthetic fixtures only.
const snapshot = () => structuredClone(fixture) as Snapshot;
const ids = (groups: ReturnType<typeof repositoryActivity>) =>
  groups.map(({ repo, loaded }) => [repo.id, loaded]);

describe('lastRun', () => {
  it('takes the latest start of a repository or one workflow and sees open runs', () => {
    const data = snapshot();
    // 8402 (Checks, queued) started last; 8401 (Desktop build) runs.
    expect(lastRun(data, 'demo-personal-kit')).toEqual({
      at: '2026-10-31T09:44:00Z',
      active: true,
    });
    expect(lastRun(data, 'demo-personal-kit', 101)).toEqual({
      at: '2026-10-31T09:42:00Z',
      active: true,
    });
    expect(lastRun(data, 'demo-personal-site', 301)).toEqual({
      at: '2026-10-31T09:35:00Z',
      active: false,
    });
    // A workflow of another repository with the same id never counts.
    expect(lastRun(data, 'demo-team-tools', 101)).toBeNull();
    expect(lastRun(data, 'demo-extra')).toBeNull();
    expect(lastRun(data, 'demo-extra', 401)).toBeNull();
  });

  it('uses the core’s lastRunAt when it is newer than the run history', () => {
    const data = snapshot();
    const sandbox = data.workflows.find((workflow) => workflow.id === 401)!;
    sandbox.lastRunAt = '2026-10-20T07:00:00Z';
    expect(lastRun(data, 'demo-extra', 401)).toEqual({
      at: '2026-10-20T07:00:00Z',
      active: false,
    });
    expect(lastRun(data, 'demo-extra')?.at).toBe('2026-10-20T07:00:00Z');
    // Older than a listed run, or unreadable: the runs speak.
    const desktop = data.workflows.find((workflow) => workflow.id === 101)!;
    desktop.lastRunAt = '2026-10-01T00:00:00Z';
    expect(lastRun(data, 'demo-personal-kit', 101)?.at).toBe(
      '2026-10-31T09:42:00Z',
    );
    desktop.lastRunAt = 'kein Datum';
    expect(lastRun(data, 'demo-personal-kit', 101)?.at).toBe(
      '2026-10-31T09:42:00Z',
    );
    const checks = data.workflows.find((workflow) => workflow.id === 102)!;
    checks.lastRunAt = '2026-11-01T06:00:00Z';
    // The repository counts every workflow, a workflow only itself.
    expect(lastRun(data, 'demo-personal-kit')?.at).toBe('2026-11-01T06:00:00Z');
    expect(lastRun(data, 'demo-personal-kit', 101)?.at).toBe(
      '2026-10-31T09:42:00Z',
    );
  });

  it('reports a scope as idle once its runs are completed', () => {
    const data = snapshot();
    for (const run of data.runs)
      Object.assign(run, { status: 'completed', conclusion: 'success' });
    expect(lastRun(data, 'demo-personal-kit')).toEqual({
      at: '2026-10-31T09:44:00Z',
      active: false,
    });
  });

  it('rides along with each loaded repository row', () => {
    const groups = repositoryActivity(snapshot(), '', '');
    const kit = groups.find(({ repo }) => repo.id === 'demo-personal-kit')!;
    expect(kit.last).toEqual({ at: '2026-10-31T09:44:00Z', active: true });
    expect(kit.runs.map((run) => run.id)).toEqual([8401, 8402, 8405]);
    const api = groups.find(({ repo }) => repo.id === 'demo-work-api')!;
    expect(api.last).toBeNull();
  });
});

describe('repository order', () => {
  const order = (data: Snapshot) =>
    repositoryActivity(data, '', '').map(({ repo }) => repo.id);

  it('puts the most recent run, run start or push first on every snapshot', () => {
    const data = snapshot();
    // Runs first, then the run-less repositories by their last push.
    expect(order(data).slice(0, 5)).toEqual([
      'demo-personal-kit',
      'demo-team-tools',
      'demo-personal-site',
      'demo-work-api',
      'demo-work-docs',
    ]);
    // A push newer than every run wins over an old run.
    data.repos.find((repo) => repo.id === 'demo-work-docs')!.pushedAt =
      '2026-11-01T08:00:00Z';
    expect(order(data)[0]).toBe('demo-work-docs');
  });

  it('orders a repository known only by Workflow.lastRunAt by that run', () => {
    const data = snapshot();
    const api = data.repos.find((repo) => repo.id === 'demo-work-api')!;
    api.pushedAt = null;
    // A synthetic workflow without a run in the history.
    data.workflows.push({
      ...data.workflows[0]!,
      id: 901,
      repoId: api.id,
      lastRunAt: '2026-10-31T09:40:00Z',
    });
    // After runner-tools (09:42) and before website (09:36).
    expect(order(data).slice(0, 4)).toEqual([
      'demo-personal-kit',
      'demo-team-tools',
      'demo-work-api',
      'demo-personal-site',
    ]);
  });

  it('breaks ties by owner/name and puts repositories without any time last', () => {
    const data = snapshot();
    for (const repo of data.repos) repo.pushedAt = 'kein Datum';
    data.runs = [];
    for (const workflow of data.workflows) workflow.lastRunAt = null;
    const groups = repositoryActivity(data, '', '');
    expect(groups.map(({ repo }) => `${repo.owner}/${repo.name}`)).toEqual(
      groups.map(({ repo }) => `${repo.owner}/${repo.name}`).sort(),
    );
    data.repos.find((repo) => repo.id === 'demo-work-api')!.pushedAt =
      '2026-10-01T00:00:00Z';
    data.repos.find((repo) => repo.id === 'demo-work-docs')!.pushedAt =
      '2026-10-01T00:00:00Z';
    const [first, second, third] = order(data);
    expect([first, second]).toEqual(['demo-work-api', 'demo-work-docs']);
    expect(third).not.toBe('demo-work-docs');
  });

  it('takes the newest readable time of a repository', () => {
    const at = (time: string) => Date.parse(time);
    expect(
      lastActivity({
        repo: { pushedAt: '2026-10-02T00:00:00Z' },
        latest: { updatedAt: '2026-10-01T00:00:00Z' },
        last: { at: '2026-10-03T00:00:00Z', active: false },
      }),
    ).toBe(at('2026-10-03T00:00:00Z'));
    expect(
      lastActivity({
        repo: { pushedAt: 'kein Datum' },
        latest: { updatedAt: '2026-10-01T00:00:00Z' },
        last: null,
      }),
    ).toBe(at('2026-10-01T00:00:00Z'));
    expect(
      lastActivity({ repo: { pushedAt: null }, latest: undefined, last: null }),
    ).toBe(-Infinity);
  });
});

describe('repository search', () => {
  it('reaches the account’s other repositories by owner/name, after every loaded match', () => {
    const data = snapshot();
    expect(ids(repositoryActivity(data, '', 'sandbox'))).toEqual([
      ['demo-extra', false],
    ]);
    // Loaded matches first; the rest by newest push: legacy (September)
    // before sandbox (August).
    const all = repositoryActivity(data, 'demo-work', 'demo');
    expect(all.filter(({ loaded }) => loaded)).toHaveLength(10);
    expect(ids(all.slice(-2))).toEqual([
      ['demo-work-legacy', false],
      ['demo-extra', false],
    ]);
    expect(ids(repositoryActivity(data, '', 'TEAM-DEMO/'))).toEqual([
      ['demo-extra', false],
    ]);
  });

  it('matches every word of the query in any order', () => {
    const groups = repositoryActivity(snapshot(), '', 'build launch-kit');
    expect(groups.map(({ repo }) => repo.id)).toEqual(['demo-personal-kit']);
    expect(groups[0]!.workflows.map((workflow) => workflow.name)).toEqual([
      'Desktop build',
    ]);
    // An unloaded repository needs every word in its owner/name.
    expect(ids(repositoryActivity(snapshot(), '', 'demo sandbox'))).toEqual([
      ['demo-extra', false],
    ]);
  });

  it('keeps the account filter and never lists them without a query', () => {
    const data = snapshot();
    expect(repositoryActivity(data, 'demo-personal', 'sandbox')).toEqual([]);
    for (const query of ['', '   '])
      expect(
        repositoryActivity(data, '', query).every(({ loaded }) => loaded),
      ).toBe(true);
    // The workflow name is no way in: unloaded repositories have none yet.
    expect(repositoryActivity(data, '', 'sandbox check')).toEqual([]);
  });

  it('shows nothing of an unloaded repository but its name and push', () => {
    const [found] = repositoryActivity(snapshot(), '', 'sandbox');
    // The demo fixture holds its workflow already; the row shows none.
    expect(found).toMatchObject({
      loaded: false,
      runs: [],
      workflows: [],
      latest: undefined,
      last: null,
      status: null,
      recent: [],
    });
    expect(found!.repo.pushedAt).toBe('2026-08-12T11:00:00Z');
  });

  it('turns into a regular row once the repository is loaded', () => {
    const data = snapshot();
    data.repos.find((repo) => repo.id === 'demo-extra')!.selected = true;
    const [group] = repositoryActivity(data, '', 'sandbox');
    expect(group?.loaded).toBe(true);
    expect(group?.workflows.map((workflow) => workflow.id)).toEqual([401]);
    expect(group?.recent).toHaveLength(8);
    // And it stays in the list without a query.
    expect(
      repositoryActivity(data, '', '').some(
        ({ repo }) => repo.id === 'demo-extra',
      ),
    ).toBe(true);
  });
});
