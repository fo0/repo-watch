<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import RunLink, { copyLink } from './RunLink.svelte';
  import { ArmedConfirm } from './armedConfirm.svelte';
  import { clockTime, runDuration, runMoment } from './format';
  import { m } from './i18n/index.svelte';
  import {
    isActive,
    runLabel,
    runTone,
    shortRepo,
    statusLabel,
    TONE_ICON,
  } from './runs';
  import { safeGitHubUrl } from './safeGitHubUrl';
  import type { NoticeTone } from './StatusBanners.svelte';
  import type { Run } from './types';

  let {
    run,
    expanded,
    repo,
    account,
    demo,
    native,
    now,
    canCancel = false,
    cancelling = false,
    onCancel,
    canStartAgain = false,
    onStartAgain,
    onToggle,
    onNotice,
  }: {
    run: Run;
    expanded: boolean;
    /** Full repository name, `owner/name`; the row shows the name only. */
    repo: string;
    /** Account display name; only read while expanded. */
    account: string;
    demo: boolean;
    native: boolean;
    /** Shared minute clock (ms) for relative times and running durations. */
    now: number;
    /** Open, with write access: the details offer „Run abbrechen“. */
    canCancel?: boolean;
    /** A cancel request is pending or accepted; the run is still open. */
    cancelling?: boolean;
    onCancel?: () => void;
    /** Its workflow can be started again: the details offer „Erneut starten …“. */
    canStartAgain?: boolean;
    onStartAgain?: () => void;
    onToggle: () => void;
    /** Panel banner for link results: copied, or why opening/copying failed. */
    onNotice: (text: string, tone: NoticeTone) => void;
  } = $props();

  const linkError = (message: string) => onNotice(message, 'warning');
  const text = $derived(m().runs.row);
  const cancelText = $derived(m().runs.cancel);
  const noLink = $derived(demo ? text.noLinkDemo : text.noLink);

  const copy = () => runUrl && copyLink(runUrl, onNotice, text.copyFailed);

  // One status icon and colour per tone everywhere, as in „Läuft gerade“.
  const tone = $derived(runTone(run));
  const open = $derived(isActive(run));
  /** A stuck run explains itself in a tooltip on its status. */
  const hint = $derived(tone === 'stuck' ? text.stuckHint : undefined);
  const runUrl = $derived(safeGitHubUrl(run.htmlUrl));
  const moment = $derived(runMoment(run, now, demo));
  const elapsed = $derived(runDuration(run, now, !demo));
  /** „Run abbrechen“ asks once more before it cancels. */
  const confirm = new ArmedConfirm<true>();
  onDestroy(confirm.disarm);
</script>

<article class="run" class:expanded data-run={`${run.repoId}:${run.id}`}>
  <button
    class="run-button"
    aria-expanded={expanded}
    aria-label={`${runLabel(run, repo)} · ${statusLabel(run)} · ${moment.spoken}${hint ? ` · ${hint}` : ''}`}
    onclick={() => onToggle()}
  >
    <!-- A running run turns a spinner, or shows the still icon when reduced
         motion is asked for (02-activity.css picks one). -->
    <span class="status-tile" data-tone={tone} title={hint}
      >{#if tone === 'run'}<Icon
          name="spinner"
          size={15}
          class="tile-spinner"
        />{/if}<Icon name={TONE_ICON[tone]} size={15} /></span
    >
    <!-- The repository leads, since every repository may name its workflow
         „Release“; workflow and run number open the meta line. -->
    <span class="run-body"
      ><span class="run-repo" title={repo}>{shortRepo(repo)}</span><span
        class="run-meta"
        ><span class="run-workflow" title={run.name}
          >{run.name}<span class="run-number">#{run.id}</span></span
        ><span class="branch-chip" title={run.branch}
          ><Icon name="branch" size={11} /><span>{run.branch}</span></span
        >{#if !open && tone !== 'stuck'}<span class="run-took"
            ><span class="meta-dot" aria-hidden="true">·</span><span
              class="run-duration">{elapsed}</span
            ></span
          >{/if}</span
      ></span
    >
    <!-- Open runs count up since their start; completed ones say when. -->
    <span class="run-side"
      ><span class="run-status tone-dot" data-tone={tone} title={hint}
        >{statusLabel(run)}</span
      ><time class="run-time" datetime={moment.datetime} title={moment.title}
        >{open ? text.since(elapsed) : moment.relative}</time
      ></span
    >
  </button>
  {#if expanded}<div class="run-details">
      <div class="run-fact">
        <span>{text.account}</span><strong>{account}</strong>
      </div>
      <div class="run-fact">
        <span>{text.updated}</span><strong
          >{clockTime(run.updatedAt)}{demo ? m().common.demoTime : ''}</strong
        >
      </div>
      <div class="run-fact">
        <span>{text.status}</span><strong title={hint}
          >{statusLabel(run)}</strong
        >
      </div>
      <!-- The actions of a run appear with its details, never on the row. -->
      <div class="run-links">
        {#if runUrl}<RunLink
            url={runUrl}
            label={text.open}
            {native}
            onError={linkError}
            button
          />{:else}<button
            class="secondary-button compact"
            disabled
            title={noLink}><Icon name="external" size={13} />{text.open}</button
          >{/if}<button
          class="secondary-button compact"
          disabled={!runUrl}
          title={runUrl ? undefined : noLink}
          onclick={() => void copy()}
          ><Icon name="copy" size={13} />{text.copy}</button
        >{#if canStartAgain}<button
            class="secondary-button compact run-again"
            title={text.startAgainTitle}
            onclick={() => onStartAgain?.()}
            ><Icon name="play" size={13} />{text.startAgain}</button
          >{/if}{#if canCancel}<button
            class="secondary-button compact run-cancel"
            class:armed={confirm.is(true)}
            aria-disabled={cancelling ? 'true' : undefined}
            title={confirm.is(true) ? cancelText.confirmTitle : undefined}
            onclick={() =>
              !cancelling && confirm.press(true, () => onCancel?.())}
            onblur={confirm.disarm}
            ><Icon name="stop" size={13} />{cancelling
              ? cancelText.pendingTitle
              : confirm.is(true)
                ? cancelText.armedLong
                : cancelText.run}</button
          >{/if}
        {#if !runUrl}<span class="run-link-note">{noLink}.</span>{/if}
      </div>
    </div>{/if}
</article>
