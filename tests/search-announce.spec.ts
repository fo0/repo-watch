import { expect, test, type Page } from './support/test';
import { openPage } from './support/demo';

const search = (page: Page) => page.locator('.search-field input');
/** The visually hidden live region beside the lists. */
const region = (page: Page) => page.locator('.search-results');

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(
    page.getByRole('button', { name: 'Einstellungen', exact: true }),
  ).toBeEnabled();
});

test('a settled search announces its result count politely, once', async ({
  page,
}) => {
  await openPage(page, 'repos');
  await expect(region(page)).toHaveRole('status');
  await expect(region(page)).toHaveClass(/\bsr-only\b/);
  await expect(region(page)).toHaveText('');
  // A paused clock holds the debounce; the demo's polling waits too.
  await page.clock.install();
  await page.clock.pauseAt(Date.now() + 2000);
  await search(page).pressSequentially('dem');
  await page.clock.runFor(300);
  await search(page).pressSequentially('o');
  await page.clock.runFor(500);
  // Typing announces nothing: the count waits for a pause.
  await expect(region(page)).toHaveText('');
  await page.clock.runFor(300);
  const shown = await page.locator('.repository-card').count();
  expect(shown).toBeGreaterThan(1);
  await expect(region(page)).toHaveText(`${shown} Treffer`);

  await search(page).fill('unmatched-query');
  await expect(region(page)).toHaveText('');
  await page.clock.runFor(800);
  await expect(region(page)).toHaveText('Keine Treffer');

  // No search, nothing to announce.
  await search(page).fill('');
  await page.clock.runFor(800);
  await expect(region(page)).toHaveText('');
});

test('the overview announces the matches of all three sections', async ({
  page,
}) => {
  await page.clock.install();
  await page.clock.pauseAt(Date.now() + 2000);
  await search(page).fill('launch-kit');
  await page.clock.runFor(800);
  // Live runs, favorites and repositories: a search shows every match.
  const rows = await page
    .locator('.live-row, .pin-row, .repository-card')
    .count();
  expect(rows).toBeGreaterThan(1);
  await expect(region(page)).toHaveText(`${rows} Treffer`);
});
