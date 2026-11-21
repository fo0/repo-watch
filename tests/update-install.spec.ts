import { expect, test } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import {
  type TauriMock,
  installTauriMock,
  readCalls,
  readCommands,
} from './support/tauriMock';

/** Labelled IPC mock of the in-app update: check, then one install call with progress. */
type UpdateMock = {
  version: string;
  current: boolean;
  checkFail: boolean;
  installFail: boolean;
  total: number | null;
  stageDelay: number;
  /** Holds `check_update` in flight, like a slow network. */
  checkDelay: number;
};
let mock: TauriMock<UpdateMock>;

const VERSION = '2026.1101.905';
// Synthetic notes in the shape of GitHub's generated changelog.
const NOTES = [
  'Release 2026.1101.905 · nur IPC-Mock',
  '',
  "## What's Changed",
  '* feat: synthetische Änderung eins by @synthetic-user in https://github.com/fo0/repo-watch/pull/1',
  '* fix: synthetische Änderung zwei by @synthetic-user in https://github.com/fo0/repo-watch/pull/2',
  '',
  '**Full Changelog**: https://github.com/fo0/repo-watch/compare/v1...v2',
].join('\n');

test.beforeEach(async ({ page }) => {
  mock = await installTauriMock<
    { fixture: typeof fixture; notes: string },
    UpdateMock
  >(
    page,
    ({ fixture, notes }, state) => {
      const snapshot = {
        ...structuredClone(fixture),
        accounts: [],
        repos: [],
        workflows: [],
        runs: [],
        pins: [],
        mode: 'live',
        stale: false,
        connectivity: 'online',
      };
      const wait = () =>
        new Promise((resolve) => setTimeout(resolve, state.stageDelay));
      return async (cmd, args) => {
        if (cmd === 'snapshot' || cmd === 'refresh_snapshot')
          return structuredClone(snapshot);
        if (cmd === 'check_update') {
          if (state.checkDelay)
            await new Promise((resolve) =>
              setTimeout(resolve, state.checkDelay),
            );
          if (state.checkFail)
            throw 'Update-Prüfung fehlgeschlagen – Verbindung prüfen.';
          return state.current
            ? null
            : { version: state.version, notes, date: '2026-11-01T09:05:00Z' };
        }
        if (cmd === 'install_update') {
          const progress = args?.onProgress as {
            onmessage: (value: unknown) => void;
          };
          progress.onmessage({
            downloaded: state.total ? state.total * 0.42 : 1048576,
            total: state.total,
            stage: 'downloading',
          });
          await wait();
          progress.onmessage({
            downloaded: 0,
            total: null,
            stage: 'verifying',
          });
          await wait();
          if (state.installFail)
            throw 'Datei passt nicht zum Release – nichts installiert.';
          progress.onmessage({
            downloaded: 0,
            total: null,
            stage: 'installing',
          });
          // The real shell starts the installer here and exits the app.
          return null;
        }
        return null;
      };
    },
    {
      arg: { fixture, notes: NOTES },
      state: {
        version: VERSION,
        current: false,
        checkFail: false,
        installFail: false,
        total: 10_000_000,
        stageDelay: 600,
        checkDelay: 0,
      },
    },
  );
  await page.goto('/');
});

