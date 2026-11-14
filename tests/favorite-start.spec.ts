import { expect, test } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import { installTauriMock, readCalls } from './support/tauriMock';
import { openDemoActivity, openPage, templateCard } from './support/demo';

type Page = Parameters<typeof templateCard>[0];

/** A favorite row's buttons, by their accessible names. */
const button = (page: Page, label: string, name: string) =>
  templateCard(page, label).getByRole('button', {
    name: `launch-kit-demo · ${label} ${name}`,
    exact: true,
  });

const claim = {
  accountId: 'demo-personal',
  repoId: 'demo-personal-kit',
  workflowId: 101,
  ref: 'main',
  claimedAt: '2026-11-01T08:00:00Z',
};

/**
 * Labelled IPC mock of the native shell on the Favoriten page: the stored
 * favorites lose their required `version` (`incomplete`), a persisted claim
 * locks `main` (`locked`), and only `accept` lets a dispatch succeed.
 */
async function nativeFavorites(
  page: Page,
  options: { incomplete?: boolean; locked?: boolean; accept?: boolean },
) {
  await installTauriMock<
    { fixture: typeof fixture; claim: typeof claim; options: typeof options },
    object
  >(
    page,
    ({ fixture, claim, options }) => {
      const snapshot = {
        ...structuredClone(fixture),
        mode: 'live',
        stale: false,
        connectivity: 'online',
        message: undefined,
        uncertainStarts: options.locked ? [claim] : [],
        pins: fixture.pins.map((pin) =>
          options.incomplete
            ? {
                ...pin,
                inputs: {
                  target: pin.inputs.target,
                  dry_run: pin.inputs.dry_run,
                },
              }
            : pin,
        ),
      };
      return (cmd) => {
        if (cmd === 'snapshot' || cmd === 'refresh_snapshot') return snapshot;
        if (cmd === 'dispatch_workflow') {
          if (!options.accept) throw 'Mock: Dieser Test startet nichts.';
          return { state: 'accepted', runId: 9001, message: 'Mock-Run #9001.' };
        }
        return null;
      };
    },
    { arg: { fixture, claim, options }, state: {} },
  );
  await page.addInitScript(() =>
    localStorage.setItem(
      'repowatch.activeAccount.v1',
      JSON.stringify('demo-personal'),
    ),
  );
  await page.goto('/');
  await openPage(page, 'favorites');
}

const dispatches = async (page: Page) =>
  (await readCalls(page, 'dispatch_workflow')).length;

test('a favorite that still needs input opens the start dialog instead of starting', async ({
  page,
}) => {
  await nativeFavorites(page, { incomplete: true });
  await button(page, 'Windows build', 'jetzt starten').click();
  const dialog = page.getByRole('dialog', { name: 'Workflow starten' });
  const version = dialog.getByLabel('Version', { exact: true });
  await expect(version).toHaveValue('');
  // The field that blocks the start is marked and names the reason.
  await expect(version).toHaveAttribute('aria-invalid', 'true');
  await expect(version).toHaveAccessibleDescription(
    'Version ist erforderlich.',
  );
  await expect(dialog.getByLabel('Branch oder Tag')).toHaveValue('main');
  // The favorite section is open to fix and update the stored defaults.
  await expect(
    dialog.getByRole('button', { name: 'Favorit aktualisieren', exact: true }),
  ).toBeVisible();
  await expect(dialog.getByLabel('Name des Favoriten')).toHaveValue(
    'Windows build',
  );
  await version.fill('1.2.3');
  await expect(version).not.toHaveAttribute('aria-invalid');
  expect(await readCalls(page, 'dispatch_workflow')).toEqual([]);
});

test('an uncertain claim locks the direct start and sends nothing', async ({
  page,
}) => {
  await nativeFavorites(page, { locked: true });
  const direct = button(page, 'Windows build', 'jetzt starten');
  await expect(direct).toBeDisabled();
  await expect(direct).toHaveAccessibleDescription(
    'Startstatus ungeklärt – prüfe die Runs auf GitHub',
  );
  await direct.click({ force: true });
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(await readCalls(page, 'dispatch_workflow')).toEqual([]);
});

