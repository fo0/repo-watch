<script module lang="ts">
  import type { Page } from './view.svelte';
</script>

<script lang="ts">
  import Icon from './Icon.svelte';
  import { m } from './i18n/index.svelte';
  import { RESULT, resultStep } from './results';
  import { shortcutHint } from './shortcuts';

  let {
    page,
    query = $bindable(),
  }: {
    /** The shown page; the search filters what it lists. */
    page: Page;
    query: string;
  } = $props();

  let input = $state<HTMLInputElement>();
  /** What the field searches on the page: its name and its placeholder. */
  const labels = $derived(m().repos.search[page]);

  /**
   * Target of the global `/` and Ctrl+F shortcuts; Ctrl+F also selects the
   * query, so typing replaces it.
   */
  export function focus(select = false) {
    input?.focus();
    if (select) input?.select();
  }

  /** A plain arrow key: no modifier, no IME composition. */
  const plain = (event: KeyboardEvent) =>
    !event.isComposing &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.shiftKey;

  /**
   * ArrowDown in the field moves focus to the first result, so a search
   * continues on the keyboard; without a result the caret stays.
   */
  function onkeydown(event: KeyboardEvent) {
    if (event.key !== 'ArrowDown' || !plain(event)) return;
    const first = document.querySelector<HTMLElement>(RESULT);
    if (!first) return;
    event.preventDefault();
    first.focus();
  }

  /**
   * ArrowDown and ArrowUp on a result move focus to the next or previous
   * one, in page order; ArrowUp on the first returns to the field. On the
   * last result the key keeps its default and scrolls.
   */
  function onResultKey(event: KeyboardEvent) {
    if (event.defaultPrevented || !plain(event)) return;
    const target = event.target;
    if (!(target instanceof HTMLElement) || !target.matches(RESULT)) return;
    const results = [...document.querySelectorAll<HTMLElement>(RESULT)];
    const step = resultStep(event.key, results.indexOf(target), results.length);
    if (step === null) return;
    event.preventDefault();
    if (step === 'search') input?.focus();
    else results[step]?.focus();
  }

  /** Escape in the field clears the query first, then leaves the field. */
  export function escape(target: EventTarget | null) {
    if (!input || target !== input) return false;
    if (query) query = '';
    else input.blur();
    return true;
  }
</script>

<svelte:window onkeydown={onResultKey} />

<!-- On the overview the field is where focus lands when nothing better is
     left; a maximized section lands on its title instead. -->
<label class="search-field"
  ><Icon name="search" size={17} /><input
    bind:this={input}
    aria-label={labels.label}
    aria-keyshortcuts="/ Control+F"
    placeholder={labels.placeholder}
    data-focus-landing={page === 'overview' ? true : undefined}
    bind:value={query}
    {onkeydown}
  />{#if query}<button
      class="icon-button small"
      aria-label={m().common.clearSearch}
      onclick={() => {
        query = '';
        // The button goes with the query; focus stays in the field.
        input?.focus();
      }}><Icon name="close" size={14} /></button
    >{:else}<kbd title={shortcutHint()}>/</kbd>{/if}</label
>
