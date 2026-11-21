<script lang="ts">
  import Icon from './Icon.svelte';
  import type { HiddenChoice } from './hiddenChoice.svelte';
  import { hiddenKey } from './hiddenRepos';
  import { m } from './i18n/index.svelte';
  import type { Snapshot } from './types';

  // „Einblenden“ only stages a row; the dialog's „Speichern“ shows it again.
  let {
    snapshot,
    choice,
  }: { snapshot: Snapshot | null; choice: HiddenChoice } = $props();
  const entries = $derived(snapshot?.hiddenRepos ?? []);
  const login = (accountId: string) =>
    snapshot?.accounts.find((account) => account.id === accountId)?.login ??
    accountId;
</script>

<section class="hidden-setting" aria-labelledby="hidden-heading">
  <h3 id="hidden-heading">{m().settings.hidden.heading}</h3>
  {#if entries.length}<ul class="hidden-list">
      {#each entries as entry (hiddenKey(entry.accountId, entry.repoId))}
        {@const staged = choice.has(entry)}
        <!-- `owner/` dimmed as in the repository rows, so a fork and its
             upstream stay apart; the owner gives way first. -->
        <li class="hidden-row" class:staged>
          <span
            class="hidden-name"
            title={`${entry.owner}/${entry.name} · ${login(entry.accountId)}`}
            ><span class="repository-owner">{entry.owner}</span><span
              class="repository-slash">/</span
            ><span class="repository-short">{entry.name}</span></span
          ><button
            type="button"
            class="secondary-button compact"
            aria-pressed={staged}
            aria-label={m().settings.hidden.showRepo(
              `${entry.owner}/${entry.name}`,
            )}
            title={`${entry.owner}/${entry.name}`}
            onclick={() => choice.toggle(entry)}
            >{#if staged}<Icon name="check" size={12} />{/if}{m().settings
              .hidden.show}</button
          >
        </li>
      {/each}
    </ul>{:else}<p class="hidden-empty">{m().settings.hidden.none}</p>{/if}
</section>
