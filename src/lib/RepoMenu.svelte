<script lang="ts">
  import Icon from './Icon.svelte';
  import { copyLink, openRunLink } from './RunLink.svelte';
  import type { NoticeTone } from './StatusBanners.svelte';
  import { m } from './i18n/index.svelte';
  import { menu, openingFocus, type MenuFocus } from './menu';
  import { fullRepo } from './model';
  import { safeGitHubUrl, safeRepoUrl } from './safeGitHubUrl';
  import type { Repo, Run } from './types';

  let {
    repo,
    latest,
    hidden,
    native,
    demo,
    onNotice,
    onHide,
    onShowRuns,
  }: {
    repo: Repo;
    /** The repository's latest run, if any. */
    latest: Run | undefined;
    /** Hidden from the lists; only a search shows it. */
    hidden: boolean;
    native: boolean;
    demo: boolean;
    /** Panel banner for link results. */
    onNotice: (text: string, tone: NoticeTone) => void;
    /** Hides the repository (true) or shows it again (false), at once. */
    onHide: (hidden: boolean) => void;
    /** Opens the Verlauf with this repository's runs. */
    onShowRuns: () => void;
  } = $props();

  const id = $props.id();
  let open = $state(false);
  let initial = $state<MenuFocus>('first');
  /** Where the menu opened: under the button, or at the pointer. */
  let anchor = $state<{ x: number; y: number } | null>(null);
  let trigger = $state<HTMLButtonElement>();

  const name = $derived(fullRepo(repo));
  const runUrl = $derived(latest ? safeGitHubUrl(latest.htmlUrl) : null);
  // Demo repositories are fictional: no link to a real page.
  const repoUrl = $derived(demo ? null : safeRepoUrl(repo.owner, repo.name));
  const linkError = (message: string) => onNotice(message, 'warning');
  function copy() {
    if (repoUrl) void copyLink(repoUrl, onNotice, m().repos.menu.copyFailed);
  }

  function show(focus: MenuFocus, at: { x: number; y: number } | null = null) {
    initial = focus;
    anchor = at;
    open = true;
  }
  /** Closes first, so focus is on the button when the action runs. */
  function run(action: () => void) {
    trigger?.focus();
    open = false;
    action();
  }
  function onTriggerKey(event: KeyboardEvent) {
    const focus = openingFocus(event.key);
    if (!focus) return;
    event.preventDefault();
    show(focus);
  }

  /**
   * Fixed to the viewport inside the panel, so the list's clipping and the
   * scrolling body never cut it off; opens upwards when there is no room.
   */
  function position(node: HTMLElement) {
    const panel = node.closest('.tray')?.getBoundingClientRect();
    const button = trigger?.getBoundingClientRect();
    if (!panel || !button) return;
    const { offsetWidth: width, offsetHeight: height } = node;
    const x = anchor?.x ?? button.right - width;
    const below = anchor?.y ?? button.bottom + 4;
    const above = (anchor?.y ?? button.top - 4) - height;
    const left = Math.max(panel.left + 8, Math.min(x, panel.right - 8 - width));
    const top =
      below + height <= panel.bottom - 8
        ? below
        : Math.max(panel.top + 8, above);
    node.style.left = `${left}px`;
    node.style.top = `${top}px`;
  }

  /**
   * Places the open menu and keeps it at its button while the body scrolls;
   * once the button scrolled out of sight the menu closes.
   */
  function place(node: HTMLElement) {
    position(node);
    const body = trigger?.closest('.content');
    const follow = () => {
      const view = body?.getBoundingClientRect();
      const button = trigger?.getBoundingClientRect();
      if (
        !view ||
        !button ||
        button.bottom < view.top ||
        button.top > view.bottom
      ) {
        // Focus in the vanishing menu stays at its button, without
        // scrolling the body back to it.
        if (node.contains(document.activeElement))
          trigger?.focus({ preventScroll: true });
        return close();
      }
      anchor = null;
      position(node);
    };
    body?.addEventListener('scroll', follow);
    return { destroy: () => body?.removeEventListener('scroll', follow) };
  }
  /** Idempotent: a menu removed while focused reports its focusout late. */
  function close() {
    if (open) open = false;
  }

  // A right click on the repository's row opens the same menu at the pointer.
  $effect(() => {
    const row = trigger?.closest<HTMLElement>('[data-repo-id]');
    if (!row) return;
    const onContext = (event: MouseEvent) => {
      event.preventDefault();
      // Inside the open menu a right click changes nothing.
      if (
        event.target instanceof Element &&
        event.target.closest('[role="menu"]')
      )
        return;
      trigger?.focus();
      // The context-menu key reports no pointer: open under the button.
      show(
        'first',
        event.button === 2 ? { x: event.clientX, y: event.clientY } : null,
      );
    };
    row.addEventListener('contextmenu', onContext);
    return () => row.removeEventListener('contextmenu', onContext);
  });
