import { afterEach, describe, expect, it } from 'vitest';
import fixture from '../../fixtures/demo.json';
import { setLocale } from './i18n/index.svelte';
import { fullRepo } from './model';
import { repositoryActivity } from './repoActivity';
import {
  duration,
  historyRuns,
  HISTORY_LIMIT,
  isActive,
  recentRuns,
  recentSummary,
  repositoryStatus,
  runLabel,
  runTone,
  shortRepo,
  statusLabel,
  visibleRuns,
} from './runs';
import type { Run, Snapshot } from './types';

const snapshot = () => structuredClone(fixture) as Snapshot;

describe('activity model', () => {
  it('orders repositories by latest activity, including completed runs, and keeps unused repositories', () => {
    const data = snapshot();
    data.repos[3]!.selected = true;
    data.runs[3]!.updatedAt = '2026-10-31T10:00:00Z';
    // Without push times, repositories without runs follow by name:
    // example-org/…, then team-demo/…. Push order: repoActivity.test.ts.
    const ran = new Set(data.runs.map((run) => run.repoId));
    for (const repo of data.repos) if (!ran.has(repo.id)) repo.pushedAt = null;
    const unused = [
      ...[
        'api',
        'billing',
        'ci',
        'design',
        'docs',
        'infra',
        'metrics',
        'mobile',
        'search',
      ].map((name) => `demo-work-${name}`),
      'demo-extra',
    ];
    expect(repositoryActivity(data, '', '').map(({ repo }) => repo.id)).toEqual(
      ['demo-personal-site', 'demo-personal-kit', 'demo-team-tools', ...unused],
    );
    expect(
      repositoryActivity(data, 'demo-work', '').map(({ repo }) => repo.id),
    ).toEqual(['demo-team-tools', ...unused]);
    expect(
      repositoryActivity(data, '', 'desktop build').map(({ workflows }) =>
        workflows.map((w) => w.id),
      ),
    ).toEqual([[101]]);
    expect(repositoryActivity(data, '', 'not-found')).toEqual([]);
  });
  it('limits completed history across repositories without evicting older open runs', () => {
    const data = snapshot();
    const completed = data.runs.find((run) => run.status === 'completed')!;
    data.runs.push(
      ...Array.from({ length: 80 }, (_, index) => ({
        ...completed,
        id: 10000 + index,
        createdAt: new Date(Date.UTC(2026, 10, 1, 0, index)).toISOString(),
      })),
    );
    const history = historyRuns(data, '', '', 'all');
    expect(history.filter(isActive)).toHaveLength(3);
    expect(history.filter((run) => !isActive(run))).toHaveLength(HISTORY_LIMIT);
    expect(history.at(-1)?.id).toBe(10030);
    expect(
      historyRuns(data, 'demo-work', '', 'all').map((run) => run.repoId),
    ).toEqual(['demo-team-tools']);
  });
  it('filters selected repos, full owner/repo, accounts and activity states', () => {
    const data = snapshot();
    expect(visibleRuns(data, '', '', 'active')).toHaveLength(3);
    expect(
      visibleRuns(data, 'demo-work', '', 'all').map((run) => run.repoId),
    ).toEqual(['demo-team-tools']);
    expect(
      visibleRuns(data, '', 'example-org/runner-tools-demo', 'all'),
    ).toHaveLength(1);
    expect(
      visibleRuns(data, '', 'DEVELOP', 'all').map((run) => run.id),
    ).toEqual([8402]);
    // Every word must match, in any order: branch, workflow, repository.
    expect(
      visibleRuns(data, '', 'develop launch-kit', 'all').map((run) => run.id),
    ).toEqual([8402]);
    expect(
      visibleRuns(data, '', 'main  desktop', 'all').map((run) => run.id),
    ).toEqual([8401]);
    expect(visibleRuns(data, '', 'main develop', 'all')).toEqual([]);
    expect(visibleRuns(data, '', '', 'failed').map((run) => run.id)).toEqual([
      8404,
    ]);
    expect(
      visibleRuns(data, '', '', 'recent').every((run) => !isActive(run)),
    ).toBe(true);
    data.repos[0]!.selected = false;
    expect(
      visibleRuns(data, 'demo-personal', '', 'all').map((run) => run.id),
    ).toEqual([8404]);
  });
  it('sorts active first, then by start date and reports truthful status', () => {
    const data = snapshot();
    data.runs[4]!.createdAt = '2030-01-01T00:00:00Z';
    expect(visibleRuns(data, '', '', 'all').map((run) => run.id)).toEqual([
      8402, 8401, 8403, 8405, 8404,
    ]);
    expect(statusLabel(data.runs[1]!)).toBe('In Warteschlange');
    expect(statusLabel(data.runs[3]!)).toBe('Fehlgeschlagen');
    expect(duration(data.runs[0]!)).toBe('1:00 min');
    expect(fullRepo(data.repos[1]!)).toBe('example-org/runner-tools-demo');
  });
  it('names a run by its repository first, then workflow and number', () => {
    const data = snapshot();
    const repo = fullRepo(data.repos[1]!);
    expect(shortRepo(repo)).toBe('runner-tools-demo');
    expect(shortRepo('Repository nicht verfügbar')).toBe(
      'Repository nicht verfügbar',
    );
    expect(runLabel(data.runs[2]!, repo)).toBe(
      'example-org/runner-tools-demo · Runner health #8403',
    );
  });
  it.each([
    ['queued', null, 'In Warteschlange'],
    ['in_progress', null, 'Läuft'],
    ['waiting', null, 'Wartet'],
    ['completed', 'success', 'Erfolgreich'],
    ['completed', 'failure', 'Fehlgeschlagen'],
    ['completed', 'cancelled', 'Abgebrochen'],
    ['completed', null, 'Abgebrochen'],
    ['stuck', null, 'Hängt'],
  ] satisfies [Run['status'], Run['conclusion'], string][])(
    'labels status %s with conclusion %s as %s',
    (status, conclusion, label) => {
      const run = { ...snapshot().runs[0]!, status, conclusion };
      expect(statusLabel(run)).toBe(label);
    },
  );
  it.each([
    [59, '0:59 min'],
    [0, '0:00 min'],
    [65, '1:05 min'],
    [725, '12:05 min'],
    [-30, '0:00 min'], // updatedAt before createdAt (clock skew) clamps to zero
  ])('formats a run of %i seconds as %s', (seconds, label) => {
    const createdAt = '2026-10-31T09:00:00Z';
    const updatedAt = new Date(Date.parse(createdAt) + seconds * 1000);
    expect(
      duration({
        ...snapshot().runs[0]!,
        createdAt,
        updatedAt: updatedAt.toISOString(),
      }),
    ).toBe(label);
  });
});