test('a held Enter or a double-click starts the favorite once', async ({
  page,
}) => {
  await nativeFavorites(page, { accept: true });
  await page.clock.install();
  const start = button(page, 'Windows build', 'jetzt starten');
  const notice = page
    .getByRole('status')
    .filter({ hasText: /Windows build: Mock-Run #9001/ });
  await start.focus();
  await page.keyboard.down('Enter');
  await expect(notice).toBeVisible();
  await expect(start).not.toHaveAttribute('aria-disabled');
  expect(await dispatches(page)).toBe(1);
  // Past the cooldown after the start settled, the held key keeps repeating.
  await page.clock.runFor(2000);
  for (let repeat = 0; repeat < 5; repeat++) await page.keyboard.down('Enter');
  await page.keyboard.up('Enter');
  await expect(start).toBeFocused();
  expect(await dispatches(page)).toBe(1);
  // A double-click is one start; its second click never sends again.
  await page.clock.runFor(2000);
  await start.dblclick();
  await expect.poll(() => dispatches(page)).toBe(2);
  await page.clock.runFor(2000);
  expect(await dispatches(page)).toBe(2);
});

test('the row controls: name, settings, heart, then the start at the far right', async ({
  page,
}) => {
  await openDemoActivity(page);
  await openPage(page, 'overview');
  const row = templateCard(page, 'Windows build');
  // Every one a real button with its own name, the start reachable by
  // keyboard; „…“ opens the dialog, „jetzt“ sends at once.
  expect(
    await row
      .getByRole('button')
      .evaluateAll((buttons) =>
        buttons.map((button) => button.getAttribute('aria-label')),
      ),
  ).toEqual([
    'launch-kit-demo · Windows build starten…',
    'launch-kit-demo · Windows build bearbeiten',
    'launch-kit-demo · Windows build aus Favoriten entfernen',
    'launch-kit-demo · Windows build jetzt starten',
  ]);
  const start = row.locator('.pin-start');
  await expect(start).not.toHaveAttribute('aria-hidden');
  await expect(start).not.toHaveAttribute('tabindex');
  await expect(start).toHaveAttribute('title', 'Starten');
  const [startBox, editBox, rowBox] = await Promise.all([
    start.boundingBox(),
    row.locator('.pin-edit').boundingBox(),
    row.boundingBox(),
  ]);
  expect(startBox!.x).toBeGreaterThan(editBox!.x);
  expect(
    rowBox!.x + rowBox!.width - (startBox!.x + startBox!.width),
  ).toBeLessThan(8);
});

test('the settings button edits the defaults the direct start then sends', async ({
  page,
}) => {
  await openDemoActivity(page);
  await openPage(page, 'favorites');
  const edit = button(page, 'Windows build', 'bearbeiten');
  await expect(edit).toHaveAttribute('title', 'Standardwerte bearbeiten');
  await edit.click();
  const dialog = page.getByRole('dialog', { name: 'Workflow starten' });
  await expect(dialog.getByLabel('Version', { exact: true })).toHaveValue(
    '0.1.0',
  );
  await dialog.getByLabel('Branch oder Tag').selectOption('develop');
  await dialog.getByLabel('Name des Favoriten').fill('Develop build');
  await dialog
    .getByRole('button', { name: 'Favorit aktualisieren', exact: true })
    .click();
  await expect(
    dialog.getByText('„Develop build“ als Favorit gespeichert.', {
      exact: true,
    }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();

  // Updated in place, not copied: one row, now on develop.
  await expect(page.locator('.pin-row')).toHaveCount(1);
  const row = templateCard(page, 'Develop build');
  await expect(row.locator('.branch-chip')).toHaveText('develop');
  await button(page, 'Develop build', 'jetzt starten').click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(
    page.getByRole('status').filter({ hasText: /Develop build: .*#9001/ }),
  ).toBeVisible();
});
