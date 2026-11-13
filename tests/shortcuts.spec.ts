import { expect, test, type Page } from './support/test';
import { connectToken } from './support/connect';
import {
  PAGE_TITLES,
  accountMenuButton,
  maximizeButton,
  openConnection,
} from './support/demo';

/** The title of a maximized page; the overview has none. */
const pageTitle = (page: Page) => page.locator('.page-title');
const heading = (page: Page, name: keyof typeof PAGE_TITLES) =>
  page.getByRole('heading', { level: 2, name: PAGE_TITLES[name], exact: true });
const refreshed = (page: Page) =>
  page.getByRole('status').filter({ hasText: 'Demo-Status aktualisiert.' });

/** Records, after the panel's own handler, whether a key kept its default. */
async function watchDefaults(page: Page) {
  await page.evaluate(() => {
    const seen: string[] = [];
    Object.assign(window, { __defaults: seen });
    window.addEventListener('keydown', (event) => {
      if (event.defaultPrevented) seen.push(event.key);
    });
  });
}
const prevented = (page: Page) =>
  page.evaluate(
    () => (window as unknown as { __defaults: string[] }).__defaults,
  );

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(
    page.getByRole('button', { name: 'Einstellungen', exact: true }),
  ).toBeEnabled();
  await watchDefaults(page);
});

test('F5 and Ctrl+R refresh the panel instead of reloading the webview', async ({
  page,
}) => {
  await page.evaluate(() => Object.assign(window, { __sameDocument: true }));
  await page.keyboard.press('F5');
  await expect(refreshed(page)).toBeVisible();
  await page
    .getByRole('status')
    .getByRole('button', { name: 'Meldung schließen' })
    .click();
  await page.keyboard.press('Control+r');
  await expect(refreshed(page)).toBeVisible();
  expect(await prevented(page)).toEqual(['F5', 'r']);
  expect(
    await page.evaluate(
      () => (window as unknown as { __sameDocument?: true }).__sameDocument,
    ),
  ).toBe(true);
});

test('Ctrl+1/2/3 maximize a section; its own key returns to the overview', async ({
  page,
}) => {
  await page.keyboard.press('Control+2');
  await expect(heading(page, 'runs')).toBeFocused();
  await page.keyboard.press('Control+3');
  await expect(heading(page, 'favorites')).toBeFocused();
  await page.keyboard.press('Control+1');
  await expect(heading(page, 'repos')).toBeFocused();
  await page.keyboard.press('Control+1');
  await expect(pageTitle(page)).toHaveCount(0);
  await expect(maximizeButton(page, 'repos')).toBeFocused();
  for (const [name, key] of [
    ['repos', 'Control+1'],
    ['runs', 'Control+2'],
    ['favorites', 'Control+3'],
  ] as const)
    await expect(maximizeButton(page, name)).toHaveAttribute(
      'aria-keyshortcuts',
      key,
    );
});

test('/ focuses the search; typing keeps plain keys, Escape clears then leaves', async ({
  page,
}) => {
  const search = page.getByLabel('Übersicht durchsuchen');
  await page.keyboard.press('/');
  await expect(search).toBeFocused();
  await expect(search).toHaveValue('');
  await page.keyboard.type('demo/1');
  await expect(search).toHaveValue('demo/1');
  await page.keyboard.press('Escape');
  await expect(search).toHaveValue('');
  await expect(search).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(search).not.toBeFocused();
});

test('Ctrl+F focuses the search and selects its query, also from a field', async ({
  page,
}) => {
  const search = page.getByLabel('Übersicht durchsuchen');
  await expect(search).toHaveAttribute('aria-keyshortcuts', '/ Control+F');
  await page.keyboard.press('Control+f');
  await expect(search).toBeFocused();
  await page.keyboard.type('demo');
  await page.keyboard.press('Control+f');
  // The query is selected: typing replaces it instead of appending.
  await page.keyboard.type('alpha');
  await expect(search).toHaveValue('alpha');
  expect(await prevented(page)).toEqual(['f', 'f']);
});

test('ArrowDown in the search moves focus to the first result', async ({
  page,
}) => {
  const search = page.getByLabel('Übersicht durchsuchen');
  // On the overview the first result is the first run under „Läuft gerade“.
  await search.focus();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('.live-run').first()).toBeFocused();
  // A search narrows it down to the one matching repository.
  await search.fill('sandbox');
  await expect(page.locator('.repository-card')).toHaveCount(1);
  await search.focus();
  await page.keyboard.press('ArrowDown');
  await expect(
    page
      .getByRole('region', { name: 'team-demo/sandbox-demo', exact: true })
      .locator('.repository-toggle'),
  ).toBeFocused();
  // Without a result the caret stays in the field and the key is not taken.
  await search.fill('unmatched-query');
  await page.keyboard.press('ArrowDown');
  await expect(search).toBeFocused();
  // On a maximized page the first Verlauf run takes focus.
  await search.fill('');
  await page.keyboard.press('Control+2');
  await page.getByLabel('Verlauf durchsuchen').focus();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('.run-button').first()).toBeFocused();
  expect(await prevented(page)).toEqual([
    'ArrowDown',
    'ArrowDown',
    '2',
    'ArrowDown',
  ]);
});

