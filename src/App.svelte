<script lang="ts">
  import { onMount, tick } from 'svelte';
  import AppBar from './lib/AppBar.svelte';
  import AppFooter from './lib/AppFooter.svelte';
  import ConnectPanel from './lib/ConnectPanel.svelte';
  import DispatchDialog from './lib/DispatchDialog.svelte';
  import Modal from './lib/Modal.svelte';
  import NativeConnect from './lib/NativeConnect.svelte';
  import PanelBody from './lib/PanelBody.svelte';
  import SearchField from './lib/SearchField.svelte';
  import SettingsDialog from './lib/SettingsDialog.svelte';
  import StatusBanners from './lib/StatusBanners.svelte';
  import appPackage from '../package.json';
  import { bridge, native, observeSnapshots } from './lib/bridge';
  import { accountLabel, repoLabel } from './lib/format';
  import { m } from './lib/i18n/index.svelte';
  import { Modals } from './lib/modals.svelte';
  import { Panel } from './lib/panel.svelte';
  import { createShortcuts } from './lib/shortcuts';
  import type { Snapshot } from './lib/types';
  import { Updater } from './lib/updater.svelte';
  import { PanelView } from './lib/view.svelte';

  const appVersion = appPackage.version;
  const panel = new Panel({ bridge, native, observe: observeSnapshots });
  const updater = new Updater(native);
  const snapshot = $derived(panel.snapshot);
  const demo = $derived(panel.demo);
  const accountId = $derived(panel.accountId);

  let demoAccountConnected = $state(false);
  let searchField = $state<ReturnType<typeof SearchField>>();
  let appBar = $state<ReturnType<typeof AppBar>>();
  const view = new PanelView(panel);
  const modals = new Modals(panel, view.showRun);
  const { openSettings } = modals;

  const repoName = (id: string) => repoLabel(snapshot, id);
  const accountName = (id: string) => accountLabel(snapshot, id, native);
  const repoAccountName = (id: string) =>
    accountName(
      snapshot?.repos.find((repo) => repo.id === id)?.accountId ?? '',
    );
  /** Empty-state „Suche leeren“: its button goes, focus moves to the field. */
  const clearQuery = () => {
    view.query = '';
    searchField?.focus();
  };
  const warn = (value: string) => panel.showNotice(value, 'warning');
  /**
   * The brand leads home: the overview from its top, without a search. From
   * a maximized page focus returns to its opener; on the overview it stays.
   */
  function goHome() {
    view.query = '';
    if (!view.back())
      document.querySelector('.tray .content')?.scrollTo({ top: 0 });
  }

  /** A load error offers a retry before any snapshot exists. */
  const canRefresh = $derived(!!snapshot || !!panel.error);

  onMount(() => {
    const stop = panel.load(() => {
      if (native) void updater.check(true);
    });
    const updateTimer = native
      ? setInterval(() => void updater.check(true), 6 * 60 * 60 * 1000)
      : undefined;
    return () => {
      clearInterval(updateTimer);
      stop();
    };
  });

  /** A refresh that recovers from a load error also runs the skipped startup update check. */
  async function refreshPanel() {
    const recovering = !panel.snapshot;
    const next = await panel.refresh();
    if (native && recovering && next) void updater.check(true);
  }
  function changeAccount(value: string) {
    panel.changeAccount(value);
    view.reset();
    if (modals.kind === 'dispatch') modals.close();
  }
  function connectedNative(value: Snapshot) {
    const known = new Set(panel.snapshot?.accounts.map((item) => item.id));
    panel.viewEpoch.invalidate();
    panel.accept(value);
    // An explicit connection selects that entry; the service lists it last.
    const account = value.accounts.at(-1);
    changeAccount(account?.id ?? '');
    // A new account shows its newest repositories at once: the overview, set
    // before the dialog closes, so its focus return lands there.
    if (account && !known.has(account.id)) view.home();
    // Success closes the dialog by itself; the notice confirms it.
    modals.close();
    if (account) panel.showNotice(m().shell.connect.connected(account.login));
    void updater.check(true);
  }
  function connectedDemo() {
    demoAccountConnected = true;
    modals.close();
    panel.showNotice(m().shell.connect.demoConnected);
    void tick().then(() => appBar?.focusAccount());
  }

  /** Empty-state actions open the missing step: connect or account. */
  function openNextStep() {
    if (native && !snapshot?.accounts.length) modals.show('connect');
    else appBar?.openAccountMenu();
  }
  const shortcuts = createShortcuts({
    blocked: () => modals.kind !== null,
    refresh: () => void (canRefresh && refreshPanel()),
    toggleSection: view.toggle,
    focusSearch: (select) => searchField?.focus(select),
    // Like the settings button: only once a snapshot is loaded.
    openSettings: () => void (snapshot && openSettings()),
    // Search first, then the open page; menus handle their own Escape.
    escape: (target) => !!searchField?.escape(target) || view.back(),
  });
