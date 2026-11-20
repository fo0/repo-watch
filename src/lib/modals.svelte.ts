import type { DispatchSeed, FavoriteMode } from './DispatchDialog.svelte';
import type { SettingsTab } from './settingsTabs';
import type { Panel } from './panel.svelte';
import type { Pin, Run, Workflow } from './types';

/**
 * Opens the Verlauf with the run expanded and focused; `opener` is the
 * element that opened the closed dialog, if any, for the way back.
 */
export type RevealRun = (
  runId: number | null,
  opener?: HTMLElement | null,
) => unknown;

export type ModalKind = 'settings' | 'dispatch' | 'connect';

/**
 * Which dialog is open and where focus returns afterwards. Dialogs own only
 * their form state; running operations live in the Panel and outlive them.
 */
export class Modals {
  kind = $state<ModalKind | null>(null);
  settingsTab = $state<SettingsTab>('accounts');
  dispatchSeed = $state.raw<DispatchSeed>({});
  /** Element that opened the first of possibly chained dialogs. */
  opener: HTMLElement | null = null;
  #panel: Panel;
  #reveal: RevealRun;

  /** `reveal` switches to the full Verlauf with the run expanded (`PanelView.showRun`). */
  constructor(panel: Panel, reveal: RevealRun) {
    this.#panel = panel;
    this.#reveal = reveal;
  }

  returnFocus = () => this.opener;
  close = () => {
    this.kind = null;
  };
  /** Opens or replaces the dialog; a replacement keeps the first opener. */
  show = (kind: ModalKind) => {
    if (!this.kind)
      this.opener =
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null;
    this.kind = kind;
  };
  openSettings = (initial: SettingsTab = 'accounts') => {
    this.settingsTab = initial;
    this.show('settings');
  };
  openDispatch = (
    pin?: Pin,
    workflow?: Workflow,
    favorite?: FavoriteMode,
    ref?: string,
  ) => {
    this.dispatchSeed = { pin, workflow, favorite, ref };
    this.show('dispatch');
  };
  /**
   * „Erneut starten …“ on a run: the start dialog with its workflow and
   * branch and the workflow's default inputs. Nothing starts until the user
   * presses the start button there.
   */
  startAgain = (workflow: Workflow, run: Run) =>
    this.openDispatch(undefined, workflow, undefined, run.branch);
  /** Opens a favorite in the start dialog to edit its defaults. */
  editPin = (pin: Pin) => this.openDispatch(pin, undefined, 'edit');
  /** Starts a favorite at once; one that still needs input opens the dialog. */
  launchPin = (pin: Pin) => {
    if (this.#panel.favorites.launch(pin) === 'incomplete') this.editPin(pin);
  };
  /** Toggles a heart favorite; one that still needs input opens the dialog. */
  toggleFavorite = async (workflow: Workflow) => {
    if ((await this.#panel.favorites.toggle(workflow)) === 'incomplete')
      this.openDispatch(undefined, workflow, 'add');
  };
  /** Shows an accepted start in the full Verlauf and focuses its row. */
  showRun = async (id: number | null) => {
    // From a dialog, its opener is where the way back leads; from a notice,
    // the reveal takes the focused element itself.
    const opener = this.kind ? this.opener : undefined;
    this.close();
    await this.#reveal(id, opener);
  };
}
