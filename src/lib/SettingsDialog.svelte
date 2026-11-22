<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import AccountsTab from './AccountsTab.svelte';
  import HiddenReposSetting from './HiddenReposSetting.svelte';
  import LanguageSetting from './LanguageSetting.svelte';
  import LogSetting from './LogSetting.svelte';
  import Modal from './Modal.svelte';
  import RefreshSetting from './RefreshSetting.svelte';
  import StartupSettings from './StartupSettings.svelte';
  import ThemeSetting from './ThemeSetting.svelte';
  import UpdatePanel from './UpdatePanel.svelte';
  import { AppearanceChoice } from './appearanceChoice.svelte';
  import { Autostart } from './autostart.svelte';
  import { bridge } from './bridge';
  import { errorMessage } from './format';
  import { HiddenChoice } from './hiddenChoice.svelte';
  import { m } from './i18n/index.svelte';
  import { LogChoice } from './logChoice.svelte';
  import { RefreshChoice } from './refresh.svelte';
  import {
    SETTINGS_TABS,
    settingsTabLabel,
    type SettingsTab,
  } from './settingsTabs';
  import type { DemoScenario, HiddenRepo, Snapshot } from './types';
  import type { Updater } from './updater.svelte';

  let {
    snapshot,
    native,
    demo,
    accountId,
    accountNotice,
    scenario,
    updater,
    appVersion,
    initialTab = 'accounts',
    busy,
    returnFocus,
    onClose,
    onConnect,
    onChangeAccount,
    onScenario,
    onReset,
    onDisconnect,
    onUnhide,
  }: {
    snapshot: Snapshot | null;
    native: boolean;
    demo: boolean;
    accountId: string;
    accountNotice: string;
    scenario: DemoScenario;
    updater: Updater;
    appVersion: string;
    initialTab?: SettingsTab;
    /** A settings mutation is running. */
    busy: boolean;
    returnFocus?: (() => HTMLElement | null | undefined) | undefined;
    /** Closes without applying: „Abbrechen“, Escape and the close button. */
    onClose: () => void;
    /** Replaces this dialog with the connection dialog; staged changes are dropped. */
    onConnect: () => void;
    /** Applied by „Speichern“ only. */
    onChangeAccount: (id: string) => void;
    // Mutations reject with a user-facing error; the dialog shows it.
    onScenario: (value: DemoScenario) => Promise<unknown>;
    onReset: () => Promise<unknown>;
    onDisconnect: (id: string) => Promise<unknown>;
    onUnhide: (entry: HiddenRepo) => Promise<unknown>;
  } = $props();

  // Simple settings are staged and applied together by „Speichern“; account
  // actions („Konto hinzufügen“, „Trennen“) and updates act at once.
  let tab = $state<SettingsTab>(untrack(() => initialTab));
  let error = $state('');
  let saving = $state(false);
  let alive = true;
  /** null: untouched, the setting in effect shows. */
  let accountChoice = $state<string | null>(null);
  let scenarioChoice = $state<DemoScenario | null>(null);
  const appearance = new AppearanceChoice(bridge);
  const autostart = new Autostart(untrack(() => native));
  const refresh = new RefreshChoice(bridge);
  const hidden = new HiddenChoice();
  const log = new LogChoice(bridge);
  onDestroy(() => {
    alive = false;
    autostart.dispose();
    // Closing without „Speichern“ drops the preview.
    appearance.restore();
  });
  /** A staged account that was disconnected meanwhile falls back. */
  const account = $derived(
    accountChoice !== null &&
      snapshot?.accounts.some((item) => item.id === accountChoice)
      ? accountChoice
      : accountId,
  );
  const draftScenario = $derived(scenarioChoice ?? scenario);
  const changed = $derived({
    account: native && account !== accountId,
    scenario: demo && draftScenario !== scenario,
    theme: appearance.themePending,
    language: appearance.languagePending,
    autostart: autostart.pending !== null,
    refresh: refresh.pending,
    log: log.pending,
    hidden: hidden.pendingIn(snapshot?.hiddenRepos ?? []),
  });
  const dirty = $derived(Object.values(changed).some(Boolean));
  const saveBlocked = $derived(!dirty || saving || busy || refresh.invalid);
  /** Why „Speichern“ is off although something changed, if a value is invalid. */
  const invalidReason = $derived(
    refresh.invalid ? m().settings.refresh.invalid : null,
  );
  /**
   * Tab states: a dot whose text is part of the tab's name, like the activity
   * count. An invalid value marks its tab, so the blocked „Speichern“ is
   * explained from every tab.
   */
  const badges = $derived<
    Partial<Record<SettingsTab, { text: string; tone?: 'danger' }>>
  >({
    ...(updater.info && {
      updates: { text: m().settings.dialog.updateAvailable },
    }),
    ...(invalidReason && { app: { text: invalidReason, tone: 'danger' } }),
  });

  /** Runs a mutation and shows its error while this dialog is still open. */
  async function attempt(run: () => Promise<unknown>) {
    error = '';
    try {
      await run();
    } catch (cause) {
      if (alive) error = errorMessage(cause);
    }
  }
  const reset = () =>
    attempt(async () => {
      await onReset();
      if (alive) onClose();
    });
  const disconnect = (id: string) => attempt(() => onDisconnect(id));
  const openLogFolder = () => void attempt(() => bridge.openLogFolder());

  /**
   * Applies every staged change in turn and closes. A change that applied
   * becomes the new baseline; the first failure keeps the dialog open with
   * its error, and the remaining changes stay staged.
   */
  async function save() {
    if (saveBlocked) return;
    const pending = changed;
    saving = true;
    error = '';
    try {
      if (pending.scenario) {
        await onScenario(draftScenario);
        scenarioChoice = null;
      }
      if (pending.account) {
        onChangeAccount(account);
        accountChoice = null;
        // The choice holds for this session, but could not be remembered.
        if (accountNotice) throw new Error(accountNotice);
      }
      if (pending.theme) appearance.applyTheme();
      if (pending.language) appearance.applyLanguage();
      if (pending.refresh) await refresh.apply();
      if (pending.log) await log.apply();
      if (pending.hidden)
        await hidden.apply(snapshot?.hiddenRepos ?? [], onUnhide);
      if (pending.autostart) await autostart.apply();
      if (alive) onClose();
    } catch (cause) {
      if (alive) error = errorMessage(cause);
    } finally {
      if (alive) saving = false;
    }
  }

  function tabKey(event: KeyboardEvent & { currentTarget: HTMLElement }) {
    const direction =
      event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (!direction && event.key !== 'Home' && event.key !== 'End') return;
    event.preventDefault();
    const count = SETTINGS_TABS.length;
    const index = SETTINGS_TABS.indexOf(tab);
    const next =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? count - 1
          : (index + direction + count) % count;
    const target = SETTINGS_TABS[next];
    if (!target) return;
    tab = target;
    event.currentTarget
      .querySelectorAll<HTMLButtonElement>('[role=tab]')
      [next]?.focus();
  }