test('ArrowDown and ArrowUp move between the results in page order', async ({
  page,
}) => {
  const search = page.getByLabel('Übersicht durchsuchen');
  const results = page.locator(
    '.content :is(.live-run, .pin-open, .repository-toggle, .run-button)',
  );
  await expect(results.nth(1)).toBeVisible();
  await search.focus();
  await page.keyboard.press('ArrowDown');
  await expect(results.nth(0)).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(results.nth(1)).toBeFocused();
  await page.keyboard.press('ArrowUp');
  await expect(results.nth(0)).toBeFocused();
  // ArrowUp on the first result returns to the search field.
  await page.keyboard.press('ArrowUp');
  await expect(search).toBeFocused();
  // The step crosses sections: the last live run leads to the first favorite.
  const lastLive = page.locator('.live-run').last();
  await lastLive.focus();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('.pin-open').first()).toBeFocused();
  // On the last result the key keeps its default and focus stays.
  const last = results.last();
  await last.focus();
  await page.keyboard.press('ArrowDown');
  await expect(last).toBeFocused();
  // With a modifier the key is left alone.
  await results.nth(1).focus();
  await page.keyboard.press('Shift+ArrowUp');
  await expect(results.nth(1)).toBeFocused();
  expect(await prevented(page)).toEqual([
    'ArrowDown',
    'ArrowDown',
    'ArrowUp',
    'ArrowUp',
    'ArrowDown',
  ]);
});

test('F5, Ctrl+R and Ctrl+1/2/3 still work from the search field', async ({
  page,
}) => {
  const search = page.getByLabel('Übersicht durchsuchen');
  await page.keyboard.press('/');
  await page.keyboard.type('demo');
  await page.keyboard.press('F5');
  await expect(refreshed(page)).toBeVisible();
  await expect(search).toHaveValue('demo');
  await expect(search).toBeFocused();
  await page
    .getByRole('status')
    .getByRole('button', { name: 'Meldung schließen' })
    .click();
  await search.focus();
  await page.keyboard.press('Control+r');
  await expect(refreshed(page)).toBeVisible();
  await search.focus();
  await page.keyboard.press('Control+2');
  await expect(heading(page, 'runs')).toBeFocused();
  // The query stays with the panel, not with the page.
  await expect(page.getByLabel('Verlauf durchsuchen')).toHaveValue('demo');
  expect(await prevented(page)).toEqual(['/', 'F5', 'r', '2']);
});

test('an open dialog keeps the keyboard and never reloads', async ({
  page,
}) => {
  await page.keyboard.press('Control+2');
  await expect(heading(page, 'runs')).toBeFocused();
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  const dialog = page.getByRole('dialog', { name: 'Einstellungen' });
  await expect(dialog).toBeVisible();
  await page.keyboard.press('Control+3');
  await page.keyboard.press('/');
  await page.keyboard.press('F5');
  expect(await prevented(page)).toEqual(['2', 'F5']);
  await expect(refreshed(page)).toHaveCount(0);
  // Escape closes only the dialog; the page behind it stays open.
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(heading(page, 'runs')).toBeVisible();
});

test('Ctrl+, opens the settings, also from the search field', async ({
  page,
}) => {
  const settings = page.getByRole('button', {
    name: 'Einstellungen',
    exact: true,
  });
  await expect(settings).toHaveAttribute('aria-keyshortcuts', 'Control+,');
  await page.getByLabel('Übersicht durchsuchen').focus();
  await page.keyboard.press('Control+Comma');
  const dialog = page.getByRole('dialog', { name: 'Einstellungen' });
  await expect(dialog).toBeVisible();
  expect(await prevented(page)).toEqual([',']);
  // The open dialog keeps the keyboard; Escape closes it.
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
});

test('the search hint lists the keyboard shortcuts', async ({ page }) => {
  const hint = page.locator('.search-field kbd');
  await expect(hint).toHaveText('/');
  const title = await hint.getAttribute('title');
  expect(title).toContain('Tastenkürzel');
  for (const entry of [
    'Strg+F',
    'F5 oder Strg+R',
    'Strg+1, 2, 3',
    'Strg+,',
    'Esc',
  ])
    expect(title).toContain(entry);
  await expect(
    page.getByRole('button', { name: 'Aktualisieren', exact: true }),
  ).toHaveAttribute('aria-keyshortcuts', 'F5 Control+R');
});

// Focus regressions: SearchField.focus() and AppBar.focusAccount().
test('pressing / on a run row moves focus into the search field', async ({
  page,
}) => {
  await page.keyboard.press('Control+2');
  await page.locator('.run-button').first().focus();
  await page.keyboard.press('/');
  await expect(page.getByLabel('Verlauf durchsuchen')).toBeFocused();
  await expect(page.getByLabel('Verlauf durchsuchen')).toHaveValue('');
});

test('finishing the demo token connect focuses the account menu button', async ({
  page,
}) => {
  await openConnection(page);
  await connectToken(page, 'demo-token', 'Beispiel-Token');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(accountMenuButton(page)).toBeFocused();
});
