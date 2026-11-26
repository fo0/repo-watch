<script lang="ts">
  import Icon from './Icon.svelte';
  import { m } from './i18n/index.svelte';
  import type { OwnerChoice } from './ownerChoice.svelte';
  import { ownerRows } from './ownerFilter';
  import type { Snapshot } from './types';

  // One row per account with more than one owner; a pressed owner is
  // watched. Toggling only stages; the dialog's „Speichern“ sets the filter.
  let { snapshot, choice }: { snapshot: Snapshot | null; choice: OwnerChoice } =
    $props();
  const rows = $derived(ownerRows(snapshot));
</script>

{#if rows.length}<section class="owner-setting" aria-labelledby="owner-heading">
    <h3 id="owner-heading">{m().settings.owners.heading}</h3>
    {#each rows as row (row.accountId)}
      <div
        class="owner-row"
        role="group"
        aria-label={m().settings.owners.account(row.login)}
      >
        <!-- The account only tells rows apart when there are several. -->
        {#if rows.length > 1}<span class="owner-account" title={row.login}
            >{row.login}</span
          >{/if}
        {#each row.owners as owner (owner)}
          {@const watched = choice.has(row, owner)}
          {@const locked = choice.locked(row, owner)}
          <button
            type="button"
            class="secondary-button compact"
            aria-pressed={watched}
            aria-disabled={locked ? 'true' : undefined}
            title={locked
              ? m().settings.owners.last
              : watched
                ? m().settings.owners.watched(owner)
                : m().settings.owners.ignored(owner)}
            onclick={() => choice.toggle(row, owner)}
            >{#if watched}<Icon name="check" size={12} />{/if}{owner}</button
          >
        {/each}
      </div>
    {/each}
  </section>{/if}
