import fixture from '../fixtures/demo.json' with { type: 'json' };
import { expect, test, type Page } from './support/test';
import { chooseAccount, openPage } from './support/demo';
import { installTauriMock } from './support/tauriMock';

// The last push is asserted as a date; a fixed zone keeps it stable.
test.use({ timezoneId: 'UTC' });

const card = (page: Page, name: string) =>
  page.getByRole('region', { name, exact: true });
const favorites = (page: Page) =>
  page.getByRole('region', { name: 'Favoriten', exact: true });

test.describe('demo', () => {
  test.beforeEach(async ({ page }) => {
    // Runs on from here; the demo answers after its latency.
    await page.clock.install({ time: Date.parse('2026-11-03T10:00:00Z') });
    await page.goto('/');
    await expect(
      page.getByRole('button', { name: 'Einstellungen', exact: true }),
    ).toBeEnabled();
  });

  test('a search matches every word in any order', async ({ page }) => {
    const search = page.getByLabel('Übersicht durchsuchen');
    // Workflow first, then part of the repository name.
    await search.fill('health runner-tools');
    await expect(page.locator('.repository-card')).toHaveCount(1);
    await expect(card(page, 'example-org/runner-tools-demo')).toBeVisible();
    // A word that matches nothing empties the list.
    await search.fill('health website');
    await expect(page.locator('.repository-card')).toHaveCount(0);
  });

  test('a search finds a repository outside the automatic ones; opening it loads its workflows for a favorite', async ({
    page,
  }) => {
    const search = page.getByLabel('Übersicht durchsuchen');
    const sandbox = card(page, 'team-demo/sandbox-demo');
    await expect(sandbox).toHaveCount(0);
    await search.fill('sandbox');
    await expect(page.locator('.repository-card')).toHaveCount(1);
    // Name and last push only: no run strip, workflow count or status.
    const pushed = sandbox.locator('.repository-meta time');
    await expect(sandbox.locator('.repository-meta')).toHaveText(
      'Push 12.08.2026',
    );
    await expect(pushed).toHaveAttribute('datetime', '2026-08-12T11:00:00Z');
    await expect(pushed).toHaveAttribute(
      'title',
      'Letzter Push: 12.08.2026, 11:00 · Demo-Zeit',
    );
    await expect(
      sandbox.locator('.run-strip, .repository-count, .repository-status'),
    ).toHaveCount(0);

    const toggle = sandbox.locator('.repository-toggle');
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    const heart = sandbox.getByRole('button', {
      name: 'Sandbox check als Favorit speichern',
      exact: true,
    });
    await expect(heart).toBeEnabled();
    // Loaded, it reads like every other row.
    await expect(sandbox.locator('.run-strip')).toHaveCount(1);
    await expect(sandbox.locator('.repository-count')).toHaveText('1 Workflow');
    await expect(sandbox.locator('.repository-status')).toHaveText(
      'Noch keine Runs',
    );
    await heart.click();
    await expect(
      sandbox.getByRole('button', {
        name: 'Sandbox check aus Favoriten entfernen',
        exact: true,
      }),
    ).toHaveAttribute('aria-pressed', 'true');
    await expect(
      favorites(page).locator('.pin-row', { hasText: 'sandbox-demo' }),
    ).toHaveCount(1);

    // Clearing the search keeps the favorite and the repository.
    await search.fill('');
    await expect(
      favorites(page).locator('.pin-row', { hasText: 'sandbox-demo' }),
    ).toHaveCount(1);
    await openPage(page, 'repos');
    await expect(sandbox.locator('.run-strip')).toHaveCount(1);
    await openPage(page, 'favorites');
    await expect(
      page.locator('.pin-row', { hasText: 'sandbox-demo' }),
    ).toHaveCount(1);
  });

  test('every match is listed after the loaded ones, newest push first, within the account', async ({
    page,
  }) => {
    await openPage(page, 'repos');
    const search = page.getByLabel('Repositories durchsuchen');
    await search.fill('team-demo/');
    await expect(page.locator('.repository-card')).toHaveCount(1);
    await search.fill('-demo');
    const names = page.locator('.repository-card .repository-name');
    await expect(names).toHaveCount(14);
    await expect(names.nth(12)).toHaveText('example-org/legacy-portal-demo');
    await expect(names.nth(13)).toHaveText('team-demo/sandbox-demo');
    // A read-only repository loads like any other.
    const legacy = card(page, 'example-org/legacy-portal-demo');
    await legacy.locator('.repository-toggle').click();
    await expect(legacy.locator('.repository-tag')).toHaveText('Nur Lesen');
    await expect(legacy.getByText('Keine passenden Workflows.')).toBeVisible();
    // Another account never reaches them.
    await chooseAccount(page, 'demo-dev');
    await search.fill('sandbox');
    await expect(page.locator('.repository-card')).toHaveCount(0);
    await expect(
      page.getByRole('heading', {
        name: 'Keine passenden Repositories',
        exact: true,
      }),
    ).toBeVisible();
  });
});

