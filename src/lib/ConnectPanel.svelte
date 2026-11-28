<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import ConnectMethods, { type ConnectMethod } from './ConnectMethods.svelte';
  import TokenPermissions from './TokenPermissions.svelte';
  import { m } from './i18n/index.svelte';

  // Browser demo: the same steps as the desktop app, with an example token
  // that is checked locally and never stored.
  let {
    onConnected,
    onClose,
  }: {
    onConnected: () => void;
    onClose: () => void;
  } = $props();
  let step = $state<'choose' | 'token' | 'checking' | 'cancelled'>('choose');
  let method = $state<ConnectMethod>('token');
  let tokenError = $state('');
  let tokenInput = $state<HTMLInputElement>();
  let heading = $state<HTMLHeadingElement>();
  let hasToken = $state(false);
  let generation = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const current = step;
    void tick().then(() => {
      if (step === current) heading?.focus();
    });
  });
  function stop() {
    generation++;
    clearTimeout(timer);
  }
  function submitExampleToken(event: SubmitEvent) {
    event.preventDefault();
    if (step !== 'token' || !hasToken) return;
    const isExample = tokenInput?.value === 'demo-token';
    if (tokenInput) tokenInput.value = '';
    hasToken = false;
    if (!isExample) {
      tokenError = m().accounts.connect.exampleOnly;
      return;
    }
    tokenError = '';
    stop();
    step = 'checking';
    const ticket = generation;
    timer = setTimeout(() => {
      if (ticket === generation && step === 'checking') onConnected();
    }, 400);
  }
  function back() {
    tokenError = '';
    hasToken = false;
    step = 'choose';
  }
  function cancel() {
    stop();
    step = 'cancelled';
  }
  onDestroy(() => {
    stop();
    if (tokenInput) tokenInput.value = '';
  });
</script>

{#if step === 'choose'}
  <!-- A form, so Enter on a method continues like its primary button. -->
  <form
    onsubmit={(event) => {
      event.preventDefault();
      step = 'token';
    }}
  >
    <div class="modal-body connect-step">
      <h3 id="connect-step-title" tabindex="-1" bind:this={heading}>
        {m().accounts.connect.chooseMethod}
      </h3>
      <ConnectMethods
        bind:value={method}
        localDisabled
        localHint={m().accounts.connect.localDesktopOnly}
      />
    </div>
    <div class="modal-actions">
      <button type="button" class="secondary-button" onclick={onClose}
        >{m().common.cancel}</button
      >
      <button type="submit" class="primary-button">{m().common.next}</button>
    </div>
  </form>
{:else if step === 'token'}
  <form onsubmit={submitExampleToken}>
    <div class="modal-body connect-step">
      <h3 id="connect-step-title" tabindex="-1" bind:this={heading}>
        {m().accounts.connect.token}
      </h3>
      <label class="field"
        ><span>{m().accounts.connect.exampleToken}</span><input
          type="password"
          aria-label={m().accounts.connect.exampleToken}
          aria-describedby="token-permissions"
          bind:this={tokenInput}
          autocomplete="off"
          spellcheck="false"
          maxlength="4096"
          placeholder="demo-token"
          oninput={(event) =>
            (hasToken = event.currentTarget.value.trim().length > 0)}
        /></label
      >
      <TokenPermissions />
      {#if tokenError}<div class="alert warning" role="alert">
          {tokenError}
        </div>{/if}
    </div>
    <div class="modal-actions">
      <button type="button" class="secondary-button" onclick={back}
        >{m().common.back}</button
      >
      <button
        type="submit"
        class="primary-button"
        aria-disabled={hasToken ? undefined : 'true'}
        title={hasToken ? undefined : m().accounts.connect.enterToken}
        >{m().accounts.connect.connect}</button
      >
    </div>
  </form>
{:else}
  <div class="modal-body connect-step">
    <h3 id="connect-step-title" tabindex="-1" bind:this={heading}>
      {step === 'checking'
        ? m().accounts.connect.checking
        : m().accounts.connect.cancelled}
    </h3>
  </div>
  <div class="modal-actions">
    <button
      type="button"
      class="secondary-button"
      onclick={() => (step === 'checking' ? cancel() : onClose())}
      >{m().common.cancel}</button
    >
    <button
      type="button"
      class="primary-button"
      aria-disabled={step === 'checking' ? 'true' : undefined}
      onclick={() => {
        if (step === 'cancelled') step = 'token';
      }}
      >{step === 'checking'
        ? m().accounts.connect.checking
        : m().common.retry}</button
    >
  </div>
{/if}
