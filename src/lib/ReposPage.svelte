<script lang="ts">
  import EmptyState, { type NextStep } from './EmptyState.svelte';
  import Icon from './Icon.svelte';
  import RepositoryList from './RepositoryList.svelte';
  import { errorMessage } from './format';
  import { m } from './i18n/index.svelte';
  import type { Modals } from './modals.svelte';
  import type { Panel } from './panel.svelte';
  import type { repositoryActivity } from './repoActivity';
  import type { Repo } from './types';

  let {
    panel,
    modals,
    groups,
    searching,
    setup,
    onClearQuery,
    onShowRuns,
  }: {
    /** Starts, favorites and expanded rows come from the panel. */
    panel: Panel;
    /** Opens the start dialog for a workflow. */
    modals: Modals;
    /**
     * Every selected repository matching the search, latest activity first,
     * then the account's other repositories the search found.
     */
    groups: ReturnType<typeof repositoryActivity>;
    /** A non-blank search query is active. */
    searching: boolean;
    /** The missing account step, shown instead of the list. */
    setup: NextStep | null;
    onClearQuery: () => void;
    /** Opens the Verlauf with a repository's runs. */
    onShowRuns: (repo: Repo) => void;
  } = $props();

  /** A GitHub App sign-in decides on GitHub which repositories it lists. */
  const browserInstallation = $derived(
    !panel.demo &&
      !!panel.snapshot?.accounts.some((account) =>
        account.id.startsWith('github-device-'),
      ),
  );
  /** Errors land in the banner; a stale refresh is announced there already. */
  async function attempt(action: () => Promise<unknown>) {
    try {
      await action();
    } catch (cause) {
      panel.showNotice(errorMessage(cause), 'warning');
    }
  }
</script>

{#if setup}<EmptyState icon="repo" iconSize={25} {...setup} />
{:else if groups.length}<RepositoryList
    {groups}
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
    {onShowRuns}
  />
{:else if searching}<EmptyState
    icon="search"
    iconSize={25}
    title={m().repos.page.noMatches}
    text={m().common.tryOtherSearch}
    action={{
      label: m().common.clearSearch,
      icon: 'close',
      run: onClearQuery,
    }}
  />{:else}<p class="section-empty">{m().repos.page.none}</p>{/if}
{#if !setup && browserInstallation}<div class="repository-access">
    <button
      class="text-button"
      disabled={panel.settingsBusy}
      onclick={() => void attempt(panel.manageInstallation)}
      >{m().repos.page.manageAccess}<Icon name="external" size={14} /></button
    ><button
      class="text-button"
      disabled={panel.settingsBusy}
      onclick={() => void attempt(panel.refreshAccess)}
      >{m().repos.page.refreshAccess}<Icon name="refresh" size={14} /></button
    >
  </div>{/if}