describe('run strip', () => {
  const run = (id: number, createdAt: string, patch: Partial<Run> = {}) => ({
    ...snapshot().runs[4]!,
    id,
    repoId: 'strip-repo',
    createdAt,
    updatedAt: createdAt,
    ...patch,
  });
  const ids = (cells: (Run | null)[]) => cells.map((cell) => cell?.id ?? null);

  it('pads the demo repositories with empty cells, newest run last', () => {
    const data = snapshot();
    expect(ids(recentRuns(data, 'demo-personal-kit'))).toEqual([
      null,
      null,
      null,
      null,
      null,
      8405,
      8401,
      8402,
    ]);
    expect(ids(recentRuns(data, 'demo-personal-site'))).toEqual([
      ...Array(7).fill(null),
      8404,
    ]);
    expect(recentRuns(data, 'demo-extra')).toEqual(Array(8).fill(null));
    // The repository rows carry their strip.
    expect(
      ids(
        repositoryActivity(data, '', '').find(
          ({ repo }) => repo.id === 'demo-team-tools',
        )!.recent,
      ),
    ).toEqual([...Array(7).fill(null), 8403]);
  });

  it('names open runs first in the repository status line', () => {
    const data = snapshot();
    data.repos[3]!.selected = true;
    const status = (repoId: string) =>
      repositoryActivity(data, '', '').find(({ repo }) => repo.id === repoId)!
        .status;
    // One running and one queued: both count, the running tone leads.
    expect(status('demo-personal-kit')).toEqual({
      tone: 'run',
      label: '2 aktiv',
    });
    expect(status('demo-team-tools')).toEqual({
      tone: 'wait',
      label: 'Wartet',
    });
    expect(status('demo-personal-site')).toEqual({
      tone: 'fail',
      label: 'Fehlgeschlagen',
    });
    expect(status('demo-extra')).toBeNull();
    // A run completed later does not hide the only open one.
    Object.assign(data.runs[0]!, {
      status: 'completed',
      conclusion: 'success',
    });
    data.runs[4]!.updatedAt = '2026-10-31T10:00:00Z';
    expect(status('demo-personal-kit')).toEqual({
      tone: 'queue',
      label: 'In Warteschlange',
    });
    // Nothing open: the latest update speaks.
    Object.assign(data.runs[1]!, {
      status: 'completed',
      conclusion: 'failure',
    });
    expect(status('demo-personal-kit')).toEqual({
      tone: 'ok',
      label: 'Erfolgreich',
    });
    // Open runs without a running one keep the first one's tone.
    const open = (status: Run['status']) => ({
      ...data.runs[0]!,
      status,
      conclusion: null,
    });
    expect(
      repositoryStatus([open('queued'), open('waiting')], undefined),
    ).toEqual({ tone: 'queue', label: '2 aktiv' });
  });

  it('keeps the last runs by start time, not by update, ties by run number', () => {
    const runs = Array.from({ length: 12 }, (_, index) =>
      run(
        100 + index,
        new Date(Date.UTC(2026, 10, 1, 8, index)).toISOString(),
        // Older runs updated last must not push newer starts out.
        {
          updatedAt: new Date(
            Date.UTC(2026, 10, 2, 0, 12 - index),
          ).toISOString(),
        },
      ),
    );
    runs.push(run(99, runs[11]!.createdAt));
    expect(ids(recentRuns({ runs }, 'strip-repo'))).toEqual([
      105, 106, 107, 108, 109, 110, 99, 111,
    ]);
    expect(ids(recentRuns({ runs }, 'strip-repo', 3))).toEqual([110, 99, 111]);
    expect(recentRuns({ runs }, 'other-repo', 2)).toEqual([null, null]);
  });

  it.each([
    ['in_progress', null, 'run'],
    ['waiting', null, 'wait'],
    ['queued', null, 'queue'],
    ['completed', 'success', 'ok'],
    ['completed', 'failure', 'fail'],
    ['completed', 'cancelled', 'cancel'],
    ['completed', null, 'cancel'],
    ['stuck', null, 'stuck'],
  ] satisfies [Run['status'], Run['conclusion'], string][])(
    'classes status %s with conclusion %s as %s, like its label',
    (status, conclusion, tone) => {
      expect(runTone({ ...snapshot().runs[0]!, status, conclusion })).toBe(
        tone,
      );
    },
  );

  it('sums the strip up in words', () => {
    const at = '2026-11-01T08:00:00Z';
    const done = (id: number, conclusion: Run['conclusion']) =>
      run(id, at, { status: 'completed', conclusion });
    expect(recentSummary(Array(8).fill(null))).toBe('Noch keine Runs');
    expect(recentSummary([null, done(1, 'failure')])).toBe(
      'Letzter Run: fehlgeschlagen',
    );
    expect(
      recentSummary([
        ...[1, 2, 3, 4, 5, 6].map((id) => done(id, 'success')),
        done(7, 'failure'),
        run(8, at, { status: 'in_progress', conclusion: null }),
      ]),
    ).toBe('Letzte 8 Runs: 6 erfolgreich, 1 fehlgeschlagen, 1 läuft');
    expect(
      recentSummary([
        null,
        run(1, at, { status: 'queued', conclusion: null }),
        run(2, at, { status: 'in_progress', conclusion: null }),
        run(3, at, { status: 'in_progress', conclusion: null }),
        run(4, at, { status: 'waiting', conclusion: null }),
        done(5, 'cancelled'),
      ]),
    ).toBe(
      'Letzte 5 Runs: 1 abgebrochen, 2 laufen, 1 wartet, 1 in Warteschlange',
    );
    expect(recentSummary(recentRuns(snapshot(), 'demo-personal-kit'))).toBe(
      'Letzte 3 Runs: 1 erfolgreich, 1 läuft, 1 in Warteschlange',
    );
  });

  describe('in English', () => {
    afterEach(() => setLocale('de'));
    it('names status and the strip summary in English', () => {
      setLocale('en');
      const at = '2026-11-01T08:00:00Z';
      const done = (id: number, conclusion: Run['conclusion']) =>
        run(id, at, { status: 'completed', conclusion });
      expect(statusLabel(done(1, 'failure'))).toBe('Failed');
      expect(recentSummary([null])).toBe('No runs yet');
      expect(recentSummary([done(1, 'success')])).toBe('Last run: succeeded');
      expect(
        recentSummary([
          done(1, 'success'),
          run(2, at, { status: 'in_progress', conclusion: null }),
          run(3, at, { status: 'in_progress', conclusion: null }),
        ]),
      ).toBe('Last 3 runs: 1 succeeded, 2 running');
    });
  });
});
