<script lang="ts">
  import { m } from './i18n/index.svelte';

  /** Quiet time after the last change before the count is announced. */
  const DELAY = 700;

  let {
    query,
    count,
  }: {
    /**
     * What the user searches for and where: query, page and filter. A new
     * value starts the debounce; '' means no search and clears the region.
     */
    query: string;
    /** How many results the page lists for that search right now. */
    count: number;
  } = $props();

  let text = $state('');

  // Only a change of the search announces: run polling changes `count`
  // every few seconds and must stay silent (the count is read untracked,
  // when the timer fires).
  $effect(() => {
    const searched = query;
    text = '';
    if (!searched) return;
    const timer = setTimeout(() => {
      text = m().shell.searchResults(count);
    }, DELAY);
    return () => clearTimeout(timer);
  });
</script>

<!-- Polite and visually hidden (WCAG 4.1.3): the lists already show the
     result; this only tells a screen reader how many there are. -->
<p class="sr-only search-results" role="status">{text}</p>
