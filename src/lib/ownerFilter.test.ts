import { describe, expect, it } from 'vitest';
import { createDemoBridge, type Bridge, type PreferenceStorage } from './demo';
import { OwnerChoice } from './ownerChoice.svelte';
import {
  OWNERS_KEY,
  isFilteredOut,
  ownerRows,
  type OwnerRow,
} from './ownerFilter';
import { Panel } from './panel.svelte';
import { repositoryActivity } from './repoActivity';
import { visibleRuns } from './runs';
import type { Snapshot } from './types';

const memory = (seed: Record<string, string> = {}) => {
  const values = new Map(Object.entries(seed));
  const storage: PreferenceStorage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => void values.set(key, value),
    removeItem: (key) => void values.delete(key),
  };
  return { storage, values };
};
const flush = () => new Promise((done) => setTimeout(done, 0));
const selected = (snapshot: Snapshot) =>
  snapshot.repos.filter((repo) => repo.selected).map((repo) => repo.id);
/** demo-work (login team-demo) holds example-org's repositories and one of its own. */
const WORK = 'demo-work';
/** example-org's newest repository, with an open run (#8403). */
const TOOLS = 'demo-team-tools';
/** team-demo's only repository: no place among the 10 newest until filtered. */
const OWN = 'demo-extra';

describe('isFilteredOut', () => {
  const filters = [{ accountId: 'a', owners: ['Fixture-Org'] }];
  it('leaves out other owners of a filtered account, case-insensitively', () => {
    expect(
      isFilteredOut(filters, { accountId: 'a', owner: 'fixture-org' }),
    ).toBe(false);
    expect(isFilteredOut(filters, { accountId: 'a', owner: 'someone' })).toBe(
      true,
    );
    expect(isFilteredOut(filters, { accountId: 'b', owner: 'someone' })).toBe(
      false,
    );
    expect(
      isFilteredOut([{ accountId: 'a', owners: [] }], {
        accountId: 'a',
        owner: 'someone',
      }),
    ).toBe(false);
    expect(isFilteredOut(undefined, { accountId: 'a', owner: 'x' })).toBe(
      false,
    );
  });
});

describe('ownerRows', () => {
  it('lists accounts with several owners, the login first, then by name', async () => {
    const snapshot = await createDemoBridge(undefined, 0).snapshot();
    expect(ownerRows(snapshot)).toEqual([
      {
        accountId: WORK,
        login: 'team-demo',
        owners: ['team-demo', 'example-org'],
        watched: ['team-demo', 'example-org'],
      },
    ]);
    expect(ownerRows(null)).toEqual([]);
  });

  it('adds owners of the stored filter and merges spellings', () => {
    const snapshot = {
      accounts: [{ id: 'a', login: 'Me', label: '' }],
      repos: [
        { accountId: 'a', owner: 'zeta' },
        { accountId: 'a', owner: 'me' },
        { accountId: 'a', owner: 'Alpha' },
        { accountId: 'a', owner: 'ZETA' },
      ],
      ownerFilters: [{ accountId: 'a', owners: ['ZETA', 'gone'] }],
    } as unknown as Snapshot;
    expect(ownerRows(snapshot)).toEqual([
      {
        accountId: 'a',
        login: 'Me',
        owners: ['me', 'Alpha', 'gone', 'zeta'],
        watched: ['gone', 'zeta'],
      },
    ]);
  });
});

describe('demo bridge', () => {
  it('filters at once, frees the places, persists and watches all again', async () => {
    const { storage, values } = memory();
    const bridge = createDemoBridge(storage, 0);
    const before = await bridge.snapshot();
    expect(selected(before)).toContain(TOOLS);
    expect(selected(before)).not.toContain(OWN);

    const filtered = await bridge.setOwnerFilter(WORK, ['Team-Demo']);
    expect(filtered.ownerFilters).toEqual([
      { accountId: WORK, owners: ['Team-Demo'] },
    ]);
    expect(selected(filtered)).toEqual([
      'demo-personal-kit',
      'demo-personal-site',
      OWN,
    ]);
    expect(JSON.parse(values.get(OWNERS_KEY)!)).toHaveLength(1);
    const restarted = await createDemoBridge(storage, 0).snapshot();
    expect(restarted.ownerFilters).toEqual(filtered.ownerFilters);
    expect(selected(restarted)).toEqual(selected(filtered));

    const all = await bridge.setOwnerFilter(WORK, []);
    expect(all.ownerFilters).toBeUndefined();
    expect(selected(all)).toEqual(selected(before));
    expect(values.has(OWNERS_KEY)).toBe(false);
  });

  it('refuses an unknown account and ignores corrupt storage', async () => {
    const bridge = createDemoBridge(memory().storage, 0);
    await expect(bridge.setOwnerFilter('missing', ['x'])).rejects.toThrow(
      'Konto nicht gefunden.',
    );
    const corrupt = memory({
      [OWNERS_KEY]: '[{"accountId":"demo-work","owners":[]},{"owners":[1]},"x"',
    }).storage;
    expect(
      (await createDemoBridge(corrupt, 0).snapshot()).ownerFilters,
    ).toBeUndefined();
  });

  it('keeps a repository opened from the search unless the filter leaves it out', async () => {
    const bridge = createDemoBridge(memory().storage, 0);
    // example-org's 11th newest: only opening it from the search loads it.
    const legacy = 'demo-work-legacy';
    await bridge.openRepository(legacy);
    const kept = await bridge.setOwnerFilter(WORK, ['example-org']);
    expect(selected(kept)).toContain(legacy);
    expect(selected(kept)).not.toContain(OWN);
    const dropped = await bridge.setOwnerFilter(WORK, ['team-demo']);
    expect(selected(dropped)).not.toContain(legacy);
  });

  it('a reset watches every owner again', async () => {
    const { storage, values } = memory();
    const bridge = createDemoBridge(storage, 0);
    await bridge.setOwnerFilter(WORK, ['team-demo']);
    expect(selected(await bridge.resetDemo())).toContain(TOOLS);
    expect(values.has(OWNERS_KEY)).toBe(false);
  });
});

