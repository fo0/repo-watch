import { expect, test, type Page } from './support/test';
import { openDemoActivity } from './support/demo';

const run = (page: Page, id: number) =>
  page.locator('article.run').filter({ hasText: `#${id}` });

test('„Erneut starten …“ opens the start dialog with the run’s workflow and branch', async ({
  page,
}) => {
  await openDemoActivity(page);
  const done = run(page, 8405);
  const again = done.getByRole('button', { name: 'Erneut starten …' });
  // Collapsed, the row offers no action.
  await expect(again).toHaveCount(0);
  await done.locator('.run-button').click();
  await expect(again).toHaveAttribute(
    'title',
    'Startdialog mit diesem Workflow und Branch öffnen',
  );
  await again.click();
  const dialog = page.getByRole('dialog', { name: 'Workflow starten' });
  await expect(
    dialog.getByRole('combobox', { name: 'Repository' }),
  ).toHaveValue('demo-personal-kit');
  await expect(dialog.getByRole('combobox', { name: 'Workflow' })).toHaveValue(
    '101',
  );
  // The run's tag, not the default branch.
  await expect(
    dialog.getByRole('combobox', { name: 'Branch oder Tag' }),
  ).toHaveValue('v0.1.0');
  // Opening starts nothing; closing returns focus to the run's button.
  await dialog.getByRole('button', { name: 'Abbrechen', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(again).toBeFocused();
});

test('runs that cannot be started again offer no „Erneut starten …“', async ({
  page,
}) => {
  await openDemoActivity(page);
  // 8402: a workflow without workflow_dispatch; 8404: a read-only repository.
  for (const id of [8402, 8404]) {
    const row = run(page, id);
    await row.locator('.run-button').click();
    await expect(
      row.getByRole('button', { name: 'Link kopieren', exact: true }),
    ).toBeVisible();
    await expect(
      row.getByRole('button', { name: 'Erneut starten …' }),
    ).toHaveCount(0);
  }
});
