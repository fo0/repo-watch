<script module lang="ts">
  import type { SectionPage } from './view.svelte';

  /** Rows per section before „+N weitere“ leads to the maximized page. */
  export const CAPS: Record<SectionPage, number> = {
    runs: 3,
    favorites: 4,
    repos: 10,
  };
</script>

<script lang="ts">
  import EmptyState, { type NextStep } from './EmptyState.svelte';
  import FavoriteTiles from './FavoriteTiles.svelte';
  import Icon from './Icon.svelte';
  import LiveRuns from './LiveRuns.svelte';
  import RepositoryList from './RepositoryList.svelte';
  import SectionHeader from './SectionHeader.svelte';
  import { m } from './i18n/index.svelte';
  import type { Modals } from './modals.svelte';
  import { fullRepo } from './model';
  import type { Panel } from './panel.svelte';
  import { lastRun, type repositoryActivity } from './repoActivity';
  import type { Pin, Run } from './types';
  import type { PanelView } from './view.svelte';

  let {
    panel,
    modals,
    view,
    live,
    pins,
    groups,
    repoName,
    accountName,
    searching,
    setup,
    onClearQuery,
  }: {
    /** Starts, favorites and account state come from the panel. */
    panel: Panel;
    modals: Modals;
    view: PanelView;
    /** Open runs matching the search. */
    live: Run[];
    /** Favorites matching the search. */
    pins: Pin[];
    groups: ReturnType<typeof repositoryActivity>;
    repoName: (repoId: string) => string;
    accountName: (accountId: string) => string;
    /** A non-blank search query is active; it lifts the caps. */
    searching: boolean;
    /** The missing account step, shown instead of the sections. */
    setup: NextStep | null;
    onClearQuery: () => void;
  } = $props();

  const capped = <T,>(items: T[], page: SectionPage) =>
    searching ? items : items.slice(0, CAPS[page]);
  const shownLive = $derived(capped(live, 'runs'));
  const shownPins = $derived(capped(pins, 'favorites'));
  const shownGroups = $derived(capped(groups, 'repos'));
  const pinRun = (pin: Pin) =>
    panel.snapshot ? lastRun(panel.snapshot, pin.repoId, pin.workflowId) : null;
</script>

{#snippet more(page: SectionPage, hidden: number)}
  {#if hidden > 0}<button
      class="text-button more-button"
      data-focus-key={`more:${page}`}
      aria-label={m().shell.sections.moreLabel[page](hidden)}
      onclick={() => void view.showMore(page)}
      >{m().shell.sections.more(hidden)}<Icon
        name="maximize"
        size={12}
      /></button
    >{/if}
{/snippet}

{#if setup}<EmptyState icon="tray" iconSize={25} {...setup} />
{:else if searching && !live.length && !pins.length && !groups.length}
  <EmptyState
    icon="search"
    iconSize={25}
    title={m().shell.overview.noMatches}
    text={m().common.tryOtherSearch}
    action={{
      label: m().common.clearSearch,
      icon: 'close',
      run: onClearQuery,
    }}
  />
{:else}
  <section class="overview-section" aria-labelledby="section-runs">
    <SectionHeader
      page="runs"
      id="section-runs"
      title={m().shell.sections.runs}
      count={live.length}
      live={live.length > 0}
      onMaximize={() => void view.open('runs')}
    />
    {#if live.length}<LiveRuns
        runs={shownLive}
        demo={panel.demo}
        {repoName}
        canCancel={panel.canCancel}
        isCancelling={panel.actions.isCancelling}
        onCancel={(run) => void panel.actions.cancelRun(run)}
        onShow={(id) => void view.showRun(id)}
      />{:else}<p class="section-empty">
        {searching
          ? m().shell.overview.noLiveMatches
          : m().shell.overview.nothingRunning}
      </p>{/if}
    {@render more('runs', live.length - shownLive.length)}
  </section>

  <section class="overview-section" aria-labelledby="section-favorites">
    <SectionHeader
      page="favorites"
      id="section-favorites"
      title={m().shell.pages.favorites}
      count={pins.length}
      icon="heart"
      onMaximize={() => void view.open('favorites')}
    />
    {#if pins.length}<FavoriteTiles
        pins={shownPins}
        isUncertain={panel.starts.isQuarantined}
        {repoName}
        {accountName}
        gate={panel.starts.favoriteGate}
        onConfigure={(pin) => modals.openDispatch(pin)}
        onEdit={modals.editPin}
        onRemove={panel.favorites.remove}
        onLaunch={modals.launchPin}
        lastRun={pinRun}
        demo={panel.demo}
      />{:else}<p class="section-empty">
        {searching
          ? m().shell.overview.noFavoriteMatches
          : m().shell.overview.noFavorites}
      </p>{/if}
    {@render more('favorites', pins.length - shownPins.length)}
  </section>

  <section class="overview-section" aria-labelledby="section-repos">
    <SectionHeader
      page="repos"
      id="section-repos"
      title={m().shell.pages.repos}
      count={groups.length}
      onMaximize={() => void view.open('repos')}
    />
    {#if groups.length}<RepositoryList
        groups={shownGroups}
        onopen={(workflow) => modals.openDispatch(undefined, workflow)}
        onfavorite={(workflow) => void modals.toggleFavorite(workflow)}
        favorite={panel.favorites.find}
        gate={panel.starts.gate}
        uncertainRefs={panel.starts.uncertainRefs}
        expanded={panel.expandedRepos}
        onToggle={panel.toggleRepo}
        onOpen={(repoId) => void panel.actions.openRepository(repoId)}
        opening={panel.actions.opening}
        native={panel.native}
        demo={panel.demo}
        onNotice={(text, tone) => panel.showNotice(text, tone)}
        onHide={(repo, hidden) => void panel.actions.setHidden(repo, hidden)}
        onShowRuns={(repo) => void view.showRepoRuns(fullRepo(repo))}
      />{:else}<p class="section-empty">
        {searching
          ? m().shell.overview.noRepoMatches
          : m().shell.overview.noRepos}
      </p>{/if}
    {@render more('repos', groups.length - shownGroups.length)}
  </section>
{/if}
