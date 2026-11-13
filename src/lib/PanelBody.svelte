<script lang="ts">
  import type { Snippet } from 'svelte';
  import ActivityView from './ActivityView.svelte';
  import EmptyState, { type NextStep } from './EmptyState.svelte';
  import FavoritesPage from './FavoritesPage.svelte';
  import Icon from './Icon.svelte';
  import Overview from './Overview.svelte';
  import ReposPage from './ReposPage.svelte';
  import SectionPage from './SectionPage.svelte';
  import { m } from './i18n/index.svelte';
  import type { Modals } from './modals.svelte';
  import type { Panel } from './panel.svelte';
  import { repositoryActivity } from './repoActivity';
  import { historyRuns, visibleRuns } from './runs';
  import { matchesTerms, searchTerms } from './search';
  import { pageTitle, type PanelView } from './view.svelte';

  let {
    panel,
    modals,
    view,
    repoName,
    accountName,
    repoAccountName,
    onClearQuery,
    onNextStep,
    top,
  }: {
    panel: Panel;
    modals: Modals;
    view: PanelView;
    repoName: (repoId: string) => string;
    accountName: (accountId: string) => string;
    repoAccountName: (repoId: string) => string;
    /** Empties the search and returns focus to the field. */
    onClearQuery: () => void;
    /** Opens the missing step: connect or account. */
    onNextStep: () => void;
    /** Search and notices: one instance above every page, never remounted. */
    top: Snippet;
  } = $props();

  let content = $state<HTMLElement>();
  const snapshot = $derived(panel.snapshot);
  /** Native mode shows no repository data before an account is chosen. */
  const data = $derived(panel.viewSnapshot);
  const accountId = $derived(panel.accountId);
  const query = $derived(view.query);
  /** Whitespace alone filters nothing, so it is no search on any page. */
  const searching = $derived(query.trim() !== '');
  const live = $derived(
    data ? visibleRuns(data, accountId, query, 'active') : [],
  );
  const runs = $derived(
    data ? historyRuns(data, accountId, query, view.filter) : [],
  );
  const count = (filter: 'active' | 'failed') =>
    data ? visibleRuns(data, accountId, '', filter).length : 0;
  const pins = $derived.by(() => {
    const terms = searchTerms(query);
    return (
      snapshot?.pins.filter(
        (pin) =>
          panel.repos.some((repo) => repo.id === pin.repoId) &&
          matchesTerms(`${pin.label} ${repoName(pin.repoId)}`, terms),
      ) ?? []
    );
  });
  const groups = $derived(
    data ? repositoryActivity(data, accountId, query) : [],
  );
  /** The same next step on every page while no account is chosen. */
  const setup = $derived.by((): NextStep | null => {
    const hasAccounts = !!snapshot?.accounts.length;
    if (!panel.needsAccount) return null;
    const text = m().shell.setup;
    return hasAccounts
      ? {
          title: text.pickTitle,
          text: panel.accountNotice || text.pickText,
          action: { label: text.pickAction, icon: 'down', run: onNextStep },
        }
      : {
          title: text.connectTitle,
          text: panel.accountNotice || text.connectText,
          action: { label: text.connectAction, icon: 'plus', run: onNextStep },
        };
  });

  // A new page starts at its top; going back scrolls to the focused opener.
  $effect(() => {
    void view.page;
    if (content) content.scrollTop = 0;
  });
</script>

<!-- Keyed by page: Ctrl+1/2/3 from one page to another focuses a new title,
     which assistive tech announces; a reused, already focused one is silent. -->
{#if view.page !== 'overview'}{#key view.page}
    <SectionPage
      page={view.page}
      title={pageTitle(view.page)}
      count={view.page === 'runs'
        ? runs.length
        : view.page === 'favorites'
          ? pins.length
          : groups.length}
      onBack={view.back}
    >
      {#if view.page === 'favorites'}<button
          class="add-button"
          onclick={() => modals.openDispatch()}
          disabled={!panel.repos.length}
          ><Icon name="play" size={12} />{m().shell.overview
            .startWorkflow}</button
        >{/if}
    </SectionPage>
  {/key}{/if}
<div class="content" bind:this={content}>
  {@render top()}
  {#if !snapshot}
    <EmptyState
      icon="tray"
      iconSize={26}
      title={!panel.error
        ? m().shell.loading.title
        : panel.demo
          ? m().shell.loading.demoFailed
          : m().shell.loading.failed}
      text={!panel.error
        ? m().shell.loading.text
        : panel.demo
          ? m().shell.loading.demoFailedText
          : m().shell.loading.failedText}
    />
  {:else if view.page === 'overview'}
    <Overview
      {panel}
      {modals}
      {view}
      {live}
      {pins}
      {groups}
      {repoName}
      {accountName}
      {searching}
      {setup}
      {onClearQuery}
    />
  {:else}
    <section class="page-body" aria-labelledby="page-title">
      {#if view.page === 'runs'}
        <ActivityView
          {runs}
          filter={view.filter}
          expanded={panel.expandedRun}
          activeRuns={count('active')}
          failedRuns={count('failed')}
          demo={panel.demo}
          native={panel.native}
          {setup}
          {searching}
          {repoName}
          {repoAccountName}
          canCancel={panel.canCancel}
          isCancelling={panel.actions.isCancelling}
          onCancel={(run) => void panel.actions.cancelRun(run)}
          onFilter={(next) => (view.filter = next)}
          onToggle={panel.toggleRun}
          {onClearQuery}
          onNotice={(text, tone) => panel.showNotice(text, tone)}
        />
      {:else if view.page === 'favorites'}
        <FavoritesPage
          {panel}
          {modals}
          {pins}
          {repoName}
          {accountName}
          {searching}
          {setup}
          {onClearQuery}
        />
      {:else}
        <ReposPage
          {panel}
          {modals}
          {groups}
          {searching}
          {setup}
          {onClearQuery}
        />
      {/if}
    </section>
  {/if}
</div>
