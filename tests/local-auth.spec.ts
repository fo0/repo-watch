import { expect, test, type Page } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import { checkLocal, connectDialog } from './support/connect';
import {
  type TauriMock,
  installTauriMock,
  readCalls,
} from './support/tauriMock';

type LocalMock = {
  scanDelay: number;
  empty: boolean;
  /** Counts discovery responses after the panel had a full task to apply them. */
  scansSettled: number;
};
let mock: TauriMock<LocalMock>;

test.beforeEach(async ({ page }) => {
  mock = await installTauriMock<{ fixture: typeof fixture }, LocalMock>(
    page,
    ({ fixture }, state) => {
      const empty = {
        ...structuredClone(fixture),
        accounts: [],
        repos: [],
        workflows: [],
        runs: [],
        pins: [],
        mode: 'live',
        connectivity: 'online',
        stale: false,
      };
      const accounts = [
        {
          source: 'gh',
          host: 'github.com',
          login: 'fixture-alex',
          usable: true,
          active: true,
          detail: 'API-Anmeldung bestätigt',
          actionsRead: 'Scope repo vorhanden; Repository-Zugriff ungeprüft.',
          actionsWrite: 'Actions starten: pro Repository ungeprüft.',
        },
        {
          source: 'gh',
          host: 'github.com',
          login: 'fixture-sam',
          usable: true,
          active: false,
          detail: 'API-Anmeldung bestätigt',
          actionsRead: 'Scopes nicht auslesbar.',
          actionsWrite: 'Actions starten: pro Repository ungeprüft.',
        },
        {
          source: 'gcm',
          host: 'github.com',
          login: 'fixture-gcm',
          usable: false,
          active: false,
          detail: 'Git-Konto; API-Übernahme nicht unterstützt.',
          actionsRead: 'Git belegt keine Actions-API-Rechte.',
          actionsWrite: 'Nicht nachgewiesen.',
        },
      ];
      return async (cmd, args) => {
        if (cmd === 'snapshot' || cmd === 'refresh_snapshot')
          return structuredClone(empty);
        if (cmd === 'discover_local_auth') {
          await new Promise((resolve) => setTimeout(resolve, state.scanDelay));
          // Count it one task later, once the panel has consumed the result.
          setTimeout(() => state.scansSettled++, 0);
          return {
            accounts: state.empty ? [] : accounts,
            notices: [
              'Git erkannt; keine API-Anmeldung belegt.',
              'GCM-Credentials werden nicht exportiert.',
            ],
          };
        }
        if (cmd === 'connect_local_auth') {
          if (args?.login !== 'fixture-sam' || args?.confirmImport !== true)
            throw 'Selection mismatch';
          return {
            ...structuredClone(fixture),
            mode: 'live',
            connectivity: 'online',
            stale: false,
            accounts: [
              {
                id: 'github-gh-42',
                login: 'fixture-sam',
                label: 'GitHub · gh CLI (Kopie)',
              },
            ],
          };
        }
        return null;
      };
    },
    {
      arg: { fixture },
      state: { scanDelay: 0, empty: false, scansSettled: 0 },
    },
  );
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'GitHub-Konto verbinden', exact: true })
    .click();
});

const found = (page: Page) =>
  connectDialog(page).getByRole('radiogroup', { name: 'Gefundene Konten' });
const take = (page: Page) =>
  connectDialog(page).getByRole('button', { name: 'Übernehmen', exact: true });
const CONSENT =
  'Das Token von fixture-sam wird aus der GitHub CLI übernommen und im Windows-Tresor gespeichert.';

test('local account access needs the check, an explicit choice and „Übernehmen“ before any secret import', async ({
  page,
}) => {
  const calls = () => readCalls(page);
  const dialog = connectDialog(page);
  // The method step reads nothing local, even with the local method chosen.
  await expect(
    dialog.getByRole('radio', {
      name: /^Bestehende Anmeldung auf diesem PC verwenden/,
    }),
  ).toBeChecked();
  expect((await calls()).some((c) => c.cmd.includes('local_auth'))).toBe(false);
  await checkLocal(page);
  // Two importable accounts: nothing is preselected; GCM is listed but off.
  const list = found(page);
  await expect(list.getByRole('radio')).toHaveCount(3);
  await expect(list.getByRole('radio', { checked: true })).toHaveCount(0);
  await expect(list.getByRole('radio', { name: /fixture-gcm/ })).toBeDisabled();
  await expect(list).toContainText(
    'Git Credential Manager · nicht übernehmbar',
  );
  await expect(take(page)).toBeDisabled();
  await expect(take(page)).toHaveAttribute('title', 'Konto auswählen');
  await expect(dialog.locator('#local-consent')).toHaveCount(0);
  await list.getByRole('radio', { name: /fixture-sam/ }).check();
  // Exactly one consent line, naming the account and where its token goes.
  await expect(dialog.locator('#local-consent')).toHaveText(CONSENT);
  await expect(take(page)).toHaveAccessibleDescription(CONSENT);
  expect((await calls()).some((c) => c.cmd === 'connect_local_auth')).toBe(
    false,
  );
  await take(page).click();
  // Success closes the dialog by itself; a new account lands on the overview.
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.tray')).toHaveAttribute('data-page', 'overview');
  await expect(
    page
      .getByRole('status')
      .filter({ hasText: 'Konto fixture-sam verbunden.' }),
  ).toBeVisible();
  expect(
    (await calls()).find((c) => c.cmd === 'connect_local_auth')?.args,
  ).toEqual({ login: 'fixture-sam', confirmImport: true });
  expect(
    JSON.stringify(await page.evaluate(() => ({ ...localStorage }))),
  ).not.toContain('fixture-sam');
  expect(
    (await calls()).some(
      (c) => c.cmd === 'connect_token' || c.cmd === 'dispatch_workflow',
    ),
  ).toBe(false);
});