describe('panel lists', () => {
  it('leave out a filtered owner and its runs; the search finds it unmarked', async () => {
    const bridge = createDemoBridge(undefined, 0);
    const snapshot = await bridge.setOwnerFilter(WORK, ['team-demo']);
    // A favorite keeps a filtered repository selected; the lists still skip it.
    const pinned: Snapshot = {
      ...snapshot,
      repos: snapshot.repos.map((repo) =>
        repo.id === TOOLS ? { ...repo, selected: true } : repo,
      ),
    };
    for (const value of [snapshot, pinned]) {
      const ids = repositoryActivity(value, '', '').map((g) => g.repo.id);
      expect(ids).toContain(OWN);
      expect(ids).not.toContain(TOOLS);
      expect(
        visibleRuns(value, '', '', 'all').map((run) => run.repoId),
      ).not.toContain(TOOLS);
      const found = repositoryActivity(value, '', 'runner-tools');
      expect(found.map((g) => [g.repo.id, g.hidden])).toEqual([[TOOLS, false]]);
      expect(found[0]!.loaded).toBe(value === pinned);
    }
  });
});

describe('Actions.setOwners', () => {
  async function setup(overrides: Partial<Bridge> = {}) {
    const bridge = { ...createDemoBridge(undefined, 0), ...overrides };
    const panel = new Panel({
      bridge,
      native: false,
      observe: async () => () => {},
    });
    panel.load();
    await flush();
    return panel;
  }

  it('accepts the new selection and rejects with the shell message', async () => {
    const panel = await setup();
    await panel.actions.setOwners(WORK, ['team-demo']);
    expect(panel.snapshot!.ownerFilters).toHaveLength(1);
    const refused = await setup({
      setOwnerFilter: async () => {
        throw new Error('Einstellung konnte nicht gespeichert werden.');
      },
    });
    await expect(refused.actions.setOwners(WORK, [])).rejects.toThrow(
      'Einstellung konnte nicht gespeichert werden.',
    );
  });
});

describe('OwnerChoice', () => {
  const row = (watched = ['me', 'org', 'other']): OwnerRow => ({
    accountId: 'a',
    login: 'me',
    owners: ['me', 'org', 'other'],
    watched,
  });

  it('stages toggles and sends only the accounts that changed', async () => {
    const choice = new OwnerChoice();
    const rows = [row()];
    expect(choice.pendingIn(rows)).toBe(false);
    choice.toggle(rows[0]!, 'ORG');
    expect(choice.has(rows[0]!, 'org')).toBe(false);
    expect(choice.pendingIn(rows)).toBe(true);
    // Toggling back is no change.
    choice.toggle(rows[0]!, 'org');
    expect(choice.pendingIn(rows)).toBe(false);
    choice.toggle(rows[0]!, 'other');
    const sent: [string, string[]][] = [];
    const set = async (accountId: string, owners: string[]) =>
      void sent.push([accountId, owners]);
    await choice.apply(rows, set);
    expect(sent).toEqual([['a', ['me', 'org']]]);
    expect(choice.pendingIn(rows)).toBe(false);
  });

  it('sends no owners when all are watched again', async () => {
    const choice = new OwnerChoice();
    const rows = [row(['org'])];
    choice.toggle(rows[0]!, 'me');
    choice.toggle(rows[0]!, 'other');
    const sent: string[][] = [];
    await choice.apply(rows, async (_, owners) => void sent.push(owners));
    expect(sent).toEqual([[]]);
  });

  it('never releases the last watched owner', () => {
    const choice = new OwnerChoice();
    const only = row(['org']);
    expect(choice.locked(only, 'org')).toBe(true);
    expect(choice.locked(only, 'me')).toBe(false);
    choice.toggle(only, 'org');
    expect(choice.has(only, 'org')).toBe(true);
    expect(choice.pendingIn([only])).toBe(false);
  });

  it('keeps the account staged after a failure', async () => {
    const choice = new OwnerChoice();
    const rows = [row()];
    choice.toggle(rows[0]!, 'org');
    await expect(
      choice.apply(rows, async () => {
        throw new Error('Konto nicht gefunden.');
      }),
    ).rejects.toThrow('Konto nicht gefunden.');
    expect(choice.pendingIn(rows)).toBe(true);
  });
});
