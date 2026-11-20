import { describe, expect, it, vi } from 'vitest';
import { PanelView, type RunReveal } from './view.svelte';

function setup() {
  const panel: RunReveal = {
    expandedRun: null,
    notices: { dismissRun: vi.fn() },
  };
  return { panel, view: new PanelView(panel) };
}

describe('PanelView', () => {
  it('starts on the overview with an unfiltered, empty search', () => {
    const { view } = setup();
    expect(view.page).toBe('overview');
    expect(view.filter).toBe('all');
    expect(view.query).toBe('');
    expect(view.back()).toBe(false);
  });

  it('opens a section page and goes back to the overview', async () => {
    const { view } = setup();
    await view.open('favorites');
    expect(view.page).toBe('favorites');
    expect(view.back()).toBe(true);
    expect(view.page).toBe('overview');
    expect(view.back()).toBe(false);
  });

  it('keeps the first opener when the open page is opened again', async () => {
    const { view } = setup();
    const opener = {} as HTMLElement;
    await view.open('runs', { opener });
    await view.open('runs', { opener: null });
    expect(view.opener).toBe(opener);
    view.back();
    expect(view.opener).toBeNull();
  });

  it('toggles a page: its own key returns to the overview', async () => {
    const { view } = setup();
    view.toggle('repos');
    expect(view.page).toBe('repos');
    view.toggle('runs');
    expect(view.page).toBe('runs');
    view.toggle('runs');
    expect(view.page).toBe('overview');
  });

  it('returns to the overview from a page opened on another page', async () => {
    const { view } = setup();
    await view.open('repos');
    await view.open('favorites');
    expect(view.back()).toBe(true);
    expect(view.page).toBe('overview');
    expect(view.back()).toBe(false);
  });

  it('goes home to the overview from any page, forgetting the opener', async () => {
    const { view } = setup();
    await view.open('favorites', { opener: {} as HTMLElement });
    view.home();
    expect(view.page).toBe('overview');
    expect(view.opener).toBeNull();
    expect(view.back()).toBe(false);
  });

  it('keeps the search and the filter across page switches', async () => {
    const { view } = setup();
    view.query = 'kit';
    view.filter = 'failed';
    await view.open('runs');
    view.back();
    await view.open('favorites');
    expect(view.query).toBe('kit');
    expect(view.filter).toBe('failed');
  });

  it('„+N weitere“ under the runs keeps a filter that lists open runs', async () => {
    const { view } = setup();
    for (const [before, after] of [
      ['all', 'all'],
      ['active', 'active'],
      ['failed', 'active'],
      ['recent', 'active'],
    ] as const) {
      view.filter = before;
      await view.showMore('runs');
      expect(view.page).toBe('runs');
      expect(view.filter).toBe(after);
      view.back();
    }
    // Other sections leave the Verlauf filter alone.
    view.filter = 'failed';
    await view.showMore('favorites');
    expect(view.page).toBe('favorites');
    expect(view.filter).toBe('failed');
  });

  it('shows a run in the unfiltered Verlauf with its details open', async () => {
    const { panel, view } = setup();
    view.query = 'kit';
    view.filter = 'failed';
    await view.showRun(9001);
    expect(view.page).toBe('runs');
    expect(view.filter).toBe('all');
    expect(view.query).toBe('');
    expect(panel.expandedRun).toBe(9001);
    expect(panel.notices.dismissRun).toHaveBeenCalledWith(9001);
    expect(view.back()).toBe(true);
    expect(view.page).toBe('overview');
  });

  it('shows a repository’s runs in the unfiltered Verlauf by searching for it', async () => {
    const { panel, view } = setup();
    view.query = 'kit';
    view.filter = 'failed';
    await view.showRepoRuns('demo-dev/website-demo');
    expect(view.page).toBe('runs');
    expect(view.filter).toBe('all');
    expect(view.query).toBe('demo-dev/website-demo');
    expect(panel.expandedRun).toBeNull();
    expect(view.back()).toBe(true);
    expect(view.page).toBe('overview');
  });

  it('clears only the search when the account changes', async () => {
    const { view } = setup();
    await view.open('repos');
    view.query = 'kit';
    view.filter = 'active';
    view.reset();
    expect(view.query).toBe('');
    expect(view.filter).toBe('active');
    expect(view.page).toBe('repos');
  });
});
