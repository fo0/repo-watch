import { expect, test, type Page } from './support/test';
import { openPage } from './support/demo';
import { openNativeActivity, row, RUN_URL } from './support/nativeActivity';
import { readCalls } from './support/tauriMock';

// Rows render local times; a fixed zone keeps them stable.
test.use({ timezoneId: 'UTC' });

/** Replaces the clipboard with a recorder; `fail` makes writes reject. */
async function recordClipboard(page: Page, fail = false) {
  await page.addInitScript((fail) => {
    const copied: string[] = [];
    Object.assign(window, { __copied: copied });
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: async (text: string) => {
          if (fail) throw new DOMException('denied', 'NotAllowedError');
          copied.push(text);
        },
      },
    });
  }, fail);
}
const copied = (page: Page) =>
  page.evaluate(() => (window as unknown as { __copied: string[] }).__copied);
const notice = (page: Page, text: string) =>
  page.getByRole('status').filter({ hasText: text });

test('a collapsed row has no actions; the expanded one opens through the shell and copies its link', async ({
  page,
}) => {
  await recordClipboard(page);
  const mock = await openNativeActivity(page);
  const running = row(page, 8401);
  const toggle = running.locator('.run-button');
  const open = running.getByRole('link', {
    name: 'Auf GitHub öffnen',
    exact: true,
  });
  const copy = running.getByRole('button', {
    name: 'Link kopieren',
    exact: true,
  });
  // Collapsed, every row is one toggle and nothing else.
  for (const id of [8401, 8405]) {
    await expect(row(page, id).getByRole('button')).toHaveCount(1);
    await expect(row(page, id).getByRole('link')).toHaveCount(0);
  }
  await expect(toggle).toHaveAccessibleName(
    /^demo-dev\/launch-kit-demo · Desktop build #8401 · /,
  );
  await toggle.focus();
  await page.keyboard.press('Enter');
  await expect(toggle).toHaveAttribute('aria-expanded', 'true');
  // Only the expanded row has the actions, as real buttons with text.
  await expect(row(page, 8405).getByRole('link')).toHaveCount(0);
  await page.keyboard.press('Tab');
  await expect(open).toBeFocused();
  await expect(open).toHaveAttribute('href', RUN_URL);
  await page.keyboard.press('Enter');
  await expect
    .poll(() => readCalls(page, 'open_run'))
    .toEqual([{ cmd: 'open_run', args: { url: RUN_URL } }]);
  await page.keyboard.press('Tab');
  await expect(copy).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(notice(page, 'Link kopiert.')).toBeVisible();
  expect(await copied(page)).toEqual([RUN_URL]);
  await expect(toggle).toHaveAttribute('aria-expanded', 'true');
  expect(page.context().pages()).toHaveLength(1);
  // A middle click takes the same way instead of a webview window.
  await open.click({ button: 'middle' });
  await expect
    .poll(async () => (await readCalls(page, 'open_run')).length)
    .toBe(2);
  expect(page.context().pages()).toHaveLength(1);

  await mock.configure({
    openError: 'Browser konnte nicht geöffnet werden.',
  });
  await open.click();
  await expect(
    notice(page, 'Browser konnte nicht geöffnet werden.'),
  ).toBeVisible();

  // Collapsing hides the actions again.
  await toggle.click();
  await expect(open).toHaveCount(0);
  await expect(copy).toHaveCount(0);
});

test('a refused clipboard explains that the link was not copied', async ({
  page,
}) => {
  await recordClipboard(page, true);
  await openNativeActivity(page);
  const finished = row(page, 8405);
  await finished.locator('.run-button').click();
  await finished
    .getByRole('button', { name: 'Link kopieren', exact: true })
    .click();
  await expect(notice(page, 'Link konnte nicht kopiert werden.')).toBeVisible();
  await expect(notice(page, 'Link kopiert.')).toHaveCount(0);
});

test('demo runs offer both actions disabled with the demo hint', async ({
  page,
}) => {
  await page.goto('/');
  await openPage(page, 'runs');
  const running = row(page, 8401);
  await expect(running.getByRole('button')).toHaveCount(1);
  await running.locator('.run-button').click();
  for (const name of ['Auf GitHub öffnen', 'Link kopieren']) {
    const action = running.getByRole('button', { name, exact: true });
    await expect(action).toBeDisabled();
    await expect(action).toHaveAttribute(
      'title',
      'Demo-Runs haben keinen GitHub-Link',
    );
  }
  await expect(
    running.getByText('Demo-Runs haben keinen GitHub-Link.', { exact: true }),
  ).toBeVisible();
});

for (const width of [360, 480]) {
  test(`rows fit status, time and actions without overflow at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 760 });
    await openNativeActivity(page);
    await row(page, 8405).locator('.run-button').click();
    await expect(
      row(page, 8405).getByRole('button', { name: 'Link kopieren' }),
    ).toBeVisible();
    const layout = await page.evaluate(() =>
      [...document.querySelectorAll('article.run')].map((run) => {
        const box = (selector: string) =>
          run.querySelector(selector)!.getBoundingClientRect();
        const button = box('.run-button');
        const body = box('.run-body');
        const side = box('.run-side');
        const meta = box('.run-meta');
        const targets = [...run.querySelectorAll('.run-links > *')].map(
          (action) => action.getBoundingClientRect(),
        );
        return {
          rowHeight: button.height,
          sideInside:
            side.right <= button.right && side.bottom <= button.bottom,
          besideText: side.left >= body.right,
          // One meta line: as tall as its branch chip.
          metaLines: meta.height,
          actionsInside: targets.every(
            (target) =>
              target.right <= run.getBoundingClientRect().right &&
              target.bottom <= run.getBoundingClientRect().bottom,
          ),
          minTarget: Math.min(
            Infinity,
            ...targets.flatMap((target) => [target.width, target.height]),
          ),
          actions: targets.length,
        };
      }),
    );
    // Expanded 8405: open, copy and „Erneut starten …“ (write access).
    expect(layout.map((item) => item.actions)).toEqual([0, 3]);
    for (const item of layout) {
      expect(item.rowHeight).toBeLessThanOrEqual(79);
      expect(item.sideInside).toBe(true);
      expect(item.besideText).toBe(true);
      expect(item.metaLines).toBeLessThanOrEqual(22);
      expect(item.actionsInside).toBe(true);
      expect(item.minTarget).toBeGreaterThanOrEqual(24);
    }
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
    ).toBe(false);
    await page.screenshot({
      path: testInfo.outputPath(`activity-rows-${width}.png`),
    });
  });
}
