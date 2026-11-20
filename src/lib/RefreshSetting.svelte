<script lang="ts">
  import { onMount } from 'svelte';
  import { m } from './i18n/index.svelte';
  import {
    formatRefresh,
    presetIndex,
    REFRESH_DEFAULT,
    REFRESH_MAX,
    REFRESH_MIN,
    REFRESH_PRESETS,
    refreshError,
  } from './refresh';
  import type { RefreshChoice } from './refresh.svelte';

  // Slider and own value stage a choice; the dialog's „Speichern“ writes it.
  let { choice }: { choice: RefreshChoice } = $props();
  const LAST = REFRESH_PRESETS.length - 1;
  const RECOMMENDED = presetIndex(REFRESH_DEFAULT);
  const valid = $derived(!choice.invalid);
  /** An invalid own value leaves the slider where the setting stands. */
  const shown = $derived(
    valid ? choice.value : (choice.saved ?? REFRESH_DEFAULT),
  );
  onMount(() => void choice.read());
</script>

<section class="refresh-setting" aria-labelledby="refresh-heading">
  <div class="refresh-head">
    <h3 id="refresh-heading">{m().settings.refresh.heading}</h3>
    <output for="refresh-slider refresh-custom">{formatRefresh(shown)}</output>
  </div>
  <div class="refresh-track">
    <input
      id="refresh-slider"
      type="range"
      min="0"
      max={LAST}
      step="1"
      value={presetIndex(shown)}
      aria-labelledby="refresh-heading"
      aria-valuetext={formatRefresh(shown)}
      title={m().settings.refresh.recommendedHint}
      oninput={(event) =>
        choice.stage(
          REFRESH_PRESETS[event.currentTarget.valueAsNumber] ?? REFRESH_DEFAULT,
        )}
    />
    <span
      class="refresh-tick"
      style:--at={RECOMMENDED / LAST}
      title={m().settings.refresh.recommended}
      aria-hidden="true"
    ></span>
  </div>
  <label class="refresh-custom">
    <span>{m().settings.refresh.custom}</span>
    <input
      id="refresh-custom"
      type="number"
      min={REFRESH_MIN}
      max={REFRESH_MAX}
      step="1"
      inputmode="numeric"
      value={Number.isNaN(choice.value) ? '' : choice.value}
      aria-invalid={valid ? undefined : 'true'}
      aria-describedby={valid ? undefined : 'refresh-error'}
      oninput={(event) => choice.stage(event.currentTarget.valueAsNumber)}
    />
    <span aria-hidden="true">s</span>
  </label>
  <!-- Announced as it appears: focus stays in the field being typed in. -->
  {#if !valid}<p id="refresh-error" class="refresh-error" role="alert">
      {refreshError()}
    </p>{/if}
</section>
