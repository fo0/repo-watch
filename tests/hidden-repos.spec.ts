import { expect, test, type Page } from './support/test';
import { openPage } from './support/demo';
import { menuButton, notice, repoRow } from './support/nativeRepos';

/** Hiding repositories (ADR-0008) in the browser demo, which stores the list. */
const HIDDEN = 'example-org/runner-tools-demo';
/** The 11th newest repository of its account: it moves up when one is hidden. */
const NEXT = 'example-org/legacy-portal-demo';

const dialog = (page: Page) =>
  page.getByRole('dialog', { name: 'Einstellungen', exact: true });
const hiddenSection = (page: Page) =>
  dialog(page).getByRole('region', { name: 'Ausgeblendete Repositories' });
async function openApp(page: Page) {
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await dialog(page).getByRole('tab', { name: 'App' }).click();
}
async function choose(page: Page, repo: string, item: string) {
  await menuButton(page, repo).click();
  await page
    .getByRole('menu', { name: `Aktionen für ${repo}` })
    .getByRole('menuitem', { name: item, exact: true })
    .click();
}
/** Its open run in „Läuft gerade“. */
const liveRun = (page: Page) =>
  page.getByRole('button', {
    name: /^example-org\/runner-tools-demo · Runner health #8403 · /,
  });
/** Rows of the Verlauf for one repository. */
const runsOf = (page: Page, repoId: string) =>
  page.locator(`[data-run^="${repoId}:"]`);

test('„Ausblenden“ removes a repository and its runs at once and offers „Rückgängig“', async ({
  page,
}) => {
  await page.goto('/');
  await expect(liveRun(page)).toBeVisible();
  await openPage(page, 'repos');
  await expect(repoRow(page, HIDDEN)).toBeVisible();
  await expect(repoRow(page, NEXT)).toHaveCount(0);

  await choose(page, HIDDEN, 'Ausblenden');
  await expect(repoRow(page, HIDDEN)).toHaveCount(0);
  await expect(repoRow(page, NEXT)).toBeVisible();
  const hidden = notice(page, 'runner-tools-demo ausgeblendet');
  await hidden.getByRole('button', { name: 'Rückgängig', exact: true }).click();
  await expect(repoRow(page, HIDDEN)).toBeVisible();
  await expect(repoRow(page, NEXT)).toHaveCount(0);
  await expect(notice(page, 'runner-tools-demo eingeblendet')).toBeVisible();

  // Hidden again, it stays hidden after a reload and its runs leave the lists.
  await choose(page, HIDDEN, 'Ausblenden');
  await expect(repoRow(page, HIDDEN)).toHaveCount(0);
  await page.reload();
  await openPage(page, 'overview');
  await expect(liveRun(page)).toHaveCount(0);
  await openPage(page, 'runs');
  await expect(runsOf(page, 'demo-personal-kit')).not.toHaveCount(0);
  await expect(runsOf(page, 'demo-team-tools')).toHaveCount(0);
  await openPage(page, 'repos');
  await expect(repoRow(page, NEXT)).toBeVisible();
  await expect(repoRow(page, HIDDEN)).toHaveCount(0);

  // The search still finds it; its menu offers „Einblenden“.
  await page.getByLabel('Repositories durchsuchen').fill('runner-tools');
  await expect(repoRow(page, HIDDEN)).toBeVisible();
  await menuButton(page, HIDDEN).click();
  await expect(
    page.getByRole('menuitem', { name: 'Einblenden', exact: true }),
  ).toBeVisible();
  await page.keyboard.press('Escape');
});

test('Settings → App shows a hidden repository again only with „Speichern“', async ({
  page,
}) => {
  await page.goto('/');
  await openPage(page, 'repos');
  await choose(page, HIDDEN, 'Ausblenden');
  await expect(repoRow(page, HIDDEN)).toHaveCount(0);

  await openApp(page);
  const show = hiddenSection(page).getByRole('button', {
    name: 'example-org/runner-tools-demo einblenden',
    exact: true,
  });
  await expect(show).toHaveAttribute('aria-pressed', 'false');
  // The owner is shown too, as in the repository rows.
  await expect(hiddenSection(page).locator('.hidden-name')).toHaveText(
    'example-org/runner-tools-demo',
  );
  const save = dialog(page).getByRole('button', {
    name: 'Speichern',
    exact: true,
  });
  await expect(save).toHaveAttribute('aria-disabled', 'true');
  // Marking only stages; „Abbrechen“ drops it.
  await show.click();
  await expect(show).toHaveAttribute('aria-pressed', 'true');
  await expect(save).not.toHaveAttribute('aria-disabled');
  await dialog(page)
    .getByRole('button', { name: 'Abbrechen', exact: true })
    .click();
  await expect(repoRow(page, HIDDEN)).toHaveCount(0);

  await openApp(page);
  await show.click();
  await save.click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(repoRow(page, HIDDEN)).toBeVisible();
  await expect(repoRow(page, NEXT)).toHaveCount(0);
  await openApp(page);
  await expect(hiddenSection(page)).toContainText('Keine');
});