test('an available update installs in the app: changelog, progress, check, restart', async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 360, height: 760 });
  await expect(page.getByText(`Update ${VERSION} verfügbar`)).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath('update-banner-360.png') });
  await page.getByRole('button', { name: 'Ansehen', exact: true }).click();
  // The update dot is announced as part of the tab name, not as bare decoration.
  await expect(
    page.getByRole('tab', { name: /^Updates\s*Update verfügbar$/ }),
  ).toHaveAttribute('aria-selected', 'true');
  const panel = page.getByRole('region', { name: 'App-Update' });
  await expect(
    panel.getByText(`Version ${VERSION} verfügbar`, { exact: true }),
  ).toBeVisible();
  await expect(panel.getByText('01.11.2026')).toBeVisible();
  await expect(
    panel.getByRole('list', { name: 'Änderungen' }).getByRole('listitem'),
  ).toHaveText([
    'feat: synthetische Änderung eins',
    'fix: synthetische Änderung zwei',
  ]);
  await expect(panel).not.toContainText('Full Changelog');
  await expect(panel).not.toContainText('Release-Seite');
  await expect(page.getByRole('alert')).toHaveCount(0);
  expect(
    await panel.evaluate((node) => node.scrollWidth <= node.clientWidth),
  ).toBe(true);
  await page.screenshot({
    path: testInfo.outputPath('update-available-360.png'),
  });

  await panel
    .getByRole('button', { name: 'Installieren', exact: true })
    .click();
  const bar = panel.getByRole('progressbar', { name: 'Lädt … 42 %' });
  await expect(bar).toHaveAttribute('value', '42');
  await page.screenshot({
    path: testInfo.outputPath('update-downloading-360.png'),
  });
  await expect(
    panel.getByRole('button', { name: 'Nach Updates suchen' }),
  ).toBeDisabled();
  await expect(panel.getByText('Wird geprüft …')).toBeVisible();
  await expect(
    panel.getByText('Wird installiert – RepoWatch startet neu …'),
  ).toBeVisible();
  await expect(panel.getByRole('button', { name: 'Installieren' })).toHaveCount(
    0,
  );
  expect(
    await panel.evaluate((node) => node.scrollWidth <= node.clientWidth),
  ).toBe(true);
  const installs = await readCalls(page, 'install_update');
  expect(installs).toHaveLength(1);
  const commands = await readCommands(page);
  expect(commands).not.toContain('download_update');
  expect(commands).not.toContain('open_release_page');
});

test('a failed install keeps the update and offers „Erneut versuchen“', async ({
  page,
}) => {
  await mock.configure({ installFail: true, total: null, stageDelay: 300 });
  await page.getByRole('button', { name: 'Ansehen', exact: true }).click();
  const panel = page.getByRole('region', { name: 'App-Update' });
  await panel
    .getByRole('button', { name: 'Installieren', exact: true })
    .click();
  // Without a known size the bar is indeterminate.
  const bar = panel.getByRole('progressbar', { name: 'Lädt …', exact: true });
  await expect(bar).toBeVisible();
  await expect(bar).not.toHaveAttribute('value');
  await expect(panel.getByRole('alert')).toHaveText(
    'Datei passt nicht zum Release – nichts installiert.',
  );
  await expect(
    panel.getByText(`Version ${VERSION} verfügbar`, { exact: true }),
  ).toBeVisible();
  await mock.configure({ installFail: false });
  await panel.getByRole('button', { name: 'Erneut versuchen' }).click();
  await expect(panel.getByRole('alert')).toHaveCount(0);
  await expect(
    panel.getByText('Wird installiert – RepoWatch startet neu …'),
  ).toBeVisible();
  expect(await readCalls(page, 'install_update')).toHaveLength(2);
});

test('a current version reads „Aktuell.“ and offers no installation', async ({
  page,
}) => {
  await mock.configure({ current: true });
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await page.getByRole('tab', { name: 'Updates' }).click();
  const panel = page.getByRole('region', { name: 'App-Update' });
  await panel.getByRole('button', { name: 'Nach Updates suchen' }).click();
  await expect(panel.getByRole('status')).toHaveText('Aktuell.');
  await expect(panel.getByText(/^Zuletzt geprüft \d\d:\d\d$/)).toBeVisible();
  await expect(panel.getByRole('button', { name: 'Installieren' })).toHaveCount(
    0,
  );
  await expect(page.getByRole('alert')).toHaveCount(0);
  await page.getByRole('button', { name: 'Dialog schließen' }).click();
  await expect(page.getByText(/^Update .* verfügbar$/)).toHaveCount(0);
});

