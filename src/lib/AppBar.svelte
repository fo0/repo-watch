<script lang="ts">
  import AccountMenu from './AccountMenu.svelte';
  import Icon from './Icon.svelte';
  import { m } from './i18n/index.svelte';
  import type { Account, Snapshot } from './types';

  let {
    native,
    demo,
    accounts,
    accountId,
    accountName,
    connectivity,
    message,
    refreshing,
    ready,
    canRefresh,
    busy,
    onAccountChange,
    onHome,
    onConnect,
    onManageAccounts,
    onRefresh,
    onSettings,
  }: {
    native: boolean;
    demo: boolean;
    accounts: Account[];
    accountId: string;
    accountName: (id: string) => string;
    /** Undefined while the first snapshot loads. */
    connectivity: Snapshot['connectivity'] | undefined;
    /** The snapshot's reason for a stale or refused state. */
    message: string | undefined;
    refreshing: boolean;
    /** A snapshot is loaded; settings and the account menu wait for it. */
    ready: boolean;
    /** A snapshot is loaded or a load error invites a retry. */
    canRefresh: boolean;
    /** A settings change is running; the account stays as it is meanwhile. */
    busy: boolean;
    onAccountChange: (id: string) => void;
    /** The brand: back to the overview, without a search. */
    onHome: () => void;
    onConnect: () => void;
    onManageAccounts: () => void;
    onRefresh: () => void;
    onSettings: () => void;
  } = $props();

  let accountMenu = $state<ReturnType<typeof AccountMenu>>();

  /** Moves focus to the account menu button. */
  export function focusAccount() {
    accountMenu?.focus();
  }
  /** Opens the account menu, e.g. from the empty state's „Konto wählen“. */
  export function openAccountMenu() {
    accountMenu?.openMenu();
  }
</script>

<!-- Natively the bar drags the window from its free space; its controls,
     the brand's home button among them, stay clickable. The heading keeps
     its name „RepoWatch“; the tooltip says where the button leads.
     A running refresh turns its button off via aria-disabled, not disabled,
     so a keyboard press keeps focus there instead of dropping it to <body>. -->
<header class="app-bar" data-tauri-drag-region={native ? true : undefined}>
  <h1 class="brand">
    <button
      type="button"
      class="brand-home"
      title={m().shell.appBar.home}
      onclick={() => onHome()}
      ><span class="brand-icon"><Icon name="tray" size={16} /></span><span
        class="brand-name">RepoWatch</span
      ></button
    >
  </h1>
  <div class="app-bar-actions">
    <AccountMenu
      bind:this={accountMenu}
      {native}
      {accounts}
      {accountId}
      {accountName}
      {connectivity}
      {message}
      disabled={!ready || busy}
      onChange={onAccountChange}
      {onConnect}
      onManage={onManageAccounts}
    /><button
      class:spinning={refreshing}
      class="icon-button"
      aria-label={m().shell.appBar.refresh}
      aria-keyshortcuts="F5 Control+R"
      title={m().shell.appBar.refreshTitle(demo)}
      onclick={() => !refreshing && onRefresh()}
      disabled={!canRefresh}
      aria-disabled={refreshing ? 'true' : undefined}
      ><Icon name="refresh" size={17} /></button
    ><button
      class="icon-button"
      aria-label={m().shell.appBar.settings}
      aria-keyshortcuts="Control+,"
      title={m().shell.appBar.settings}
      onclick={() => onSettings()}
      disabled={!ready}><Icon name="settings" size={17} /></button
    >
  </div>
</header>
