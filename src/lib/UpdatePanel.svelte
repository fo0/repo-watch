<script lang="ts">
  import Icon from './Icon.svelte';
  import { clockTime } from './format';
  import { m } from './i18n/index.svelte';
  import type { DownloadProgress, UpdateInfo } from './types';
  import { changelogSummary, releaseDate, type UpdatePhase } from './updates';

  let {
    currentVersion,
    native,
    info,
    phase,
    progress,
    error,
    checkedAt,
    onCheck,
    onInstall,
  }: {
    currentVersion: string;
    native: boolean;
    info: UpdateInfo | null;
    phase: UpdatePhase;
    progress: DownloadProgress;
    error: string;
    /** When the last check succeeded (epoch ms), null before. */
    checkedAt: number | null;
    onCheck: () => void;
    /** Downloads, verifies and installs in one go; the app then restarts. */
    onInstall: () => void;
  } = $props();
  const busy = $derived(
    ['checking', 'downloading', 'verifying', 'installing'].includes(phase),
  );
  const percent = $derived(
    progress.total
      ? Math.min(100, Math.floor((progress.downloaded / progress.total) * 100))
      : undefined,
  );
  const published = $derived(info ? releaseDate(info.date) : '');
  const bullets = $derived(info ? changelogSummary(info.notes) : []);
</script>

<section class="update-panel" aria-label={m().updates.label}>
  <div class="update-row">
    <div class="update-version">
      <strong>{m().updates.version(currentVersion)}</strong>
      {#if native && checkedAt !== null}<span
          >{m().updates.lastChecked(clockTime(checkedAt))}</span
        >{/if}
    </div>
    <!-- The icon spins while the check runs, like the app bar's refresh.
         Off via aria-disabled: a pressed check keeps focus here. -->
    {#if native}<button
        class="secondary-button"
        class:spinning={phase === 'checking'}
        aria-disabled={busy ? 'true' : undefined}
        onclick={() => !busy && onCheck()}
        ><Icon name="refresh" size={14} />{m().updates.check}</button
      >{/if}
  </div>
  {#if !native}
    <p class="update-line">{m().updates.demo}</p>
  {:else if phase === 'current'}
    <p class="update-line update-ok" role="status">
      <Icon name="check" size={14} />{m().updates.current}
    </p>
  {:else if info}
    <div class="update-release">
      <p class="update-title">
        <strong>{m().updates.available(info.version)}</strong
        >{#if published}<span>{published}</span>{/if}
      </p>
      {#if bullets.length}<ul
          class="update-notes"
          aria-label={m().updates.notes}
        >
          {#each bullets as bullet, index (index)}<li>{bullet}</li>{/each}
        </ul>{/if}
      {#if phase === 'downloading'}
        <div class="update-progress" role="status">
          <label for="update-download"
            >{m().updates.downloading}{percent === undefined
              ? ''
              : ` ${percent} %`}</label
          >
          {#if percent === undefined}<progress id="update-download"
            ></progress>{:else}<progress
              id="update-download"
              max="100"
              value={percent}
            ></progress>{/if}
        </div>
      {:else if phase === 'verifying'}
        <p class="update-line" role="status">{m().updates.verifying}</p>
      {:else if phase === 'installing'}
        <p class="update-line" role="status">
          {m().updates.installing}
        </p>
      {:else}
        {#if error}<div class="alert danger" role="alert">
            <Icon name="warning" size={14} /><span>{error}</span>
          </div>{/if}
        <button class="primary-button" disabled={busy} onclick={onInstall}
          >{error ? m().common.retry : m().updates.install}</button
        >
      {/if}
    </div>
  {/if}
  {#if error && !info}<div class="alert danger" role="alert">
      <Icon name="warning" size={14} /><span>{error}</span>
    </div>{/if}
</section>