</script>

<div class="repo-menu">
  <button
    bind:this={trigger}
    type="button"
    class="icon-button repo-menu-button"
    class:open
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={open ? `${id}-menu` : undefined}
    aria-label={m().repos.menu.actionsFor(name)}
    title={m().repos.menu.actionsTitle}
    data-focus-key={`repo-menu:${repo.id}`}
    onclick={() => (open ? (open = false) : show('first'))}
    onkeydown={onTriggerKey}><Icon name="more" size={16} /></button
  >
  {#if open}
    <!-- tabindex="-1": a press on the padding focuses the menu
         itself, so focusout does not close it. -->
    <div
      id={`${id}-menu`}
      class="menu repo-actions-menu"
      role="menu"
      tabindex="-1"
      aria-label={m().repos.menu.actionsFor(name)}
      use:place
      use:menu={{ trigger, initial, onClose: close }}
    >
      {#if latest}{#if runUrl}<a
            class="menu-item"
            role="menuitem"
            tabindex="-1"
            href={runUrl}
            target="_blank"
            rel="noopener noreferrer"
            onclick={(event) =>
              run(() => openRunLink(event, runUrl, native, linkError))}
            onauxclick={(event) => {
              if (event.button === 1)
                run(() => openRunLink(event, runUrl, native, linkError));
            }}><Icon name="external" size={16} />{m().repos.menu.openLastRun}</a
          >{:else}<button
            type="button"
            class="menu-item"
            role="menuitem"
            tabindex="-1"
            disabled
            title={demo
              ? m().repos.menu.demoRunNoLink
              : m().repos.menu.runNoLink}
            ><Icon name="external" size={16} />{m().repos.menu
              .openLastRun}</button
          >{/if}{/if}
      <!-- Only a shown repository has runs in the Verlauf (ADR-0008). -->
      {#if latest && repo.selected && !hidden}<button
          type="button"
          class="menu-item"
          role="menuitem"
          tabindex="-1"
          onclick={() => run(onShowRuns)}
          ><Icon name="activity" size={16} />{m().repos.menu.showRuns}</button
        >{/if}
      <button
        type="button"
        class="menu-item"
        role="menuitem"
        tabindex="-1"
        disabled={!repoUrl}
        title={repoUrl
          ? repoUrl
          : demo
            ? m().repos.demoRepoNoLink
            : m().repos.repoNoLink}
        onclick={() => run(copy)}
        ><Icon name="copy" size={16} />{m().repos.menu.copyLink}</button
      >
      <button
        type="button"
        class="menu-item"
        role="menuitem"
        tabindex="-1"
        title={hidden ? m().repos.menu.showTitle : m().repos.menu.hideTitle}
        onclick={() => run(() => onHide(!hidden))}
        ><Icon name={hidden ? 'eye' : 'eyeOff'} size={16} />{hidden
          ? m().repos.menu.show
          : m().repos.menu.hide}</button
      >
    </div>
  {/if}
</div>
