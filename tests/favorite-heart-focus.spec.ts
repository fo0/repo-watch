import { expect, test, type Page } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import { installTauriMock } from './support/tauriMock';
import { openPage, templateCard } from './support/demo';

/**
 * Labelled IPC mock of the native shell: a favorite change answers after
 * 350 ms, like a real IPC round trip, so the heart stays busy meanwhile.
 */
async function nativePins(page: Page) {
  await installTauriMock<{ fixture: typeof fixture }, object>(
    page,
    ({ fixture }) => {
      const snapshot = {
        ...structuredClone(fixture),
        mode: 'live',
        stale: false,
        connectivity: 'online',
        message: undefined,
        uncertainStarts: [],
      };
      const later = () => new Promise((resolve) => setTimeout(resolve, 350));
      return async (cmd, args) => {
        if (cmd === 'snapshot' || cmd === 'refresh_snapshot')
          return structuredClone(snapshot);
        if (cmd === 'save_pin') {
          await later();
          const pin = args?.pin as (typeof snapshot.pins)[number];
          snapshot.pins = [
            ...snapshot.pins.filter((p) => p.id !== pin.id),
            pin,
          ];
          return structuredClone(snapshot);
        }
        if (cmd === 'delete_pin') {
          await later();
          snapshot.pins = snapshot.pins.filter((p) => p.id !== args?.pinId);
          return structuredClone(snapshot);
        }
        return null;
      };
    },
    { arg: { fixture }, state: {} },
  );
  await page.addInitScript(() =>
    localStorage.setItem(
      'repowatch.activeAccount.v1',
      JSON.stringify('demo-personal'),
    ),
  );
  await page.goto('/');
}

test('a workflow heart keeps focus while its favorite change runs', async ({
  page,
}) => {
  await nativePins(page);
  await openPage(page, 'overview');
  const kit = page.getByRole('region', {
    name: 'demo-dev/launch-kit-demo',
    exact: true,
  });
  await kit.locator('.repository-toggle').click();
  const heart = kit.getByRole('button', { name: /^Desktop build (aus|als)/ });
  await expect(heart).toHaveAttribute('aria-pressed', 'true');
  await heart.focus();
  await page.keyboard.press('Enter');
  await expect(heart).toHaveAttribute('aria-disabled', 'true');
  await expect(heart).toBeFocused();
  await expect(heart).toHaveAttribute('aria-pressed', 'false');
  await expect(heart).not.toHaveAttribute('aria-disabled');
  await expect(heart).toBeFocused();
  // A press while the change runs changes nothing.
  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');
  await expect(heart).toHaveAttribute('aria-pressed', 'false');
});

test('a favorite row heart keeps focus while the removal runs, then hands it on', async ({
  page,
}) => {
  await nativePins(page);
  await openPage(page, 'favorites');
  const heart = templateCard(page, 'Windows build').getByRole('button', {
    name: 'launch-kit-demo · Windows build aus Favoriten entfernen',
    exact: true,
  });
  await heart.focus();
  await page.keyboard.press('Enter');
  await expect(heart).toHaveAttribute('aria-disabled', 'true');
  await expect(heart).toBeFocused();
  await expect(heart).toHaveCount(0);
  // No row is left: focus lands on the page's title, never on <body>.
  await expect
    .poll(() => page.evaluate(() => document.activeElement !== document.body))
    .toBe(true);
});
