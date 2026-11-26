import fixture from '../fixtures/demo.json' with { type: 'json' };
import { expect, test, type Page } from './support/test';
import { expectPage, openPage } from './support/demo';
import { menuButton, repoRow } from './support/nativeRepos';
import { installTauriMock, readCalls } from './support/tauriMock';

/**
 * The owner filter (#108) in the browser demo, which stores it. The demo's
 * work account (login team-demo) holds example-org's repositories and one of
 * its own.
 */
const FILTERED = 'example-org/runner-tools-demo';
/** team-demo's only repository: it moves up once example-org is left out. */
const NEXT = 'team-demo/sandbox-demo';

const dialog = (page: Page) =>
  page.getByRole('dialog', { name: 'Einstellungen', exact: true });
const section = (page: Page) =>
  dialog(page).getByRole('region', { name: 'Organisationen' });
const owner = (page: Page, name: string) =>
  section(page)
    .getByRole('group', { name: 'Organisationen von team-demo' })
    .getByRole('button', { name, exact: true });
const save = (page: Page) =>
  dialog(page).getByRole('button', { name: 'Speichern', exact: true });
async function openApp(page: Page) {
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await dialog(page).getByRole('tab', { name: 'App' }).click();
}
/** Its open run in „Läuft gerade“. */
const liveRun = (page: Page) =>
  page.getByRole('button', {
    name: /^example-org\/runner-tools-demo · Runner health #8403 · /,
  });
const runsOf = (page: Page, repoId: string) =>
  page.locator(`[data-run^="${repoId}:"]`);

test('„Organisationen“ stages owners, keeps one and drops discarded changes', async ({
  page,
}) => {
  await page.goto('/');
  await openPage(page, 'repos');
  await openApp(page);
  // Only the work account has two owners: one row, its login first.
  await expect(section(page).getByRole('group')).toHaveCount(1);
  await expect(section(page).getByRole('button')).toHaveText([
    'team-demo',
    'example-org',
  ]);
  // The section sits directly above „Ausgeblendete Repositories“.
  await expect(
    dialog(page).locator('.owner-setting + .hidden-setting'),
  ).toHaveCount(1);
  await expect(owner(page, 'example-org')).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await expect(save(page)).toHaveAttribute('aria-disabled', 'true');

  await owner(page, 'example-org').click();
  await expect(owner(page, 'example-org')).toHaveAttribute(
    'aria-pressed',
    'false',
  );
  await expect(save(page)).not.toHaveAttribute('aria-disabled');
  // The last watched owner cannot be released.
  const own = owner(page, 'team-demo');
  await expect(own).toHaveAttribute('aria-disabled', 'true');
  await own.click({ force: true });
  await expect(own).toHaveAttribute('aria-pressed', 'true');

  // „Abbrechen“ drops the change.
  await dialog(page)
    .getByRole('button', { name: 'Abbrechen', exact: true })
    .click();
  await expect(repoRow(page, FILTERED)).toBeVisible();
  await expect(repoRow(page, NEXT)).toHaveCount(0);
  await openApp(page);
  await expect(owner(page, 'example-org')).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await expect(owner(page, 'team-demo')).not.toHaveAttribute('aria-disabled');
});

test('„Speichern“ leaves an owner out of the lists, also after a reload; the search still finds it', async ({
  page,
}) => {
  await page.goto('/');
  await expect(liveRun(page)).toBeVisible();
  await openPage(page, 'repos');
  await expect(repoRow(page, FILTERED)).toBeVisible();
  await expect(repoRow(page, NEXT)).toHaveCount(0);

  await openApp(page);
  await owner(page, 'example-org').click();
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(repoRow(page, FILTERED)).toHaveCount(0);
  await expect(repoRow(page, NEXT)).toBeVisible();
  // The personal account keeps its repositories.
  await expect(repoRow(page, 'demo-dev/launch-kit-demo')).toBeVisible();

  await page.reload();
  await openPage(page, 'overview');
  await expect(liveRun(page)).toHaveCount(0);
  await openPage(page, 'runs');
  await expect(runsOf(page, 'demo-personal-kit')).not.toHaveCount(0);
  await expect(runsOf(page, 'demo-team-tools')).toHaveCount(0);
  await openPage(page, 'repos');
  await expect(repoRow(page, NEXT)).toBeVisible();
  await expect(repoRow(page, FILTERED)).toHaveCount(0);

  // The search still finds it, and its menu hides it as for any repository.
  await page.getByLabel('Repositories durchsuchen').fill('runner-tools');
  await expect(repoRow(page, FILTERED)).toBeVisible();
  await menuButton(page, FILTERED).click();
  await expect(
    page.getByRole('menuitem', { name: 'Ausblenden', exact: true }),
  ).toBeVisible();
  await page.keyboard.press('Escape');
  await page.getByLabel('Repositories durchsuchen').fill('');

  // Watching every owner again sends no filter: the place goes back.
  await openApp(page);
  await expect(owner(page, 'example-org')).toHaveAttribute(
    'aria-pressed',
    'false',
  );
  await owner(page, 'example-org').click();
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(repoRow(page, FILTERED)).toBeVisible();
  await expect(repoRow(page, NEXT)).toHaveCount(0);
});

test('the native shell gets the watched owners once, all owners as none', async ({
  page,
}) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      'repowatch.activeAccount.v1',
      JSON.stringify('demo-work'),
    ),
  );
  await installTauriMock<typeof fixture, object>(
    page,
    (fixture) => {
      const snapshot = {
        ...structuredClone(fixture),
        mode: 'live',
        stale: false,
        connectivity: 'online',
        ownerFilters: [] as { accountId: string; owners: string[] }[],
      };
      return (cmd, args) => {
        if (cmd === 'set_owner_filter') {
          const { accountId, owners } = args as {
            accountId: string;
            owners: string[];
          };
          snapshot.ownerFilters = owners.length ? [{ accountId, owners }] : [];
        }
        return cmd === 'snapshot' ||
          cmd === 'refresh_snapshot' ||
          cmd === 'set_owner_filter'
          ? structuredClone(snapshot)
          : null;
      };
    },
    { arg: fixture, state: {} },
  );
  await page.goto('/');
  await expectPage(page, 'overview');
  await openApp(page);
  await owner(page, 'example-org').click();
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  await openApp(page);
  await owner(page, 'example-org').click();
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await readCalls(page, 'set_owner_filter')).toEqual([
    {
      cmd: 'set_owner_filter',
      args: { accountId: 'demo-work', owners: ['team-demo'] },
    },
    { cmd: 'set_owner_filter', args: { accountId: 'demo-work', owners: [] } },
  ]);
});