</script>

<svelte:window onkeydown={shortcuts} />

<!-- `inert` already hides the page from assistive tech while a dialog is
     open; aria-hidden states it for tools that do not model `inert`. -->
<main
  class="desktop"
  inert={modals.kind !== null}
  aria-hidden={modals.kind !== null ? 'true' : undefined}
>
  <section class="tray" aria-label="RepoWatch" data-page={view.page}>
    <AppBar
      bind:this={appBar}
      {native}
      {demo}
      accounts={snapshot?.accounts ?? []}
      {accountId}
      {accountName}
      connectivity={snapshot?.connectivity}
      message={snapshot?.message}
      refreshing={panel.refreshing}
      ready={!!snapshot}
      {canRefresh}
      busy={panel.settingsBusy}
      onAccountChange={changeAccount}
      onHome={goHome}
      onConnect={() => modals.show('connect')}
      onManageAccounts={() => openSettings('accounts')}
      onRefresh={() => void refreshPanel()}
      onSettings={() => openSettings()}
    />

    <PanelBody
      {panel}
      {modals}
      {view}
      {repoName}
      {accountName}
      {repoAccountName}
      onClearQuery={clearQuery}
      onNextStep={openNextStep}
    >
      {#snippet top()}
        <SearchField
          bind:this={searchField}
          page={view.page}
          bind:query={view.query}
        />
        <StatusBanners
          update={updater.info}
          {demo}
          {snapshot}
          error={panel.error}
          notices={panel.notices.items}
          showsAccount={panel.showsAccount}
          paused={modals.kind !== null}
          onShowUpdate={() => openSettings('updates')}
          onShowRun={modals.showRun}
          onDismissNotice={(id) => panel.notices.dismiss(id)}
        />
      {/snippet}
    </PanelBody>

    <AppFooter
      stale={!!snapshot?.stale}
      {demo}
      connectivity={snapshot?.connectivity}
      accountCount={snapshot?.accounts.length ?? 0}
      {demoAccountConnected}
      updatedAt={snapshot?.updatedAt}
      refreshing={panel.refreshing}
      {canRefresh}
      onRefresh={() => void refreshPanel()}
    />
  </section>
</main>

{#if modals.kind === 'settings'}
  <SettingsDialog
    {snapshot}
    {native}
    {demo}
    {accountId}
    accountNotice={panel.accountNotice}
    scenario={panel.scenario}
    {updater}
    {appVersion}
    initialTab={modals.settingsTab}
    busy={panel.settingsBusy}
    returnFocus={modals.returnFocus}
    onClose={modals.close}
    onConnect={() => modals.show('connect')}
    onChangeAccount={changeAccount}
    onScenario={panel.changeScenario}
    onReset={panel.resetDemo}
    onDisconnect={panel.disconnectAccount}
    onUnhide={panel.actions.unhide}
  />
{:else if modals.kind === 'connect'}
  <Modal
    title={m().shell.connect.title}
    eyebrow={`REPOWATCH · ${demo ? 'DEMO' : 'GITHUB'}`}
    returnFocus={modals.returnFocus}
    onClose={modals.close}
  >
    {#if native}<NativeConnect
        accounts={snapshot?.accounts ?? []}
        onConnected={connectedNative}
        onClose={modals.close}
      />{:else}<ConnectPanel
        onConnected={connectedDemo}
        onClose={modals.close}
      />{/if}
  </Modal>
{:else if modals.kind === 'dispatch'}
  <DispatchDialog
    {panel}
    seed={modals.dispatchSeed}
    returnFocus={modals.returnFocus}
    onClose={modals.close}
    onShowRun={modals.showRun}
    onLinkError={warn}
  />
{/if}
