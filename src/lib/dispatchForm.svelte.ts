import type { DispatchSeed } from './DispatchDialog.svelte';
import { m } from './i18n/index.svelte';
import { defaultInputs, inputProblem, validateDispatch } from './model';
import type { Panel } from './panel.svelte';
import type { DispatchRequest, DispatchTarget, InputValue, Pin } from './types';

/**
 * Form state of the start dialog: target, inputs and favorite fields.
 * `onRetarget` runs whenever the target changes, so the dialog can drop
 * results and pending answers that belong to the previous target.
 */
export class DispatchForm {
  repoId = $state('');
  workflowId = $state(0);
  ref = $state('main');
  inputs = $state<Record<string, InputValue>>({});
  pinLabel = $state('');
  /** The legacy `Pin.quickStart` the edited favorite had; saved unchanged. */
  #pinQuickStart = false;
  editingPinId = $state<string | null>(null);
  #panel: Panel;
  #onRetarget: () => void;

  repo = $derived.by(() =>
    this.#panel.snapshot?.repos.find((repo) => repo.id === this.repoId),
  );
  workflows = $derived.by(
    () =>
      this.#panel.snapshot?.workflows.filter(
        (workflow) => workflow.repoId === this.repoId,
      ) ?? [],
  );
  workflow = $derived(
    this.workflows.find((workflow) => workflow.id === this.workflowId),
  );
  target = $derived<DispatchTarget>({
    accountId: this.repo?.accountId ?? '',
    repoId: this.repoId,
    workflowId: this.workflowId,
    ref: this.ref,
  });
  validation = $derived.by(() =>
    this.#panel.snapshot && this.repo && this.workflow
      ? validateDispatch(this.#panel.snapshot, this.request('validation'))
      : m().dispatch.dialog.chooseTarget,
  );
  /** The workflow input the validation message names; the form marks it. */
  invalidInput = $derived.by(() => {
    const problem = this.workflow && inputProblem(this.workflow, this.inputs);
    return problem && problem.message === this.validation ? problem.name : null;
  });

  constructor(panel: Panel, seed: DispatchSeed, onRetarget: () => void) {
    this.#panel = panel;
    this.#onRetarget = onRetarget;
    const { pin, workflow } = seed;
    const repos = panel.repos;
    this.selectRepo(
      pin?.repoId ??
        workflow?.repoId ??
        repos.find((repo) => repo.permission === 'write')?.id ??
        repos[0]?.id ??
        '',
    );
    if (workflow) this.selectWorkflow(workflow.id);
    if (pin) {
      this.selectWorkflow(pin.workflowId);
      this.ref = pin.ref;
      this.inputs = { ...pin.inputs };
      this.pinLabel = pin.label;
      this.#pinQuickStart = pin.quickStart === true;
      this.editingPinId = pin.id;
    }
  }

  request(id: string): DispatchRequest {
    return { ...this.target, inputs: { ...this.inputs }, requestId: id };
  }
  selectWorkflow = (value: number) => {
    this.#onRetarget();
    this.workflowId = value;
    this.editingPinId = null;
    const workflow = this.workflows.find((item) => item.id === value);
    this.inputs = workflow ? defaultInputs(workflow) : {};
    this.ref = this.repo?.defaultBranch ?? 'main';
    if (workflow && !workflow.refs.includes(this.ref))
      this.ref = workflow.refs[0] ?? '';
    this.pinLabel = workflow?.name ?? '';
    this.#pinQuickStart = false;
  };
  selectRepo = (value: string) => {
    this.repoId = value;
    // `workflows` already follows the new repository.
    const first =
      this.workflows.find((item) => item.dispatch) ?? this.workflows[0];
    this.selectWorkflow(first?.id ?? 0);
  };
  changeRef = (value: string) => {
    this.#onRetarget();
    this.ref = value;
  };
  setInput = (name: string, value: InputValue) => {
    this.inputs = { ...this.inputs, [name]: value };
  };
  /** The favorite to save: the edited one, or a new copy. */
  pin(asNew: boolean): Pin {
    return {
      id: !asNew && this.editingPinId ? this.editingPinId : crypto.randomUUID(),
      label: this.pinLabel,
      ...this.target,
      inputs: { ...this.inputs },
      quickStart: this.#pinQuickStart,
    };
  }
}
