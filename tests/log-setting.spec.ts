import { expect, test, type Page } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import { installTauriMock, readCalls } from './support/tauriMock';

/** „Protokoll“ in Settings → App: the level is staged, the folder opens at once. */
const dialog = (page: Page) =>
  page.getByRole('dialog', { name: 'Einstellungen', exact: true });
const save = (page: Page) =>
  dialog(page).getByRole('button', { name: 'Speichern', exact: true });
const levels = (page: Page) =>
  dialog(page).getByRole('radiogroup', { name: 'Protokoll' });
const level = (page: Page, name: string) =>
  levels(page).getByRole('radio', { name, exact: true });
const folder = (page: Page) =>
  dialog(page).getByRole('button', { name: 'Ordner öffnen', exact: true });
async function openApp(page: Page) {
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await dialog(page).getByRole('tab', { name: 'App' }).click();
}
const stored = (page: Page) =>
  page.evaluate(() => localStorage.getItem('repowatch.demo.log.v1'));

test('the log level stages and applies only with „Speichern“; the demo has no folder', async ({
  page,
}) => {
  await page.goto('/');
  await openApp(page);
  await expect(level(page, 'Info')).toBeChecked();
  await expect(level(page, 'Debug')).not.toBeChecked();
  // The browser demo writes no log file.
  await expect(folder(page)).toHaveCount(0);
  await expect(save(page)).toBeDisabled();
  // A staged level is dropped by „Abbrechen“.
  await level(page, 'Debug').check();
  await expect(save(page)).toBeEnabled();
  await dialog(page)
    .getByRole('button', { name: 'Abbrechen', exact: true })
    .click();
  expect(await stored(page)).toBeNull();
  await openApp(page);
  await expect(level(page, 'Info')).toBeChecked();
  await level(page, 'Debug').check();
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await stored(page)).toBe('"debug"');
  await page.reload();
  await openApp(page);
  await expect(level(page, 'Debug')).toBeChecked();
  await expect(save(page)).toBeDisabled();
});

test('the native shell reads the level, saves it with „Speichern“ and opens the folder at once', async ({
  page,
}) => {
  await installTauriMock<{ fixture: typeof fixture }, { failOpen: boolean }>(
    page,
    ({ fixture }, state) => {
      let level = 'debug';
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
        if (cmd === 'refresh_interval') return 60;
        if (cmd === 'log_level') return level;
        if (cmd === 'set_log_level') {
          level = String(args?.level);
          return level;
        }
        if (cmd === 'open_log_folder' && state.failOpen)
          throw new Error('Log-Ordner konnte nicht geöffnet werden.');
        return null;
      };
    },
    { arg: { fixture }, state: { failOpen: false } },
  );
  await page.goto('/');
  await openApp(page);
  await expect(level(page, 'Debug')).toBeChecked();
  // The folder button acts at once and stages nothing.
  await folder(page).click();
  expect(await readCalls(page, 'open_log_folder')).toHaveLength(1);
  await expect(save(page)).toBeDisabled();
  await expect(dialog(page)).toBeVisible();
  await level(page, 'Info').check();
  expect(await readCalls(page, 'set_log_level')).toEqual([]);
  await save(page).click();
  await expect(dialog(page)).toHaveCount(0);
  expect(await readCalls(page, 'set_log_level')).toEqual([
    { cmd: 'set_log_level', args: { level: 'info' } },
  ]);
  // A folder that cannot be opened is reported in the dialog.
  await page.evaluate(() => {
    window.__repoWatchIpcMock!.state.failOpen = true;
  });
  await openApp(page);
  await folder(page).click();
  await expect(dialog(page).getByRole('alert')).toHaveText(
    'Log-Ordner konnte nicht geöffnet werden.',
  );
});
