import { expect, test, type Locator } from './support/test';
import {
  chooseAccount,
  openDemoActivity,
  openLaunch,
  openPage,
  scenario,
  templateCard,
} from './support/demo';

test.beforeEach(async ({ page }) => {
  await openDemoActivity(page);
});

/** The favorite card's button that opens it in the start dialog. */
const opener = (card: Locator, label: string) =>
  card.getByRole('button', {
    name: `launch-kit-demo · ${label} starten…`,
    exact: true,
  });

test('pins persist and the name opens their stored configuration', async ({
  page,
}) => {
  let dialog = await openLaunch(page);
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog.getByLabel('Name des Favoriten').fill('My saved build');
  await dialog
    .getByRole('button', { name: 'Favorit speichern', exact: true })
    .click();
  await expect(
    dialog.getByText('„My saved build“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await page.reload();
  await openPage(page, 'favorites');
  const card = templateCard(page, 'My saved build');
  await opener(card, 'My saved build').click();
  dialog = page.getByRole('dialog', { name: 'Workflow starten' });
  await expect(dialog.getByLabel('Branch oder Tag')).toHaveValue('main');
  await expect(dialog.getByLabel('Version')).toHaveValue('0.1.0');
  // No opt-in any more: every complete favorite starts at once.
  await expect(dialog.getByText(/Schnellstart/)).toHaveCount(0);
});

test('a saved complete favorite starts at once from its round start', async ({
  page,
}) => {
  const dialog = await openLaunch(page);
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog.getByLabel('Name des Favoriten').fill('Direct build');
  await dialog
    .getByRole('button', { name: 'Favorit speichern', exact: true })
    .click();
  await expect(
    dialog.getByText('„Direct build“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  const card = templateCard(page, 'Direct build');
  await card
    .getByRole('button', {
      name: 'launch-kit-demo · Direct build jetzt starten',
      exact: true,
    })
    .click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByText(/Demo-Run #9001/)).toBeVisible();
});

test('Enter in a template name saves it without starting a run', async ({
  page,
}) => {
  const dialog = await openLaunch(page);
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog.getByLabel('Name des Favoriten').fill('Saved with Enter');
  await dialog.getByLabel('Name des Favoriten').press('Enter');
  await expect(
    dialog.getByText('„Saved with Enter“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    dialog.getByText('Simulierter Start angenommen', { exact: true }),
  ).toHaveCount(0);
  await expect(
    dialog.getByRole('button', { name: 'Start simulieren' }),
  ).toBeEnabled();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await openPage(page, 'runs');
  await expect(page.getByText(/9001/)).toHaveCount(0);
});

test('an off save button names why in its tooltip', async ({ page }) => {
  const dialog = await openLaunch(page);
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  const save = dialog.getByRole('button', {
    name: 'Favorit speichern',
    exact: true,
  });
  // The name starts as the workflow's; an emptied name turns the save off.
  const name = dialog.getByLabel('Name des Favoriten');
  await expect(name).not.toHaveValue('');
  await expect(save).not.toHaveAttribute('aria-disabled');
  await name.fill('');
  await expect(save).toHaveAttribute('aria-disabled', 'true');
  await expect(save).toHaveAttribute('title', 'Name fehlt');
  // Off, a press keeps focus on the button and saves nothing.
  await save.focus();
  await page.keyboard.press('Enter');
  await expect(save).toBeFocused();
  await expect(dialog.getByText(/als Favorit gespeichert\./)).toHaveCount(0);
  await name.fill('Named build');
  await expect(save).not.toHaveAttribute('aria-disabled');
  await expect(save).not.toHaveAttribute('title');
  await save.click();
  await expect(
    dialog.getByText('„Named build“ als Favorit gespeichert.', { exact: true }),
  ).toBeVisible();
  // The save is done in place; focus stays on its button.
  await expect(
    dialog.getByRole('button', { name: 'Favorit aktualisieren', exact: true }),
  ).toBeFocused();
});

test('saving a favorite keeps its section open and focus off <body>', async ({
  page,
}) => {
  const dialog = await openLaunch(page);
  // Opened by the user: closed by default in a plain start dialog.
  const section = dialog.locator('details.save-pin');
  await expect(section).not.toHaveAttribute('open');
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  const name = dialog.getByLabel('Name des Favoriten');
  await name.fill('Focus build');
  await dialog
    .getByRole('button', { name: 'Favorit speichern', exact: true })
    .click();
  await expect(
    dialog.getByText('„Focus build“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await expect(section).toHaveAttribute('open');
  await expect(
    dialog.getByRole('button', { name: 'Favorit aktualisieren', exact: true }),
  ).toBeFocused();
  // Enter in the name saves again; the caret stays in the field.
  await name.fill('Focus build 2');
  await name.press('Enter');
  await expect(
    dialog.getByText('„Focus build 2“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await expect(section).toHaveAttribute('open');
  await expect(name).toBeFocused();
  // „Als neuen Favoriten speichern“ keeps focus on itself.
  const asNew = dialog.getByRole('button', {
    name: 'Als neuen Favoriten speichern',
  });
  await name.fill('Focus build 3');
  await asNew.click();
  await expect(
    dialog.getByText('„Focus build 3“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await expect(section).toHaveAttribute('open');
  await expect(asNew).toBeFocused();
});

test('saving is blocked during dispatch and the result still appears', async ({
  page,
}) => {
  const dialog = await openLaunch(page);
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog
    .getByLabel('Name des Favoriten')
    .fill('Do not save during dispatch');
  const blocked = await dialog.evaluate(async (node) => {
    const buttons = [...node.querySelectorAll('button')];
    const start = buttons.find((button) =>
      button.textContent?.includes('Start simulieren'),
    )!;
    const save = buttons.find((button) =>
      button.textContent?.includes('Favorit speichern'),
    )!;
    start.click();
    await new Promise<void>((resolve) => queueMicrotask(resolve));
    // Off via aria-disabled, so a pressed save keeps focus; the click is ignored.
    const disabled = save.getAttribute('aria-disabled') === 'true';
    save.click();
    return disabled;
  });
  expect(blocked).toBe(true);
  await expect(
    dialog.getByText('Simulierter Start angenommen', { exact: true }),
  ).toBeVisible();
  await expect(dialog.getByText(/als Favorit gespeichert\./)).toHaveCount(0);
});

test('offline still allows opening and editing a saved template', async ({
  page,
}) => {
  await scenario(page, 'offline');
  await openPage(page, 'favorites');
  const card = templateCard(page, 'Windows build');
  await expect(opener(card, 'Windows build')).toBeEnabled();
  await opener(card, 'Windows build').click();
  const dialog = page.getByRole('dialog', { name: 'Workflow starten' });
  await expect(dialog.getByLabel('Version')).toBeEnabled();
  await expect(
    dialog.getByRole('button', { name: 'Start simulieren' }),
  ).toBeDisabled();
});

test('an empty favorites search hides every card and offers a clear action', async ({
  page,
}) => {
  await openPage(page, 'favorites');
  await page.getByLabel('Favoriten durchsuchen').fill('unmatched-query');
  const state = page.locator('.empty-state').filter({
    has: page.getByRole('heading', { name: 'Keine passenden Favoriten' }),
  });
  await expect(state).toBeVisible();
  await expect(page.locator('.pin-row')).toHaveCount(0);
  await state.getByRole('button', { name: 'Suche leeren' }).click();
  await expect(templateCard(page, 'Windows build')).toBeVisible();
});

test('copying an existing template keeps the original and its values', async ({
  page,
}) => {
  await openPage(page, 'favorites');
  const original = templateCard(page, 'Windows build');
  await opener(original, 'Windows build').click();
  const dialog = page.getByRole('dialog', { name: 'Workflow starten' });
  await dialog.getByLabel('Branch oder Tag').selectOption('develop');
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog.getByLabel('Name des Favoriten').fill('Develop copy');
  await expect(
    dialog.getByRole('button', { name: 'Favorit aktualisieren', exact: true }),
  ).toBeVisible();
  await dialog
    .getByRole('button', { name: 'Als neuen Favoriten speichern', exact: true })
    .click();
  await expect(
    dialog.getByText('„Develop copy“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await expect(templateCard(page, 'Develop copy')).toBeVisible();
  await opener(original, 'Windows build').click();
  await expect(
    page.getByRole('dialog').getByLabel('Branch oder Tag'),
  ).toHaveValue('main');
});

test('compact saved templates keep their configuration and removal reachable', async ({
  page,
}) => {
  const dialog = await openLaunch(page);
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog.getByLabel('Name des Favoriten').fill('Compact build');
  await dialog
    .getByRole('button', { name: 'Favorit speichern', exact: true })
    .click();
  await expect(
    dialog.getByText('„Compact build“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  const card = templateCard(page, 'Compact build');
  await expect(
    card.getByRole('button', {
      name: 'launch-kit-demo · Compact build jetzt starten',
      exact: true,
    }),
  ).toBeVisible();
  await opener(card, 'Compact build').click();
  await expect(page.getByRole('dialog').getByLabel('Version')).toHaveValue(
    '0.1.0',
  );
  await expect(page.getByText(/9001/)).toHaveCount(0);
  await page.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await card
    .getByRole('button', {
      name: 'launch-kit-demo · Compact build aus Favoriten entfernen',
      exact: true,
    })
    .click();
  await expect(card).toHaveCount(0);
  await expect(templateCard(page, 'Windows build')).toBeVisible();
});

test('compact rows keep the repository, account context and click targets at 360px', async ({
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 760 });
  await chooseAccount(page, 'team-demo');
  const dialog = await openLaunch(page);
  const label =
    'Nightly runner health checks for the shared maintenance environment';
  await dialog.getByText('Als Favorit speichern', { exact: true }).click();
  await dialog.getByLabel('Name des Favoriten').fill(label);
  await dialog
    .getByRole('button', { name: 'Favorit speichern', exact: true })
    .click();
  await expect(
    dialog.getByText(`„${label}“ als Favorit gespeichert.`, {
      exact: true,
    }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  const card = templateCard(page, label);
  await expect(card.locator('.pin-repo')).toHaveText('runner-tools-demo');
  // The full repository, the label, the account and the workflow's last run
  // (its waiting run is open) live in the tooltip.
  await expect(card.locator('.pin-open')).toHaveAttribute(
    'title',
    new RegExp(
      `^example-org/runner-tools-demo · ${label} · main · team-demo · Läuft gerade · Letzter Run: \\d{2}\\.\\d{2}\\.\\d{4}, \\d{2}:\\d{2} · Demo-Zeit$`,
    ),
  );
  await expect(card.getByText('main', { exact: true })).toBeVisible();
  const sizes = await card.evaluate((element) => {
    const cardBounds = element.getBoundingClientRect();
    const repo = element.querySelector('.pin-repo')!;
    return {
      overflow: element.scrollWidth > element.clientWidth,
      height: cardBounds.height,
      // The favorite's name gives way; the repository is never cut.
      repoClipped: repo.scrollWidth > repo.clientWidth,
      font: parseFloat(
        getComputedStyle(element.querySelector('.pin-label')!).fontSize,
      ),
      targets: [...element.querySelectorAll('button')].map((button) => {
        const r = button.getBoundingClientRect();
        return {
          width: r.width,
          height: r.height,
          inside: r.left >= cardBounds.left && r.right <= cardBounds.right,
        };
      }),
    };
  });
  expect(sizes.overflow).toBe(false);
  expect(sizes.height).toBeLessThanOrEqual(36);
  expect(sizes.repoClipped).toBe(false);
  expect(sizes.font).toBeGreaterThanOrEqual(12);
  for (const target of sizes.targets) {
    expect(target.width).toBeGreaterThanOrEqual(24);
    expect(target.height).toBeGreaterThanOrEqual(24);
    expect(target.inside).toBe(true);
  }
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
  ).toBe(false);
});