test.describe('native', () => {
  type OpenMock = { fail: string };
  const NOW = Date.parse('2026-11-01T09:00:00Z');

  async function openNativeSearch(page: Page, fail = '') {
    await page.clock.install({ time: NOW - 5_000 });
    await page.addInitScript(() =>
      localStorage.setItem(
        'repowatch.activeAccount.v1',
        JSON.stringify('demo-personal'),
      ),
    );
    const mock = await installTauriMock<{ fixture: typeof fixture }, OpenMock>(
      page,
      ({ fixture }, state) => {
        const base = structuredClone(fixture);
        const hidden = {
          ...base.repos[0]!,
          id: 'native-archive',
          name: 'archive-tools-demo',
          pushedAt: '2026-09-01T08:00:00Z',
          selected: false,
        };
        const workflow = {
          ...base.workflows[0]!,
          id: 901,
          repoId: hidden.id,
          name: 'Archive sync',
          inputs: [],
          refs: ['main'],
          lastRunAt: '2026-10-29T08:00:00Z',
        };
        const live = (loaded: boolean) => ({
          ...base,
          mode: 'live',
          stale: false,
          connectivity: 'online',
          message: undefined,
          repos: [...base.repos, { ...hidden, selected: loaded }],
          workflows: loaded ? [...base.workflows, workflow] : base.workflows,
        });
        let loaded = false;
        const open = window as unknown as { releaseOpen?: () => void };
        return (cmd) => {
          if (cmd === 'snapshot' || cmd === 'refresh_snapshot')
            return structuredClone(live(loaded));
          if (cmd !== 'open_repository') return null;
          if (state.fail) throw state.fail;
          // Answers once the spec releases it, so the loading state shows.
          return new Promise((resolve) => {
            open.releaseOpen = () => {
              loaded = true;
              resolve(structuredClone(live(true)));
            };
          });
        };
      },
      { arg: { fixture }, state: { fail } },
    );
    await page.goto('/');
    await page.clock.pauseAt(NOW);
    await page.getByLabel('Übersicht durchsuchen').fill('archive');
    return mock;
  }

  test('the row says it loads, then shows the workflows and their last run', async ({
    page,
  }) => {
    await openNativeSearch(page);
    const row = card(page, 'demo-dev/archive-tools-demo');
    await expect(row.locator('.repository-meta')).toHaveText('Push 01.09.2026');
    const toggle = row.locator('.repository-toggle');
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    const body = row.locator('.repository-actions');
    await expect(body).toHaveAttribute('aria-busy', 'true');
    await expect(body.locator('.repository-no-actions')).toHaveText(
      'Workflows werden geladen …',
    );
    // The info line comes first, already while the workflows load.
    await expect(body.locator('.repository-info time')).toHaveText(
      'Push 01.09.2026',
    );
    await expect(
      body.getByRole('link', {
        name: 'demo-dev/archive-tools-demo auf GitHub öffnen',
        exact: true,
      }),
    ).toBeVisible();
    await page.evaluate(() =>
      (window as unknown as { releaseOpen: () => void }).releaseOpen(),
    );
    const sync = row.getByRole('button', {
      name: 'Archive sync · main',
      exact: true,
    });
    await expect(sync.locator('small')).toHaveText('main · vor 3 Tagen');
    await expect(sync).toHaveAccessibleDescription(
      'Archive sync · main · Letzter Run: 29.10.2026, 08:00',
    );
    await expect(body).not.toHaveAttribute('aria-busy', 'true');
    // The core's lookup dates the row; the history has no run of it.
    await expect(row.locator('.repository-meta time')).toHaveText(
      'vor 3 Tagen',
    );
    await expect(row.locator('.repository-status')).toHaveCount(0);
    await expect(row.locator('.run-strip')).toHaveCount(1);
  });

  test('a failed load closes the row again and says why', async ({ page }) => {
    await openNativeSearch(page, 'Repository konnte nicht geladen werden.');
    const row = card(page, 'demo-dev/archive-tools-demo');
    const toggle = row.locator('.repository-toggle');
    await toggle.click();
    await expect(
      page
        .locator('.notice-banner')
        .filter({ hasText: 'Repository konnte nicht geladen werden.' }),
    ).toBeVisible();
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await expect(row.locator('.run-strip')).toHaveCount(0);
  });
});