</script>

<Modal
  title={m().settings.dialog.title}
  eyebrow={`REPOWATCH · ${demo ? 'DEMO' : 'GITHUB'}`}
  wide
  {returnFocus}
  onClose={() => !saving && onClose()}
>
  <div
    class="settings-tabs"
    role="tablist"
    tabindex="-1"
    aria-label={m().settings.dialog.tabsLabel}
    onkeydown={tabKey}
  >
    {#each SETTINGS_TABS as item (item)}
      <button
        id={`settings-tab-${item}`}
        role="tab"
        aria-selected={tab === item}
        aria-controls="settings-panel"
        tabindex={tab === item ? 0 : -1}
        onclick={() => (tab = item)}
        >{settingsTabLabel(item)}{#if badges[item]}<span
            class="tab-update-dot"
            data-tone={badges[item]?.tone}
            role="img"
            aria-label={badges[item]?.text}
            title={badges[item]?.text}
          ></span>{/if}</button
      >
    {/each}
  </div>
  <div
    class="modal-body"
    id="settings-panel"
    role="tabpanel"
    aria-labelledby={`settings-tab-${tab}`}
    tabindex="0"
  >
    {#if tab === 'accounts'}
      <AccountsTab
        {snapshot}
        {native}
        {demo}
        {accountId}
        choice={account}
        accountNotice={accountNotice === error ? '' : accountNotice}
        scenario={draftScenario}
        busy={busy || saving}
        onChoose={(id) => (accountChoice = id)}
        {onConnect}
        onScenario={(value) => (scenarioChoice = value)}
        onReset={reset}
        onDisconnect={disconnect}
      />
    {:else if tab === 'app'}
      <LanguageSetting bind:value={appearance.language} />
      <ThemeSetting bind:value={appearance.theme} />
      <RefreshSetting choice={refresh} />
      <HiddenReposSetting {snapshot} choice={hidden} />
      <LogSetting choice={log} {native} onOpenFolder={openLogFolder} />
      <StartupSettings {native} {autostart} />
    {:else}
      <UpdatePanel
        currentVersion={appVersion}
        {native}
        info={updater.info}
        phase={updater.phase}
        progress={updater.progress}
        error={updater.message}
        checkedAt={updater.checkedAt}
        onCheck={() => updater.check()}
        onInstall={updater.install}
      />
    {/if}
  </div>
  {#if error}<div class="dialog-alert danger" role="alert">
      {error}
    </div>{/if}
  <div class="modal-actions">
    <button
      type="button"
      class="secondary-button"
      disabled={saving}
      onclick={() => onClose()}>{m().common.cancel}</button
    ><!-- Off via aria-disabled: it keeps focus while saving. --><button
      type="button"
      class="primary-button"
      aria-disabled={saveBlocked ? 'true' : undefined}
      title={invalidReason ??
        (dirty || saving ? undefined : m().settings.dialog.noChanges)}
      onclick={() => void save()}
      >{saving ? m().common.saving : m().common.save}</button
    >
  </div>
</Modal>
