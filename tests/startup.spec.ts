import { expect, test, type Page } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import { installTauriMock, readCalls, readCommands } from './support/tauriMock';

type MockState = {
  status: string;
  registered: boolean;
  readError: boolean;
  writeError: boolean;
  delay: number;
  readDelay: number;
};
async function setup(page: Page, status = 'disabled') {
  const mock = await installTauriMock<{ fixture: typeof fixture }, MockState>(
    page,
    ({ fixture }, state) => {
      const snapshot = {
        ...fixture,
        mode: 'live',
        stale: false,
        connectivity: 'online',
        accounts: [],
        repos: [],
        workflows: [],
        runs: [],
        pins: [],
      };
      return async (cmd, args) => {
        if (cmd === 'snapshot' || cmd === 'refresh_snapshot')
          return structuredClone(snapshot);
        if (cmd === 'autostart_status') {
          await new Promise((resolve) => setTimeout(resolve, state.readDelay));
          if (state.readError)
            throw 'Windows-Autostart konnte nicht gelesen werden.';
          return { status: state.status, registered: state.registered };
        }
        if (cmd === 'set_autostart') {
          await new Promise((resolve) => setTimeout(resolve, state.delay));
          if (state.writeError)
            throw 'Windows-Autostart konnte nicht geändert werden.';
          state.registered = Boolean(args?.enabled);
          state.status = args?.enabled ? 'enabled' : 'disabled';
          return { status: state.status, registered: state.registered };
        }
        return null;
      };
    },
    {
      arg: { fixture },
      state: {
        status,
        registered: [
          'enabled',
          'disabledByWindows',
          'unknownApproval',
        ].includes(status),
        readError: false,
        writeError: false,
        delay: 0,
        readDelay: 0,
      },
    },
  );
  await page.goto('/');
  await openAppSettings(page);
  return mock;
}
async function openAppSettings(page: Page) {
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await page.getByRole('tab', { name: 'App', exact: true }).click();
}
async function mutations(page: Page) {
  return (await readCalls(page, 'set_autostart')).length;
}
async function capture(page: Page, filename: string) {
  const output = process.env.STARTUP_REVIEW_DIR;
  if (!output) return;
  await mkdir(output, { recursive: true });
  await page.getByRole('dialog').screenshot({
    path: path.join(output, filename),
    animations: 'disabled',
  });
}

const save = (page: Page) =>
  page.getByRole('button', { name: 'Speichern', exact: true });

test('the switch is staged: „Speichern“ writes it once, reads it back and closes; „Abbrechen“ writes nothing', async ({
  page,
}) => {
  await page.setViewportSize({ width: 480, height: 760 });
  const mock = await setup(page);
  const checkbox = page.getByRole('checkbox', { name: 'Mit Windows starten' });
  await expect(checkbox).not.toBeChecked();
  expect(await mutations(page)).toBe(0);
  await capture(page, '01-windows-autostart-aus.png');
  // Staged and dropped: Windows is never touched.
  await checkbox.click();
  await expect(checkbox).toBeChecked();
  await page.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  expect(await mutations(page)).toBe(0);
  await openAppSettings(page);
  await expect(checkbox).not.toBeChecked();
  await mock.configure({ delay: 350 });
  await checkbox.click();
  await expect(checkbox).toBeChecked();
  expect(await mutations(page)).toBe(0);
  await capture(page, '02-windows-autostart-vorgemerkt.png');
  await save(page).click();
  // A second click while the write runs never writes twice.
  await expect(
    page.getByRole('button', { name: 'Wird gespeichert …', exact: true }),
  ).toBeDisabled();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(await mutations(page)).toBe(1);
  await openAppSettings(page);
  await expect(checkbox).toBeChecked();
  await expect(save(page)).toBeDisabled();
  await checkbox.click();
  await expect(checkbox).not.toBeChecked();
  await save(page).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(await mutations(page)).toBe(2);
  expect((await readCalls(page, 'set_autostart')).map((c) => c.args)).toEqual([
    { enabled: true },
    { enabled: false },
  ]);
  const calls = await readCommands(page);
  expect(calls.some((cmd) => /auth|connect|dispatch/.test(cmd))).toBe(false);
});

test('Windows veto is shown at 360px, can open Windows settings and remove only own registration', async ({
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 640 });
  await setup(page, 'disabledByWindows');
  const checkbox = page.getByRole('checkbox', { name: 'Mit Windows starten' });
  await expect(checkbox).toBeDisabled();
  await expect(checkbox).not.toBeChecked();
  await expect(page.getByText(/In Windows deaktiviert/)).toBeVisible();
  expect(await mutations(page)).toBe(0);
  expect(
    await page
      .getByRole('dialog')
      .evaluate((node) => node.scrollWidth <= node.clientWidth),
  ).toBe(true);
  await capture(page, '03-windows-autostart-windows-sperre.png');
  await page.getByRole('button', { name: 'Windows-Autostart öffnen' }).click();
  await expect
    .poll(async () => (await readCalls(page, 'open_startup_settings')).length)
    .toBe(1);
  await page
    .getByRole('button', { name: 'Autostarteintrag entfernen' })
    .click();
  await expect(checkbox).toBeEnabled();
  await expect(checkbox).not.toBeChecked();
  expect(await mutations(page)).toBe(1);
});

