<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { startupNote, type Autostart } from './autostart.svelte';
  import { m } from './i18n/index.svelte';

  // The switch stages a choice; the dialog's „Speichern“ writes it.
  let { native, autostart }: { native: boolean; autostart: Autostart } =
    $props();
  const note = $derived(startupNote(autostart.current, autostart.busy));
  const status = $derived(autostart.current?.status);
  /** Windows itself blocks or hides RepoWatch's entry. */
  const vetoed = $derived(
    status === 'disabledByWindows' || status === 'unknownApproval',
  );
  onMount(() => void autostart.read());
</script>

<svelte:window onfocus={() => void autostart.read()} />

<section
  class="startup-panel"
  aria-labelledby="startup-heading"
  aria-busy={autostart.busy}
>
  <h3 id="startup-heading">{m().settings.startup.heading}</h3>
  {#if !native}
    <p class="field-help">{m().settings.startup.demo}</p>
  {:else if status === 'unsupported'}
    <p class="field-help">{note}</p>
  {:else}
    <label class="startup-option">
      <input
        type="checkbox"
        checked={autostart.checked}
        disabled={autostart.busy || !autostart.editable}
        aria-describedby={note ? 'startup-status' : undefined}
        onchange={(event) => {
          autostart.stage(event.currentTarget.checked);
          event.currentTarget.checked = autostart.checked;
        }}
      />
      <span>
        <strong>{m().settings.startup.withWindows}</strong>
        <span>{m().settings.startup.withWindowsHint}</span>
      </span>
    </label>
    {#if note}<p id="startup-status" class="startup-status" role="status">
        {note}
      </p>{/if}
    {#if autostart.loaded && !autostart.editable}
      <div class="startup-actions">
        <!-- The read and the removal turn their own button off via
             aria-disabled, so a keyboard press keeps focus there. -->
        <button
          class="secondary-button"
          aria-disabled={autostart.busy ? 'true' : undefined}
          onclick={autostart.read}
          ><Icon name="refresh" size={14} />{m().settings.startup
            .checkStatus}</button
        >
        {#if vetoed}
          <button
            class="secondary-button"
            disabled={autostart.busy}
            onclick={autostart.openWindowsSettings}
            >{m().settings.startup.openWindows}</button
          >
          {#if autostart.current?.registered}<button
              class="text-button"
              aria-disabled={autostart.busy ? 'true' : undefined}
              onclick={() => !autostart.busy && autostart.remove()}
              >{m().settings.startup.removeEntry}</button
            >{/if}
        {/if}
      </div>
    {/if}
    {#if autostart.error}<div class="alert danger" role="alert">
        <Icon name="warning" size={16} /><span>{autostart.error}</span>
      </div>{/if}
  {/if}
</section>
