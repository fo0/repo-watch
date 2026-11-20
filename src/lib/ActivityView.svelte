<script module lang="ts">
  import type { ActivityFilter, RunTone } from './runs';

  /** The filter chips; each label comes from the catalog by its id. */
  const FILTERS: { id: ActivityFilter; tone?: RunTone }[] = [
    { id: 'all' },
    { id: 'active', tone: 'run' },
    { id: 'failed', tone: 'fail' },
    { id: 'recent' },
  ];
</script>

<script lang="ts">
  import EmptyState, { type NextStep } from './EmptyState.svelte';
  import Icon from './Icon.svelte';
  import RunRow from './RunRow.svelte';
  import { minuteClock } from './clock';
  import { focusLost, hasDom } from './focus';
  import { m } from './i18n/index.svelte';
  import { HISTORY_LIMIT, isActive } from './runs';
  import type { NoticeTone } from './StatusBanners.svelte';
  import type { Run } from './types';

  let {
    runs,
    filter,
    expanded,
    activeRuns,
    failedRuns,
    demo,
    native,
    setup,
    searching,
    repoName,
    repoAccountName,
    canCancel = () => false,
    isCancelling = () => false,
    onCancel = () => {},
    canStartAgain = () => false,
    onStartAgain = () => {},
    onFilter,
    onToggle,
    onClearQuery,
    onNotice,
  }: {
    runs: Run[];
    filter: ActivityFilter;
    /** ID of the run whose details are open. */
    expanded: number | null;
    activeRuns: number;
    failedRuns: number;
    demo: boolean;
    native: boolean;
    /** The missing account or repository step, named like on every page. */
    setup: NextStep | null;
    searching: boolean;
    repoName: (repoId: string) => string;
    repoAccountName: (repoId: string) => string;
    /** The run is open and its repository grants write access. */
    canCancel?: (run: Run) => boolean;
    isCancelling?: (run: Run) => boolean;
    onCancel?: (run: Run) => void;
    /** Its workflow can be started again: dispatchable, with write access. */
    canStartAgain?: (run: Run) => boolean;
    /** Opens the start dialog with the run's workflow and branch. */
    onStartAgain?: (run: Run) => void;
    onFilter: (filter: ActivityFilter) => void;
    onToggle: (runId: number) => void;
    onClearQuery: () => void;
    onNotice: (text: string, tone: NoticeTone) => void;
  } = $props();

  /** The refresh and history-limit note is on demand, not always visible. */
  let showNote = $state(false);
  let chips = $state<HTMLElement>();
  /**
   * „Alle anzeigen“ under a filter that lists nothing: back to „Alle“, which
   * takes focus, since the empty state's button goes with the runs it shows.
   */
  function showAll() {
    onFilter('all');
    chips?.querySelector<HTMLElement>('button')?.focus();
  }
  /** Aktiv and Fehlgeschlagen name their size; the other chips do not. */
  const count = (id: ActivityFilter) =>
    id === 'active' ? activeRuns : id === 'failed' ? failedRuns : undefined;
  const text = $derived(m().runs.activity);
  /** Open runs under „Jetzt“, the completed history under „Abgeschlossen“. */
  const groups = $derived(
    [
      { id: 'now', title: text.now, runs: runs.filter(isActive) },
      {
        id: 'done',
        title: text.done,
        runs: runs.filter((run) => !isActive(run)),
      },
    ].filter((group) => group.runs.length > 0),
  );

  /*
   * A run that moves between the groups, e.g. when it completes, is drawn
   * anew in the other one, and so is the hint when its group goes: focus
   * moves to the new run toggle or hint instead of dropping to the page.
   */
  let focusedKey: string | undefined;
  $effect.pre(() => {
    void groups;
    const active = hasDom() ? document.activeElement : null;
    focusedKey = active?.matches('.info-button')
      ? 'info'
      : active?.closest<HTMLElement>('.run')?.dataset.run;
  });
  $effect(() => {
    void groups;
    if (!focusedKey || !focusLost()) return;
    const target =
      focusedKey === 'info'
        ? document.querySelector<HTMLElement>('.info-button')
        : [...document.querySelectorAll<HTMLElement>('.run')]
            .find((node) => node.dataset.run === focusedKey)
            ?.querySelector<HTMLElement>('.run-button');
    target?.focus();
  });
</script>

<div class="filters" role="group" aria-label={text.filters} bind:this={chips}>
  {#each FILTERS as item (item.id)}
    {@const runCount = count(item.id)}
    {@const label = text[item.id]}
    <button
      class:chosen={filter === item.id}
      aria-pressed={filter === item.id}
      aria-label={runCount === undefined
        ? undefined
        : `${label}, ${m().common.count.runs(runCount)}`}
      onclick={() => onFilter(item.id)}
      >{#if item.tone}<span class="tone-dot" data-tone={item.tone}
        ></span>{/if}{label}{#if runCount !== undefined}<b>{runCount}</b
        >{/if}</button
    >
  {/each}
</div>
{#each groups as group, index (group.id)}
  <div class="section-header">
    <h3 id={`runs-${group.id}`}>{group.title}</h3>
    <span class="section-count"
      ><span aria-hidden="true">{group.runs.length}</span><span class="sr-only"
        >{m().common.count.runs(group.runs.length)}</span
      ></span
    >
    {#if index === 0}<span class="section-actions"
        ><button
          class="icon-button info-button"
          aria-expanded={showNote}
          aria-controls="history-note"
          aria-label={text.note}
          title={text.note}
          onclick={() => (showNote = !showNote)}
          ><Icon name="info" size={15} /></button
        ></span
      >{/if}
  </div>
  {#if index === 0}<p class="history-note" id="history-note" hidden={!showNote}>
      <span class="tone-dot" data-tone="run"></span>{text.noteLive}<br
      />{text.noteLimit(HISTORY_LIMIT)}
    </p>{/if}
  <div class="run-list" role="group" aria-labelledby={`runs-${group.id}`}>
    {#each group.runs as run (`${run.repoId}:${run.id}`)}
      <RunRow
        {run}
        expanded={expanded === run.id}
        repo={repoName(run.repoId)}
        account={repoAccountName(run.repoId)}
        {demo}
        {native}
        now={$minuteClock}
        canCancel={canCancel(run)}
        cancelling={isCancelling(run)}
        onCancel={() => onCancel(run)}
        canStartAgain={canStartAgain(run)}
        onStartAgain={() => onStartAgain(run)}
        onToggle={() => onToggle(run.id)}
        {onNotice}
      />
    {/each}
  </div>
{:else}{#if setup}<EmptyState icon="activity" iconSize={25} {...setup} />
  {:else}<EmptyState
      icon="activity"
      iconSize={25}
      title={searching ? text.noMatches : text.quiet}
      text={searching ? m().common.tryOtherSearch : text.noneForFilter}
      action={searching
        ? { label: m().common.clearSearch, icon: 'close', run: onClearQuery }
        : filter !== 'all'
          ? { label: text.showAll, icon: 'close', run: showAll }
          : undefined}
    />{/if}{/each}
