<script module lang="ts">
  import type { Pin, Workflow } from './types';

  /**
   * Why the favorite section opens up front: `add` for a heart favorite
   * that still needs input, `edit` to change a favorite's defaults.
   */
  export type FavoriteMode = 'add' | 'edit';

  /** What the start dialog opens with. */
  export interface DispatchSeed {
    pin?: Pin | undefined;
    workflow?: Workflow | undefined;
    favorite?: FavoriteMode | undefined;
  }
</script>

<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';
  import DispatchActions from './DispatchActions.svelte';
  import DispatchOutcome from './DispatchOutcome.svelte';
  import DispatchTargetFields from './DispatchTargetFields.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';
  import SavePinPanel from './SavePinPanel.svelte';
  import WorkflowInputs from './WorkflowInputs.svelte';
  import { DispatchForm } from './dispatchForm.svelte';
  import { accountLabel, errorMessage } from './format';
  import { m } from './i18n/index.svelte';
  import { RequestEpoch, targetKey } from './model';
  import { blockedReason, startReason } from './startReason';
  import type { Panel } from './panel.svelte';
  import type { DispatchOutcome as Outcome, UncertainStart } from './types';

  let {
    panel,
    seed,
    returnFocus,
    onClose,
    onShowRun,
    onLinkError,
  }: {
    panel: Panel;
    seed: DispatchSeed;
    returnFocus?: (() => HTMLElement | null | undefined) | undefined;
    onClose: () => void;
    onShowRun: (runId: number) => void;
    onLinkError: (message: string) => void;
  } = $props();

  const snapshot = $derived(panel.snapshot);
  const demo = $derived(panel.demo);
  let outcome = $state<Outcome | null>(null);
  let error = $state('');
  let notice = $state('');
  let noticeTone = $state<'subtle' | 'warning'>('subtle');
  let result = $state<ReturnType<typeof DispatchOutcome>>();
  /** The start this dialog watches: its own or one running for its target. */
  let watched = $state<number | null>(null);
  /** Bumped whenever the target changes; a stale ticket hands results to the banner. */
  const epoch = new RequestEpoch();
  let alive = true;
  onDestroy(() => {
    alive = false;
    epoch.invalidate();
  });
  function clearMessages() {
    outcome = null;
    error = '';
    notice = '';
    watched = null;
  }
  const form = untrack(
    () =>
      new DispatchForm(panel, seed, () => {
        epoch.invalidate();
        clearMessages();
      }),
  );
  if (untrack(() => seed.favorite === 'add'))
    notice = m().dispatch.dialog.completeFavorite;

  const target = $derived(form.target);
  const uncertain = $derived(panel.starts.isQuarantined(target));
  /** The persisted claim behind `uncertain` as reported, else this session's untimed lock. */
  const claim = $derived(panel.starts.claimFor(target));
  /** A start for this target is running, from this dialog or a direct start. */
  const starting = $derived(panel.starts.isRunning(target));
  $effect.pre(() => {
    const seq = panel.starts.runningSeq(target);
    if (seq !== undefined) watched = seq;
  });
  /** Result of a watched start this dialog does not own, shown read-only. */
  const followed = $derived.by(() => {
    const last = watched === null ? undefined : panel.starts.settledFor(target);
    return last && last.seq === watched ? last.result : null;
  });
  const shownOutcome = $derived(
    outcome ?? (followed && 'outcome' in followed ? followed.outcome : null),
  );
  const shownError = $derived(
    error || (followed && 'error' in followed ? followed.error : ''),
  );
  /** One reason why the start button is off, the most basic cause first. */
  const reason = $derived(
    startReason({
      manual: form.workflow?.dispatch === false,
      readOnly: !!form.repo && form.repo.permission !== 'write',
      blocked: blockedReason(snapshot),
      running: starting,
      uncertain: uncertain || shownOutcome?.state === 'uncertain',
    }) ??
      (shownOutcome?.state === 'accepted'
        ? m().dispatch.dialog.alreadyStarted
        : panel.templateBusy
          ? m().dispatch.dialog.savingFavorite
          : form.validation),
  );

  async function start() {
    if (starting || panel.templateBusy) return;
    const ticket = epoch.next();
    clearMessages();
    const answer = await panel.starts.dispatch(
      form.request(crypto.randomUUID()),
      // Closing, replacing or retargeting the dialog hands the result to the banner.
      { owner: () => alive && epoch.current(ticket) },
    );
    if (!answer) return;
    if ('error' in answer) error = answer.error;
    else outcome = answer.outcome;
    await tick();
    result?.focus();
  }
  async function checkStatus() {
    const key = targetKey(target);
    const latest = await panel.refresh();
    if (!alive || targetKey(target) !== key) return;
    if (!latest || latest.stale || latest.connectivity !== 'online') {
      notice = '';
      error = m().dispatch.uncertain.checkFailed;
    } else {
      error = '';
      noticeTone = 'warning';
      notice = m().dispatch.uncertain.stillUnconfirmed;
    }
  }
  /**
   * Explicit release after checking GitHub; never starts anything. Releases
   * the claim the confirmation held when it opened, never a newer one.
   */
  async function release(confirmed?: UncertainStart) {
    const held = confirmed ?? claim;
    if (!held) return;
    const key = targetKey(target);
    await panel.starts.release(held);
    if (!alive || targetKey(target) !== key) return;
    outcome = null;
    watched = null;
    error = '';
    noticeTone = 'subtle';
    notice = m().dispatch.uncertain.released;
    await tick();
    result?.focus();
  }
  async function saveTemplate(asNew: boolean) {
    if (!form.repo || panel.templateBusy || starting) return;
    const pin = form.pin(asNew);
    const ticket = epoch.next();
    error = '';
    notice = '';
    try {
      if (!(await panel.saveTemplate(pin)) || !alive || !epoch.current(ticket))
        return;
      form.editingPinId = pin.id;
      noticeTone = 'subtle';
      notice = m().favorites.notices.saved(pin.label.trim());
    } catch (cause) {
      if (alive && epoch.current(ticket)) error = errorMessage(cause);
    }
  }
