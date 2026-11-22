import { expect, test, type Page } from './support/test';
import fixture from '../fixtures/demo.json' with { type: 'json' };
import { accountMenuButton, scenario } from './support/demo';
import { installTauriMock } from './support/tauriMock';

const banner = (page: Page) => page.locator('.alert.warning');
const dot = (page: Page) => accountMenuButton(page).locator('.presence');

test.describe('browser demo', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(accountMenuButton(page)).toBeEnabled();
  });

  test('a green dot keeps the plain tooltip and adds no description', async ({
    page,
  }) => {
    const trigger = accountMenuButton(page);
    await expect(dot(page)).toHaveClass(/\bok\b/);
    await expect(trigger).toHaveAttribute('title', 'Alle Konten');
    await expect(trigger).not.toHaveAttribute('aria-describedby');
    await expect(banner(page)).toHaveCount(0);
  });

  for (const [value, presence] of [
    ['offline', 'wait'],
    ['forbidden', 'fail'],
  ] as const) {
    test(`a ${presence} dot explains itself with the banner text (${value})`, async ({
      page,
    }) => {
      await scenario(page, value);
      const trigger = accountMenuButton(page);
      await expect(dot(page)).toHaveClass(new RegExp(`\\b${presence}\\b`));
      const reason = (await banner(page).textContent())?.trim() ?? '';
      expect(reason).not.toBe('');
      await expect(trigger).toHaveAttribute('title', `Alle Konten\n${reason}`);
      await expect(trigger).toHaveAccessibleDescription(reason);
      // The accessible name stays as it was; nothing visible grows.
      await expect(trigger).toHaveAccessibleName(
        'Konto: Alle Konten – Konto wechseln',
      );
      await expect(trigger.locator('.account-name')).toHaveText('Alle Konten');
      // Back online, the reason goes away again.
      await scenario(page, 'normal');
      await expect(dot(page)).toHaveClass(/\bok\b/);
      await expect(trigger).toHaveAttribute('title', 'Alle Konten');
      await expect(trigger).not.toHaveAttribute('aria-describedby');
    });
  }
});

test('natively without a snapshot message the dot names a generic reason', async ({
  page,
}) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      'repowatch.activeAccount.v1',
      JSON.stringify('github-42'),
    ),
  );
  // Labelled IPC mock: one synthetic account, offline, no message.
  await installTauriMock<{ fixture: typeof fixture }, Record<string, never>>(
    page,
    ({ fixture }) => {
      const account = {
        id: 'github-42',
        login: 'fixture-user',
        label: 'GitHub · Token',
      };
      const snapshot = {
        ...structuredClone(fixture),
        mode: 'live',
        stale: true,
        connectivity: 'offline',
        message: undefined,
        accounts: [account],
        repos: [],
        workflows: [],
        runs: [],
        pins: [],
      };
      return (cmd) => {
        if (cmd === 'snapshot' || cmd === 'refresh_snapshot')
          return structuredClone(snapshot);
        if (cmd === 'browser_auth_config') return { configured: true };
        if (cmd === 'discover_local_auth') return { accounts: [], notices: [] };
        return null;
      };
    },
    { arg: { fixture }, state: {} },
  );
  await page.goto('/');
  const trigger = accountMenuButton(page);
  await expect(trigger).toBeEnabled();
  await expect(dot(page)).toHaveClass(/\bwait\b/);
  const label = (await trigger.locator('.account-name').textContent()) ?? '';
  await expect(trigger).toHaveAttribute(
    'title',
    `${label}\nOffline · letzter bekannter Stand`,
  );
  await expect(trigger).toHaveAccessibleDescription(
    'Offline · letzter bekannter Stand',
  );
  await expect(banner(page)).toBeVisible();
});
