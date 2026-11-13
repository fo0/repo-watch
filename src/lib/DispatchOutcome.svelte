<script lang="ts">
  import Icon from './Icon.svelte';
  import ReleaseLock from './ReleaseLock.svelte';
  import RunLink from './RunLink.svelte';
  import { m } from './i18n/index.svelte';
  import { safeGitHubUrl } from './safeGitHubUrl';
  import type { DispatchOutcome, UncertainStart } from './types';

  let {
    outcome,
    error,
    notice,
    noticeTone,
    uncertain,
    demo,
    native,
    checking,
    releasable,
    claim,
    releasing,
    onCheckStatus,
    onRelease,
    onShowRun,
    onLinkError,
  }: {
    /** This dialog's start result, if it has one. */
    outcome: DispatchOutcome | null;
    error: string;
    notice: string;
    noticeTone: 'subtle' | 'warning';
    /** The target is quarantined by an earlier start with unknown outcome. */
    uncertain: boolean;
    demo: boolean;
    native: boolean;
    /** A status refresh is running. */
    checking: boolean;
    /** A claim (persisted, or this session's untimed lock) exists, so it can be released. */
    releasable: boolean;
    /** The claim the confirmation holds on to when it opens (untimed for a session lock). */
    claim?: UncertainStart | undefined;
    releasing: boolean;
    onCheckStatus: () => void;
    onRelease: (claim?: UncertainStart) => Promise<void>;
    onShowRun: (runId: number) => void;
    onLinkError: (message: string) => void;
  } = $props();
  let box = $state<HTMLElement>();
  let errorBox = $state<HTMLElement>();
  let noticeBox = $state<HTMLElement>();
  const runUrl = $derived(safeGitHubUrl(outcome?.htmlUrl ?? ''));

  /** Brings a fresh result or error into view and moves focus to it. */
  export function focus() {
    const target = box ?? errorBox ?? noticeBox;
    target?.scrollIntoView({ block: 'nearest' });
    target?.focus({ preventScroll: true });
  }
</script>

{#if error}<div
    class="alert danger"
    role="alert"
    tabindex="-1"
    bind:this={errorBox}
  >
    <Icon name="warning" size={16} /><span>{error}</span>
  </div>{/if}
{#if notice}<div
    class={`alert ${noticeTone}`}
    role="status"
    tabindex="-1"
    bind:this={noticeBox}
  >
    <Icon
      name={noticeTone === 'warning' ? 'warning' : 'check'}
      size={16}
    /><span>{notice}</span>
  </div>{/if}

<!-- A running check turns its button off via aria-disabled, not disabled,
     so a keyboard press keeps focus there instead of dropping it to <body>. -->
{#snippet checkStatus()}<button
    type="button"
    class="text-button"
    onclick={() => !checking && onCheckStatus()}
    aria-disabled={checking ? 'true' : undefined}
    >{m().dispatch.uncertain.checkStatus}<Icon
      name="refresh"
      size={12}
    /></button
  >{#if releasable}<ReleaseLock
      {releasing}
      {claim}
      {onRelease}
    />{/if}{/snippet}

{#if outcome}<div
    class="outcome"
    class:accepted={outcome.state === 'accepted'}
    class:uncertain={outcome.state === 'uncertain'}
    class:rejected={outcome.state === 'rejected'}
    role="status"
    tabindex="-1"
    bind:this={box}
  >
    <Icon name={outcome.state === 'accepted' ? 'check' : 'warning'} size={18} />
    <div>
      <strong
        >{outcome.state === 'accepted'
          ? demo
            ? m().dispatch.outcome.acceptedDemo
            : m().dispatch.outcome.accepted
          : outcome.state === 'uncertain'
            ? m().dispatch.uncertain.title
            : m().dispatch.outcome.rejected}</strong
      >
      <p>{outcome.message}</p>
      {#if outcome.state === 'accepted' && outcome.runId}
        {@const runId = outcome.runId}
        {#if runUrl}<RunLink
            url={runUrl}
            label={m().dispatch.outcome.viewRun(runId)}
            {native}
            onError={onLinkError}
          />{/if}
        <button
          type="button"
          class="text-button"
          onclick={() => onShowRun(runId)}
          >{m().dispatch.outcome.showInHistory}<Icon
            name="arrow"
            size={13}
          /></button
        >
      {/if}
      {#if outcome.state === 'uncertain'}{@render checkStatus()}{/if}
    </div>
  </div>
{:else if uncertain}<div class="outcome uncertain" role="status">
    <Icon name="warning" size={18} />
    <div>
      <strong>{m().dispatch.uncertain.title}</strong>
      <p>{m().dispatch.uncertain.locked}</p>
      {@render checkStatus()}
    </div>
  </div>{/if}