</script>

<Modal
  title={m().dispatch.dialog.title}
  eyebrow={`REPOWATCH · ${demo ? 'DEMO' : 'GITHUB'}`}
  {returnFocus}
  {onClose}
>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      // Starts require an explicit activation of the start button.
    }}
  >
    <div class="modal-body dispatch-body">
      <div class="simulation-note">
        <Icon name="shield" size={15} /><span
          >{demo
            ? m().dispatch.dialog.demoNote
            : m().dispatch.dialog.liveNote}</span
        >
      </div>
      <DispatchTargetFields
        repos={panel.repos}
        workflows={form.workflows}
        repoId={form.repoId}
        workflowId={form.workflowId}
        gitRef={form.ref}
        defaultBranch={form.repo?.defaultBranch}
        disabled={starting}
        accountName={(id) => accountLabel(snapshot, id, panel.native)}
        onRepo={form.selectRepo}
        onWorkflow={form.selectWorkflow}
        onRef={form.changeRef}
      />
      <WorkflowInputs
        workflow={form.workflow}
        values={form.inputs}
        invalid={form.invalidInput}
        describedBy="dispatch-validation"
        disabled={starting}
        onChange={(name, value) => {
          form.setInput(name, value);
          error = '';
        }}
      />
      <!-- Status regions: the text changes while the form is edited or the
           connection drops, and a screen reader hears it without moving. -->
      {#if form.validation}<div
          class="alert warning"
          id="dispatch-validation"
          role="status"
        >
          <Icon name="shield" size={16} /><span>{form.validation}</span>
        </div>{/if}
      {#if panel.blocked}<div class="alert warning" role="status">
          <Icon name="warning" size={16} /><span
            >{snapshot?.connectivity === 'offline'
              ? m().dispatch.dialog.offline
              : m().dispatch.dialog.forbidden}</span
          >
        </div>{/if}
      <SavePinPanel
        open={!!seed.favorite}
        bind:label={form.pinLabel}
        editing={!!form.editingPinId}
        saving={panel.templateBusy}
        locked={starting}
        valid={!form.validation}
        onSave={(asNew) => void saveTemplate(asNew)}
      />
      <DispatchOutcome
        bind:this={result}
        outcome={shownOutcome}
        error={shownError}
        {notice}
        {noticeTone}
        {uncertain}
        {demo}
        native={panel.native}
        checking={panel.refreshing}
        releasable={!!claim}
        {claim}
        releasing={panel.starts.isReleasing(target)}
        onCheckStatus={checkStatus}
        onRelease={release}
        {onShowRun}
        {onLinkError}
      />
    </div>
    <DispatchActions
      {demo}
      {starting}
      finished={!!shownOutcome}
      canStart={!reason}
      {reason}
      {onClose}
      onStart={() => void start()}
    />
  </form>
</Modal>
