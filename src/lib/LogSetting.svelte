<script lang="ts">
  import { onMount } from 'svelte';
  import { m } from './i18n/index.svelte';
  import type { LogChoice } from './logChoice.svelte';
  import { LOG_LEVELS } from './logLevel';

  // The level stages a choice; the dialog's „Speichern“ writes it. „Ordner
  // öffnen“ acts at once and exists only where a log file does.
  let {
    choice,
    native,
    onOpenFolder,
  }: { choice: LogChoice; native: boolean; onOpenFolder: () => void } =
    $props();
  onMount(() => void choice.read());
</script>

<section class="log-setting" aria-labelledby="log-heading">
  <h3 id="log-heading">{m().settings.log.heading}</h3>
  <div class="log-row">
    <div class="theme-options" role="radiogroup" aria-labelledby="log-heading">
      {#each LOG_LEVELS as level (level)}
        <label class="theme-option">
          <input
            type="radio"
            name="log-level"
            value={level}
            checked={choice.value === level}
            onchange={() => choice.stage(level)}
          />
          <span>{m().settings.log[level]}</span>
        </label>
      {/each}
    </div>
    {#if native}<button
        type="button"
        class="secondary-button"
        onclick={onOpenFolder}>{m().settings.log.openFolder}</button
      >{/if}
  </div>
</section>
