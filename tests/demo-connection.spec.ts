import { expect, test } from './support/test';
import { chooseToken } from './support/connect';
import { openDemoActivity, openConnection } from './support/demo';

test.beforeEach(async ({ page }) => {
  await openDemoActivity(page);
});

test('the demo offers the same method step: browser sign-in paused, local sign-in desktop-only, token preselected', async ({
  page,
}) => {
  const dialog = await openConnection(page);
  await expect(
    dialog.getByRole('radio', { name: /^Mit GitHub anmelden/ }),
  ).toBeDisabled();
  await expect(dialog.getByRole('radiogroup')).toContainText(
    'Vorübergehend deaktiviert',
  );
  await expect(
    dialog.getByRole('radio', {
      name: /^Bestehende Anmeldung auf diesem PC verwenden.*Nur in der Desktop-App$/,
    }),
  ).toBeDisabled();
  await expect(
    dialog.getByRole('radio', { name: /^Mit Token verbinden/ }),
  ).toBeChecked();
  await dialog.getByRole('button', { name: 'Weiter', exact: true }).click();
  const connect = dialog.getByRole('button', {
    name: 'Verbinden',
    exact: true,
  });
  await expect(connect).toBeDisabled();
  await expect(connect).toHaveAttribute('title', 'Token eingeben');
  // „Zurück“ returns to the method step, not out of the dialog.
  await dialog.getByRole('button', { name: 'Zurück', exact: true }).click();
  await expect(
    dialog.getByRole('heading', { name: 'Anmeldeart wählen' }),
  ).toBeFocused();
  expect(page.context().pages()).toHaveLength(1);
});

test('Enter on the preselected token method continues to the token form', async ({
  page,
}) => {
  const dialog = await openConnection(page);
  await dialog.getByRole('radio', { name: /^Mit Token verbinden/ }).focus();
  await page.keyboard.press('Enter');
  await expect(
    dialog.getByRole('heading', { name: 'Mit Token verbinden' }),
  ).toBeFocused();
  await expect(dialog.getByLabel('Beispiel-Token')).toBeVisible();
});

test('cancelling a pending token preview invalidates its late confirmation', async ({
  page,
}) => {
  await page.clock.install();
  const dialog = await openConnection(page);
  await chooseToken(page);
  await dialog.getByLabel('Beispiel-Token').fill('demo-token');
  // Time stands still: the preview's confirmation cannot arrive on its own.
  await page.clock.pauseAt(Date.now() + 2000);
  await dialog.getByRole('button', { name: 'Verbinden', exact: true }).click();
  await expect(
    dialog.getByRole('heading', { name: 'Wird geprüft …' }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await page.clock.fastForward(1000);
  await expect(
    dialog.getByRole('heading', { name: 'Anmeldung abgebrochen' }),
  ).toBeVisible();
  await expect(
    dialog.getByRole('button', { name: 'Erneut versuchen', exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/demo-dev simuliert verbunden/)).toHaveCount(0);
});

test('token preview clears its masked input and never persists user input', async ({
  page,
}) => {
  const dialog = await openConnection(page);
  await chooseToken(page);
  const input = dialog.getByLabel('Beispiel-Token');
  const connect = dialog.getByRole('button', {
    name: 'Verbinden',
    exact: true,
  });
  await input.fill('not-a-real-token-example');
  await connect.click();
  await expect(input).toHaveValue('');
  await expect(dialog.getByRole('alert')).toHaveText(
    'Nur „demo-token“ funktioniert hier. Nichts gespeichert.',
  );
  const storage = await page.evaluate(() =>
    JSON.stringify({ ...localStorage }),
  );
  expect(storage).not.toContain('not-a-real-token-example');
  await input.fill('demo-token');
  await connect.click();
  // Success closes the dialog by itself.
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText(/demo-dev simuliert verbunden/)).toBeVisible();
  expect(
    await page.evaluate(() => JSON.stringify({ ...localStorage })),
  ).not.toContain('demo-token');
});

test('confirmation notice centers text, icon and close target at narrow and standard widths', async ({
  page,
}) => {
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Konto hinzufügen', exact: true })
    .click();
  const dialog = page.getByRole('dialog', { name: 'GitHub verbinden' });
  await chooseToken(page);
  await dialog.getByLabel('Beispiel-Token').fill('demo-token');
  await dialog.getByRole('button', { name: 'Verbinden', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText(/demo-dev simuliert verbunden/)).toBeVisible();
  for (const width of [480, 360]) {
    await page.setViewportSize({ width, height: 760 });
    const geometry = await page.locator('.notice-banner').evaluate((banner) => {
      const center = (node: Element) => {
        const r = node.getBoundingClientRect();
        return r.top + r.height / 2;
      };
      return {
        center: center(banner),
        children: ['span', 'svg', 'button'].map((selector) =>
          center(banner.querySelector(selector)!),
        ),
        height: banner.getBoundingClientRect().height,
      };
    });
    for (const center of geometry.children)
      expect(Math.abs(center - geometry.center)).toBeLessThanOrEqual(0.5);
    // At most two lines of 12px text (the metadata minimum) plus padding.
    expect(geometry.height).toBeLessThanOrEqual(55);
  }
});
