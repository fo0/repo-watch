// Cancelling open runs, loading repositories the search found, hiding
// repositories from the lists and choosing the owners an account watches.
import { tick } from 'svelte';
import type { NoticeTone } from './StatusBanners.svelte';
import type { Bridge } from './demo';
import { UNDO_TIMEOUT } from './favorites.svelte';
import { focusLost, hasDom, landing } from './focus';
import { errorMessage } from './format';
import { hiddenKey } from './hiddenRepos';
import { m } from './i18n/index.svelte';
import type { RequestEpoch } from './model';
import type { Notices } from './notices.svelte';
import { isActive } from './runs';
import type { HiddenRepo, Repo, Run, Snapshot } from './types';

type ActionPort = Pick<
  Bridge,
  'cancelRun' | 'openRepository' | 'setRepoHidden' | 'setOwnerFilter'
>;
/** The notice of one hidden repository and its „Rückgängig“. */
const hideNotice = (accountId: string, repoId: string) =>
  `hide:${hiddenKey(accountId, repoId)}`;
/** The toggle of the row after (else before) a repository's row, by its id. */
function neighbourRow(repoId: string) {
  const row = document.querySelector(`[data-repo-id="${CSS.escape(repoId)}"]`);
  return (
    [row?.nextElementSibling, row?.previousElementSibling]
      .map((item) => item?.getAttribute('data-repo-id'))
      .find(Boolean) ?? null
  );
}
const rowToggle = (repoId: string | null) =>
  repoId
    ? document.querySelector<HTMLElement>(
        `[data-repo-id="${CSS.escape(repoId)}"] .repository-toggle`,
      )
    : null;

/** The panel state these actions read and change. */
export interface ActionHost {
  readonly viewEpoch: RequestEpoch;
  readonly expandedRepos: Set<string>;
  /** A mutation is running; it answers with its own snapshot. */
  readonly mutating: boolean;
  refreshing: boolean;
  accept(value: Snapshot): void;
  toggleRepo(repoId: string): void;
  showNotice(text: string, tone?: NoticeTone): void;
  readonly notices: Notices;
}

/**
 * Run cancellation and on-demand repositories. A cancel request is sent once;
 * the run stays „wird abgebrochen“ until a snapshot reports it closed, since
 * GitHub's acceptance is not the run's end.
 */
export class Actions {
  /** Runs with an accepted or pending cancel request. */
  cancelling = $state.raw(new Set<number>());
  /** Repositories whose workflows are loading. */
  opening = $state.raw(new Set<string>());
  /** Repositories whose hide or show request is running. */
  hiding = $state.raw(new Set<string>());
  /** Notice keys that still offer „Rückgängig“ for a hidden repository. */
  #undo = new Set<string>();
  #host: ActionHost;
  #bridge: ActionPort;

  constructor(host: ActionHost, bridge: ActionPort) {
    this.#host = host;
    this.#bridge = bridge;
  }

  /** The run's cancel request is pending or accepted and it is still open. */
  isCancelling = (run: Run) => isActive(run) && this.cancelling.has(run.id);

  cancelRun = async (run: Run) => {
    if (this.cancelling.has(run.id) || !isActive(run)) return;
    this.cancelling = new Set(this.cancelling).add(run.id);
    const ticket = this.#ticket();
    // A changed account cleared the set: its answer belongs to another view.
    const current = () => this.cancelling.has(run.id);
    try {
      const value = await this.#bridge.cancelRun(run.repoId, run.id);
      if (!current()) return;
      this.#accept(ticket, value);
      this.#host.showNotice(m().actions.cancelRequested(run.id));
    } catch (cause) {
      if (!current()) return;
      this.#drop('cancelling', run.id);
      this.#host.showNotice(errorMessage(cause), 'warning');
    }
  };

