<script lang="ts">
  import Icon from './Icon.svelte';
  import { m } from './i18n/index.svelte';

  let {
    open,
    label = $bindable(),
    editing,
    saving,
    locked,
    valid,
    onSave,
  }: {
    /** Opened up front to complete or edit a favorite. */
    open: boolean;
    label: string;
    /** The dialog edits an existing template. */
    editing: boolean;
    /** A template save is running. */
    saving: boolean;
    /** A start for this target is running; the form stays as it was sent. */
    locked: boolean;
    /** The current selection would pass dispatch validation. */
    valid: boolean;
    onSave: (asNew: boolean) => void;
  } = $props();
  const canSave = $derived(!saving && !locked && valid && !!label.trim());
  let details = $state<HTMLDetailsElement>();
  // Follows the prop only: as a template attribute, `open` was re-set with
  // the save button's state and closed a section the user had opened.
  $effect(() => {
    if (details) details.open = open;
  });
  /**
   * Why the save buttons are off, as their tooltip; none while saving, the
   * button then reads „Speichert …“.
   */
  const reason = $derived(
    saving
      ? undefined
      : locked
        ? m().dispatch.reasons.running
        : !valid
          ? m().favorites.save.incomplete
          : !label.trim()
            ? m().favorites.save.nameMissing
            : undefined,
  );
</script>

<details class="save-pin" bind:this={details}>
  <summary
    ><Icon name="heart" size={14} />{m().favorites.save.summary}<Icon
      name="down"
      size={13}
    /></summary
  >
  <div class="save-pin-body">
    <label class="field"
      ><span>{m().favorites.save.name}</span><input
        aria-label={m().favorites.save.name}
        bind:value={label}
        onkeydown={(event) => {
          if (event.key !== 'Enter') return;
          // Enter saves the favorite; it never submits a start.
          event.preventDefault();
          event.stopPropagation();
          if (canSave) onSave(false);
        }}
        disabled={locked}
        readonly={saving}
        maxlength="80"
        placeholder={m().favorites.save.placeholder}
      /></label
    >
    <p class="field-help">{m().favorites.save.help}</p>
    <!-- Off via aria-disabled, not disabled: a pressed save keeps focus
         here while it runs, and the tooltip names why it is off. -->
    <button
      type="button"
      class="secondary-button compact"
      onclick={() => canSave && onSave(false)}
      aria-disabled={canSave ? undefined : 'true'}
      title={reason}
      ><Icon name="heart" size={13} />{saving
        ? m().favorites.save.saving
        : editing
          ? m().favorites.save.update
          : m().favorites.save.save}</button
    >
    {#if editing}<button
        type="button"
        class="text-button copy-template"
        onclick={() => canSave && onSave(true)}
        aria-disabled={canSave ? undefined : 'true'}
        title={reason}
        >{m().favorites.save.saveAsNew}<Icon name="plus" size={13} /></button
      >{/if}
  </div>
</details>