test('a failed check shows one alert line and installs nothing', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Ansehen', exact: true }).click();
  const panel = page.getByRole('region', { name: 'App-Update' });
  await mock.configure({ checkFail: true });
  await panel.getByRole('button', { name: 'Nach Updates suchen' }).click();
  await expect(panel.getByRole('alert')).toHaveText(
    'Update-Prüfung fehlgeschlagen – Verbindung prüfen.',
  );
  await expect(panel.getByRole('button', { name: 'Installieren' })).toHaveCount(
    0,
  );
  expect(await readCommands(page)).not.toContain('install_update');
});

test('the banner closes for this version and returns for a newer one', async ({
  page,
}) => {
  const banner = page.getByText(`Update ${VERSION} verfügbar`);
  await expect(banner).toBeVisible();
  const close = page.getByRole('button', { name: 'Hinweis schließen' });
  await close.focus();
  await close.click();
  await expect(banner).toHaveCount(0);
  // Focus is handed on, never dropped to <body>.
  expect(
    await page.evaluate(() => document.activeElement !== document.body),
  ).toBe(true);
  // A repeated check of the same version keeps it closed.
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await page.getByRole('tab', { name: 'Updates' }).click();
  await page.getByRole('button', { name: 'Nach Updates suchen' }).click();
  await expect(
    page.getByText(`Version ${VERSION} verfügbar`, { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Dialog schließen' }).click();
  await expect(banner).toHaveCount(0);
  await mock.configure({ version: '2026.1101.1000' });
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await page.getByRole('tab', { name: 'Updates' }).click();
  await page.getByRole('button', { name: 'Nach Updates suchen' }).click();
  await expect(
    page.getByText('Version 2026.1101.1000 verfügbar', { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Dialog schließen' }).click();
  await expect(page.getByText('Update 2026.1101.1000 verfügbar')).toBeVisible();
});

test('„Nach Updates suchen“ keeps focus while the check runs', async ({
  page,
}) => {
  await mock.configure({ current: true, checkDelay: 400 });
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await page.getByRole('tab', { name: /^Updates/ }).click();
  const panel = page.getByRole('region', { name: 'App-Update' });
  const check = panel.getByRole('button', { name: 'Nach Updates suchen' });
  // The startup check has settled, so the press below starts the only check.
  await expect(check).not.toHaveAttribute('aria-disabled');
  const before = (await readCalls(page, 'check_update')).length;
  await check.focus();
  await page.keyboard.press('Enter');
  await expect(check).toHaveAttribute('aria-disabled', 'true');
  await expect(check).toBeFocused();
  // A press while the check runs sends no second check.
  await page.keyboard.press('Enter');
  await expect(panel.getByText('Aktuell.', { exact: true })).toBeVisible();
  await expect(check).not.toHaveAttribute('aria-disabled');
  await expect(check).toBeFocused();
  expect(await readCalls(page, 'check_update')).toHaveLength(before + 1);
});

test('„Installieren“ hands focus to the progress, a failure back to „Erneut versuchen“', async ({
  page,
}) => {
  await mock.configure({ installFail: true, stageDelay: 1500 });
  await page.getByRole('button', { name: 'Ansehen', exact: true }).click();
  const panel = page.getByRole('region', { name: 'App-Update' });
  await panel
    .getByRole('button', { name: 'Installieren', exact: true })
    .focus();
  await page.keyboard.press('Enter');
  // The pressed button is gone; focus is on the named progress area, not
  // on <body>, and stays there from download to check.
  const progress = panel.getByRole('group', {
    name: 'Installationsfortschritt',
  });
  await expect(progress.getByRole('progressbar')).toBeVisible();
  await expect(progress).toBeFocused();
  await expect(progress.getByText('Wird geprüft …')).toBeVisible();
  await expect(progress).toBeFocused();
  // The failed install brings the button back as the retry, with focus.
  await expect(panel.getByRole('alert')).toBeVisible();
  await expect(
    panel.getByRole('button', { name: 'Erneut versuchen' }),
  ).toBeFocused();
});