test('external removal and Task Manager changes refresh on focus and explicit read', async ({
  page,
}) => {
  const mock = await setup(page, 'enabled');
  const checkbox = page.getByRole('checkbox', { name: 'Mit Windows starten' });
  await expect(checkbox).toBeChecked();
  await mock.configure({ status: 'disabledByWindows' });
  await page.evaluate(() => window.dispatchEvent(new Event('focus')));
  await expect(checkbox).not.toBeChecked();
  await expect(page.getByText(/In Windows deaktiviert/)).toBeVisible();
  await mock.configure({ status: 'disabled', registered: false });
  await page.getByRole('button', { name: 'Status prüfen' }).click();
  await expect(checkbox).toBeEnabled();
  await expect(
    page.getByRole('button', { name: 'Autostarteintrag entfernen' }),
  ).toHaveCount(0);
  expect(await mutations(page)).toBe(0);
});

test('read/write failures stay uncertain until a fresh OS read succeeds', async ({
  page,
}) => {
  const mock = await setup(page);
  const checkbox = page.getByRole('checkbox', { name: 'Mit Windows starten' });
  const panel = page.locator('.startup-panel');
  await mock.configure({ writeError: true });
  await checkbox.click();
  await save(page).click();
  // The dialog stays open with one error line; nothing optimistic is kept.
  await expect(page.locator('.dialog-alert')).toHaveText(
    'Windows-Autostart konnte nicht geändert werden.',
  );
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(checkbox).not.toBeChecked();
  await expect(checkbox).toBeDisabled();
  await expect(panel).toContainText('Status unbekannt.');
  await mock.configure({ readError: true });
  await page.getByRole('button', { name: 'Status prüfen' }).click();
  await expect(panel.getByRole('alert')).toContainText('konnte nicht gelesen');
  await mock.configure({
    readError: false,
    writeError: false,
    status: 'enabled',
    registered: true,
  });
  await page.getByRole('button', { name: 'Status prüfen' }).click();
  await expect(checkbox).toBeChecked();
  await expect(panel.getByRole('alert')).toHaveCount(0);
});

test('„Status prüfen“ keeps focus while Windows is read', async ({ page }) => {
  const mock = await setup(page, 'disabledByWindows');
  await mock.configure({ readDelay: 350 });
  const check = page.getByRole('button', { name: 'Status prüfen' });
  await check.focus();
  await page.keyboard.press('Enter');
  await expect(check).toHaveAttribute('aria-disabled', 'true');
  await expect(check).toBeFocused();
  await expect(check).not.toHaveAttribute('aria-disabled');
  await expect(check).toBeFocused();
  expect((await readCalls(page, 'autostart_status')).length).toBe(2);
});

test('foreign registrations cannot be edited; future Windows formats stay unknown', async ({
  page,
}) => {
  const mock = await setup(page, 'conflict');
  const checkbox = page.getByRole('checkbox', { name: 'Mit Windows starten' });
  await expect(checkbox).toBeDisabled();
  await expect(page.getByText(/Es wird nichts überschrieben/)).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Autostarteintrag entfernen' }),
  ).toHaveCount(0);
  await mock.configure({ status: 'unknownApproval', registered: true });
  await page.getByRole('button', { name: 'Status prüfen' }).click();
  await expect(page.getByText(/Windows-Freigabe unbekannt/)).toBeVisible();
  await expect(checkbox).toBeDisabled();
  expect(await mutations(page)).toBe(0);
});

test('browser preview makes no Windows changes', async ({ page }) => {
  await page.goto('/');
  await openAppSettings(page);
  await expect(
    page.getByText(/Die Browser-Demo ändert keine Windows-Einstellungen/),
  ).toBeVisible();
  await expect(
    page.getByRole('checkbox', { name: 'Mit Windows starten' }),
  ).toHaveCount(0);
});

test('non-Windows native builds expose no Windows switch', async ({ page }) => {
  await setup(page, 'unsupported');
  await expect(
    page.getByText('Windows-Autostart ist nur unter Windows verfügbar.'),
  ).toBeVisible();
  await expect(
    page.getByRole('checkbox', { name: 'Mit Windows starten' }),
  ).toHaveCount(0);
  expect(await mutations(page)).toBe(0);
});
