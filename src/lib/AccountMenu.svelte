<script lang="ts">
  import Icon from './Icon.svelte';
  import { avatarLetter } from './format';
  import { m } from './i18n/index.svelte';
  import { menu, openingFocus, type MenuFocus } from './menu';
  import type { Account, Snapshot } from './types';

  let {
    native,
    accounts,
    accountId,
    accountName,
    connectivity,
    message,
    disabled,
    onChange,
    onConnect,
    onManage,
  }: {
    native: boolean;
    accounts: Account[];
    /** The active account; '' is none natively and all accounts in the demo. */
    accountId: string;
    /** Display name of an account: login and source natively, the login in the demo. */
    accountName: (id: string) => string;
    /** Undefined while the first snapshot loads. */
    connectivity: Snapshot['connectivity'] | undefined;
    /** The snapshot's reason, the one the warning banner shows. */
    message: string | undefined;
    /** No snapshot yet, or a settings change is running. */
    disabled: boolean;
    onChange: (id: string) => void;
    /** Opens the connection dialog. */
    onConnect: () => void;
    /** Opens settings on the accounts tab. */
    onManage: () => void;
  } = $props();

  const id = $props.id();
  let open = $state(false);
  let initial = $state<MenuFocus>('first');
  let trigger = $state<HTMLButtonElement>();

  const active = $derived(accounts.find((account) => account.id === accountId));
  // Natively '' means no deliberate choice yet; only the demo has an aggregate.
  const label = $derived(
    active
      ? accountName(active.id)
      : native
        ? m().common.noActiveAccount
        : m().accounts.menu.all,
  );
  const letter = $derived(
    active ? avatarLetter(active.login, '?') : native ? '' : 'A',
  );
  const presence = $derived(
    !connectivity || (native && !active)
      ? ''
      : connectivity === 'online'
        ? 'ok'
        : connectivity === 'offline'
          ? 'wait'
          : 'fail',
  );
  // A yellow or red dot says why in the tooltip and to assistive tech; the
  // banner may be scrolled away. Same text as the banner when there is one.
  const reason = $derived(
    presence === 'wait' || presence === 'fail'
      ? message ||
          (presence === 'wait'
            ? m().accounts.menu.reason.offline
            : m().accounts.menu.reason.forbidden)
      : '',
  );

  // A settings change that starts meanwhile closes the menu.
  $effect(() => {
    if (disabled) open = false;
  });

  /** Moves focus to the menu button, e.g. after a demo connection. */
  export function focus() {
    trigger?.focus();
  }
  /** Opens the menu on its first entry, e.g. from „Konto wählen“. */
  export function openMenu() {
    if (!disabled) show('first');
  }

  function show(focus: MenuFocus) {
    initial = focus;
    open = true;
  }
  function onTriggerKey(event: KeyboardEvent) {
    const focus = openingFocus(event.key);
    if (!focus) return;
    event.preventDefault();
    show(focus);
  }
  /** Closes first, so a dialog opened next records the button as its opener. */
  function run(action: () => void) {
    trigger?.focus();
    open = false;
    action();
  }
  function choose(id: string) {
    run(() => {
      if (id !== accountId) onChange(id);
    });
  }
</script>

{#snippet item(value: string, name: string, initialLetter: string)}
  {@const checked = value === accountId}
  <button
    type="button"
    class="menu-item"
    role="menuitemradio"
    aria-checked={checked}
    tabindex="-1"
    onclick={() => choose(value)}
    ><span class="avatar" class:current={checked} aria-hidden="true"
      >{initialLetter}</span
    ><span class="menu-item-label">{name}</span>{#if checked}<Icon
        name="check"
        size={15}
        class="menu-check"
      />{/if}</button
  >
{/snippet}

<div class="account-menu">
  <div
    class="account-status"
    role="status"
    aria-label={m().accounts.menu.active}
  >
    <button
      bind:this={trigger}
      type="button"
      class="account-trigger"
      class:open
      aria-haspopup="menu"
      aria-expanded={open}
      aria-controls={open ? `${id}-menu` : undefined}
      aria-label={m().accounts.menu.switchTo(
        active || !native ? m().accounts.menu.named(label) : label,
      )}
      aria-describedby={reason ? `${id}-reason` : undefined}
      title={reason ? `${label}\n${reason}` : label}
      {disabled}
      onclick={() => (open ? (open = false) : show('first'))}
      onkeydown={onTriggerKey}
      ><span class="avatar" class:current={!!letter} aria-hidden="true"
        >{#if letter}{letter}{:else}<Icon
            name="github"
            size={13}
          />{/if}{#if presence}<span class={`presence ${presence}`}
          ></span>{/if}</span
      ><span class="account-name">{label}</span><Icon
        name="down"
        size={14}
        class="account-caret"
      /></button
    >{#if reason}<span id={`${id}-reason`} class="sr-only">{reason}</span>{/if}
  </div>
  {#if open}
    <!-- tabindex="-1": a press on the label, separator or padding focuses the
         menu itself instead of the page, so focusout does not close it. -->
    <div
      id={`${id}-menu`}
      class="menu account-list"
      role="menu"
      tabindex="-1"
      aria-label={m().accounts.menu.switch}
      use:menu={{ trigger, initial, onClose: () => (open = false) }}
    >
      <!-- Natively without a connected account there is nothing to choose. -->
      {#if !native || accounts.length}
        <div class="menu-label" id={`${id}-accounts`} aria-hidden="true">
          {m().accounts.menu.group}
        </div>
        <div role="group" aria-labelledby={`${id}-accounts`}>
          {#if !native}{@render item('', m().accounts.menu.all, 'A')}{/if}
          {#each accounts as account (account.id)}
            {@render item(
              account.id,
              accountName(account.id),
              avatarLetter(account.login, '?'),
            )}
          {/each}
        </div>
        <div class="menu-separator" role="separator"></div>
      {/if}
      <button
        type="button"
        class="menu-item"
        role="menuitem"
        tabindex="-1"
        onclick={() => run(onConnect)}
        ><Icon name="userPlus" size={16} />{m().accounts.menu.connect}</button
      >
      <button
        type="button"
        class="menu-item"
        role="menuitem"
        tabindex="-1"
        onclick={() => run(onManage)}
        ><Icon name="settings" size={16} />{m().accounts.menu.manage}</button
      >
    </div>
  {/if}
</div>
