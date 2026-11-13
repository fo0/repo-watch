import { expect, test, type Page } from './support/test';
import { openLaunch, openPage, scenario, templateCard } from './support/demo';

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(
    page.getByRole('button', { name: 'Einstellungen', exact: true }),
  ).toBeEnabled();
});

/** The search field of the current view, whatever its label. */
const search = (page: Page) => page.locator('.search-field input');

test('clearing the search keeps focus in the search field', async ({
  page,
}) => {
  await search(page).fill('kit');
  await page
    .locator('.search-field')
    .getByRole('button', { name: 'Suche leeren' })
    .click();
  await expect(search(page)).toHaveValue('');
  await expect(search(page)).toBeFocused();

  // The empty state's „Suche leeren“ goes back to the field on every page.
  for (const name of ['overview', 'repos', 'runs', 'favorites'] as const) {
    await openPage(page, name);
    await search(page).fill('unmatched-query');
    await page
      .locator('.empty-state')
      .getByRole('button', { name: 'Suche leeren' })
      .click();
    await expect(search(page)).toHaveValue('');
    await expect(search(page)).toBeFocused();
  }
});

test('a direct start keeps focus on its button while it runs and after it settles', async ({
  page,
}) => {
  const dialog = await openLaunch(page);
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog.getByLabel('Name des Favoriten').fill('Quick build');
  await dialog
    .getByRole('button', { name: 'Favorit speichern', exact: true })
    .click();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  // The demo answers after 300 ms; a paused clock holds the start in flight.
  await page.clock.install();
  await page.clock.pauseAt(Date.now() + 2000);
  const start = templateCard(page, 'Quick build').getByRole('button', {
    name: 'launch-kit-demo · Quick build jetzt starten',
    exact: true,
  });
  await start.focus();
  await page.keyboard.press('Enter');
  await expect(start).toHaveAttribute('aria-disabled', 'true');
  await expect(start).toBeFocused();
  // Another activation while it runs sends nothing.
  await page.keyboard.press('Enter');
  await page.clock.runFor(400);
  await expect(
    page.getByRole('status').filter({ hasText: /Quick build: .*#9001/ }),
  ).toBeVisible();
  await expect(page.getByText(/#9002/)).toHaveCount(0);
  await expect(start).toBeEnabled();
  await expect(start).toBeFocused();
});

test('the dialog start keeps focus while it runs, then hands it to the result', async ({
  page,
}) => {
  const dialog = await openLaunch(page);
  await page.clock.install();
  await page.clock.pauseAt(Date.now() + 2000);
  const start = dialog.getByRole('button', { name: 'Start simulieren' });
  await start.focus();
  await page.keyboard.press('Enter');
  const running = dialog.getByRole('button', { name: /Simuliert/ });
  await expect(running).toHaveAttribute('aria-disabled', 'true');
  await expect(running).toBeFocused();
  await page.clock.runFor(400);
  await expect(
    dialog
      .getByRole('status')
      .filter({ hasText: 'Simulierter Start angenommen' }),
  ).toBeFocused();
});

for (const view of ['favorites', 'overview'] as const) {
  test(`closing the start dialog returns focus to a locked favorite, which stays focusable (${view})`, async ({
    page,
  }) => {
    await openPage(page, view);
    // The demo answers after 300 ms; a paused clock holds the start in flight.
    await page.clock.install();
    await page.clock.pauseAt(Date.now() + 2000);
    const chip = templateCard(page, 'Windows build').getByRole('button', {
      name: 'launch-kit-demo · Windows build starten…',
      exact: true,
    });
    await chip.click();
    const dialog = page.getByRole('dialog', { name: 'Workflow starten' });
    await dialog.getByRole('button', { name: 'Start simulieren' }).click();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(chip).toBeFocused();
    await expect(chip).toHaveAttribute('aria-disabled', 'true');
    await expect(chip).toHaveAccessibleDescription('Start läuft …');
    // A locked name opens nothing.
    await page.keyboard.press('Enter');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await page.clock.runFor(400);
    await expect(chip).toBeEnabled();
  });
}

test('„Status prüfen“ keeps focus while the check runs', async ({ page }) => {
  await scenario(page, 'uncertain');
  let dialog = await openLaunch(page);
  await dialog.getByRole('button', { name: 'Start simulieren' }).click();
  await dialog.getByRole('button', { name: 'Schließen', exact: true }).click();
  await page
    .getByRole('button', { name: 'Workflow starten …', exact: true })
    .click();
  dialog = page.getByRole('dialog', { name: 'Workflow starten' });
  const check = dialog.getByRole('button', {
    name: 'Status prüfen',
    exact: true,
  });
  // The demo answers after 300 ms; a paused clock holds the check in flight.
  await page.clock.install();
  await page.clock.pauseAt(Date.now() + 2000);
  await check.focus();
  await page.keyboard.press('Enter');
  await expect(check).toHaveAttribute('aria-disabled', 'true');
  await expect(check).toBeFocused();
  await page.clock.runFor(400);
  await expect(
    dialog.getByText(
      /Status geprüft\. GitHub hat den Start weiterhin nicht bestätigt/,
    ),
  ).toBeVisible();
  await expect(check).not.toHaveAttribute('aria-disabled');
  await expect(check).toBeFocused();
});