  /** Expands the repository at once and loads its workflows into it. */
  openRepository = async (repoId: string) => {
    if (this.opening.has(repoId)) return;
    this.opening = new Set(this.opening).add(repoId);
    if (!this.#host.expandedRepos.has(repoId)) this.#host.toggleRepo(repoId);
    const ticket = this.#ticket();
    try {
      const value = await this.#bridge.openRepository(repoId);
      if (this.opening.has(repoId)) this.#accept(ticket, value);
    } catch (cause) {
      if (!this.opening.has(repoId)) return;
      if (this.#host.expandedRepos.has(repoId)) this.#host.toggleRepo(repoId);
      this.#host.showNotice(errorMessage(cause), 'warning');
    } finally {
      this.#drop('opening', repoId);
    }
  };

  /**
   * Hides a repository from the lists at once and offers „Rückgängig“, or
   * shows a hidden one again (its menu in a search). Focus in the vanished
   * row moves to the next row, else to the page's landing.
   */
  setHidden = async (repo: Repo, hidden: boolean) => {
    if (this.hiding.has(repo.id)) return;
    this.hiding = new Set(this.hiding).add(repo.id);
    const key = hideNotice(repo.accountId, repo.id);
    const next = hasDom() ? neighbourRow(repo.id) : null;
    const ticket = this.#ticket();
    try {
      const value = await this.#bridge.setRepoHidden(
        repo.accountId,
        repo.id,
        hidden,
      );
      this.#accept(ticket, value);
      this.#forget(key);
      if (!hidden)
        return this.#host.showNotice(m().actions.repoShown(repo.name));
      this.#undo.add(key);
      this.#host.notices.show({
        key,
        text: m().actions.repoHidden(repo.name),
        tone: 'subtle',
        timeout: UNDO_TIMEOUT,
        action: { label: m().common.undo, run: () => void this.#restore(repo) },
      });
      if (!hasDom()) return;
      await tick();
      if (focusLost()) (rowToggle(next) ?? landing())?.focus();
    } catch (cause) {
      this.#host.showNotice(errorMessage(cause), 'warning');
    } finally {
      this.#drop('hiding', repo.id);
    }
  };

  /**
   * Shows a hidden repository again from the settings. Rejects with the
   * shell's message, which the dialog shows.
   */
  unhide = async (entry: HiddenRepo) => {
    const ticket = this.#ticket();
    const value = await this.#bridge.setRepoHidden(
      entry.accountId,
      entry.repoId,
      false,
    );
    this.#accept(ticket, value);
    this.#forget(hideNotice(entry.accountId, entry.repoId));
  };

  /**
   * Sets the owners an account watches from the settings (none: all).
   * Rejects with the shell's message, which the dialog shows.
   */
  setOwners = async (accountId: string, owners: string[]) => {
    const ticket = this.#ticket();
    this.#accept(ticket, await this.#bridge.setOwnerFilter(accountId, owners));
  };

  async #restore(repo: Repo) {
    const key = hideNotice(repo.accountId, repo.id);
    if (!this.#undo.has(key)) return;
    await this.setHidden(repo, false);
  }
  #forget(key: string) {
    this.#undo.delete(key);
    this.#host.notices.clear(key);
  }

  /**
   * The answer is newer than any read in flight, so those reads are cancelled
   * and the refresh button is free again. A running mutation keeps its
   * ticket: it accepts its own, fresher answer (as `Starts.release` does).
   */
  #ticket(): number | null {
    if (this.#host.mutating) return null;
    this.#host.refreshing = false;
    return this.#host.viewEpoch.next();
  }
  #accept(ticket: number | null, value: Snapshot) {
    if (ticket === null || this.#host.viewEpoch.current(ticket))
      this.#host.accept(value);
  }

  /** A new account's view starts without the previous one's requests. */
  clear() {
    this.cancelling = new Set();
    this.opening = new Set();
    this.hiding = new Set();
    // „Rückgängig“ belongs to the previous account's view.
    for (const key of [...this.#undo]) this.#forget(key);
  }

  #drop<K extends 'cancelling' | 'opening' | 'hiding'>(
    key: K,
    value: K extends 'cancelling' ? number : string,
  ) {
    const next = new Set<number | string>(this[key]);
    next.delete(value);
    this[key] = next as never;
  }
}