test('„Zurück“ cancels a running check and discards its late result; an empty result is one line with „Erneut prüfen“', async ({
  page,
}) => {
  await mock.configure({ scanDelay: 400 });
  await checkLocal(page);
  const dialog = connectDialog(page);
  await expect(
    dialog.getByRole('button', { name: 'Wird geprüft …', exact: true }),
  ).toBeDisabled();
  await dialog.getByRole('button', { name: 'Zurück', exact: true }).click();
  await expect(
    dialog.getByRole('heading', { name: 'Anmeldeart wählen' }),
  ).toBeFocused();
  expect((await readCalls(page, 'cancel_connect')).length).toBeGreaterThan(0);
  // The absence check proves nothing until the late result was delivered.
  await expect.poll(() => mock.read('scansSettled')).toBe(1);
  await expect(found(page)).toHaveCount(0);
  await mock.configure({ scanDelay: 0, empty: true });
  await checkLocal(page);
  await expect(dialog.locator('.modal-body')).toContainText(
    'Keine gh-Anmeldung gefunden. Im Terminal mit „gh auth login“ anmelden.',
  );
  await expect(take(page)).toHaveCount(0);
  // Discovery notes stay out of the dialog: no help walls.
  await expect(
    dialog.getByText('Git erkannt; keine API-Anmeldung belegt.'),
  ).toHaveCount(0);
  await mock.configure({ empty: false });
  await dialog
    .getByRole('button', { name: 'Erneut prüfen', exact: true })
    .click();
  await expect(found(page).getByRole('radio')).toHaveCount(3);
});

test('a gh login RepoWatch already holds is marked, and taking it over again needs the same explicit „Übernehmen“', async ({
  page,
}, testInfo) => {
  await checkLocal(page);
  await found(page)
    .getByRole('radio', { name: /fixture-sam/ })
    .check();
  await take(page).click();
  // A new account lands on the overview with one short notice.
  await expect(page.locator('.tray')).toHaveAttribute('data-page', 'overview');
  await expect(
    page
      .getByRole('status')
      .filter({ hasText: 'Konto fixture-sam verbunden.' }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Einstellungen', exact: true })
    .click();
  const accounts = page
    .getByRole('dialog', { name: 'Einstellungen' })
    .getByRole('radiogroup', { name: 'Aktives Konto' });
  await expect(
    accounts.getByRole('radio', {
      name: 'fixture-sam · gh CLI (Kopie)',
      exact: true,
    }),
  ).toBeChecked();
  await expect(accounts.locator('.account-tag')).toHaveText('Aktiv');
  await page
    .getByRole('button', { name: 'Konto hinzufügen', exact: true })
    .click();
  await checkLocal(page);
  const sam = found(page).locator('label').filter({ hasText: 'fixture-sam' });
  await expect(sam).toContainText('GitHub CLI · bereits verbunden');
  await expect(take(page)).toBeDisabled();
  await sam.getByRole('radio').check();
  await expect(connectDialog(page).locator('#local-consent')).toHaveText(
    CONSENT,
  );
  await expect(take(page)).toBeEnabled();
  await page.setViewportSize({ width: 360, height: 840 });
  expect(
    await connectDialog(page).evaluate(
      (node) => node.scrollWidth <= node.clientWidth,
    ),
  ).toBe(true);
  await connectDialog(page).screenshot({
    path: testInfo.outputPath('connected-gh-feedback-360.png'),
  });
});

test('Enter on a method continues like the primary button: the local check starts, a token opens its form', async ({
  page,
}) => {
  const dialog = connectDialog(page);
  const local = dialog.getByRole('radio', {
    name: /^Bestehende Anmeldung auf diesem PC verwenden/,
  });
  await local.focus();
  await page.keyboard.press('Enter');
  await expect(found(page).getByRole('radio')).toHaveCount(3);
  expect(
    (await readCalls(page)).filter((c) => c.cmd === 'discover_local_auth'),
  ).toHaveLength(1);
  await dialog.getByRole('button', { name: 'Zurück', exact: true }).click();
  const token = dialog.getByRole('radio', { name: /^Mit Token verbinden/ });
  await token.check();
  await page.keyboard.press('Enter');
  await expect(
    dialog.getByRole('heading', { name: 'Mit Token verbinden' }),
  ).toBeFocused();
  await expect(dialog.getByLabel('GitHub-Token')).toBeVisible();
});
