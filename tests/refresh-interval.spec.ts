import { expect, test, type Page } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import { installTauriMock, readCalls } from './support/tauriMock';

/** „Aktualisierung“ in Settings → App: staged like every simple setting. */
const dialog = (page: Page) =>
  page.getByRole('dialog', { name: 'Einstellungen', exact: true });
const save = (page: Page) =>
  dialog(page).getByRole('button', { name: 'Speichern', exact: true });
const slider = (page: Page) =>
  dialog(page).getByRole('slider', { name: 'Aktualisierung' });
const custom = (page: Page) =>
  dialog(page).getByRole('spinbutton', { name: 'Eigener Wert' });
async function openApp(page: Page) {
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await dialog(page).getByRole('tab', { name: 'App' }).click();
}
const stored = (page: Page) =>
  page.evaluate(() => localStorage.getItem('repowatch.demo.refresh.v1'));

test('the refresh interval stages on the slider or as an own value and applies only with „Speichern“', async ({
  page,
}) => {
  await page.goto('/');
  await openApp(page);
  await expect(slider(page)).toHaveAttribute('aria-valuetext', '1 min');
  await expect(custom(page)).toHaveValue('60');
  await expect(save(page)).toBeDisabled();
  // One stop up is staged, then dropped by „Abbrechen“.
  await slider(page).focus();
  await page.keyboard.press('ArrowRight');
  await expect(slider(page)).toHaveAttribute('aria-valuetext', '2 min');
  await expect(custom(page)).toHaveValue('120');
  await expect(save(page)).toBeEnabled();
  await dialog(page)
    .getByRole('button', { name: 'Abbrechen', exact: true })
    .click();
  expect(await stored(page)).toBeNull();
  await openApp(page);
  await expect(slider(page)).toHaveAttribute('aria-valuetext', '1 min');
  // An own value outside 15–3600 s blocks saving and names the range.
  await custom(page).fill('5');
  await expect(custom(page)).toHaveAttribute('aria-invalid', 'true');
  // The error line is announced while focus stays in the field.
  await expect(dialog(page).getByRole('alert')).toHaveText(
    'Aktualisierung muss zwischen 15 s und 60 min liegen.',
  );
  await expect(custom(page)).toBeFocused();
  await expect(save(page)).toBeDisabled();
  // From another tab, the App tab's marker and the tooltip name the cause.
  const app = dialog(page).getByRole('tab', { name: 'App' });
  await dialog(page).getByRole('tab', { name: 'Konten' }).click();
  await expect(app).toHaveAccessibleName('App Aktualisierung ungültig');
  await expect(save(page)).toHaveAttribute('title', 'Aktualisierung ungültig');
  await app.click();
  // A valid own value overrides the slider and is saved.
  await custom(page).fill('90');
  await expect(slider(page)).toHaveAttribute('aria-valuetext', '1 min 30 s');
  await expect(app).toHaveAccessibleName('App');
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await stored(page)).toBe('90');
  await page.reload();
  await openApp(page);
  await expect(custom(page)).toHaveValue('90');
  await expect(save(page)).toBeDisabled();
});

test('the native shell reads the interval and receives the saved value', async ({
  page,
}) => {
  await installTauriMock<{ fixture: typeof fixture }, object>(
    page,
    ({ fixture }) => {
      let seconds = 300;
      const snapshot = {
        ...structuredClone(fixture),
        mode: 'live',
        stale: false,
        connectivity: 'online',
      };
      return (cmd, args) => {
        if (cmd === 'snapshot' || cmd === 'refresh_snapshot')
          return structuredClone(snapshot);
        if (cmd === 'autostart_status')
          return { status: 'disabled', registered: false };
        if (cmd === 'refresh_interval') return seconds;
        if (cmd === 'set_refresh_interval') {
          seconds = Number(args?.seconds);
          return seconds;
        }
        return null;
      };
    },
    { arg: { fixture }, state: {} },
  );
  await page.goto('/');
  await openApp(page);
  await expect(slider(page)).toHaveAttribute('aria-valuetext', '5 min');
  await slider(page).focus();
  await page.keyboard.press('ArrowLeft');
  await expect(slider(page)).toHaveAttribute('aria-valuetext', '2 min');
  expect(await readCalls(page, 'set_refresh_interval')).toEqual([]);
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await readCalls(page, 'set_refresh_interval')).toEqual([
    { cmd: 'set_refresh_interval', args: { seconds: 120 } },
  ]);
});
