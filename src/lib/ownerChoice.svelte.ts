import type { OwnerRow } from './ownerFilter';

const lower = (text: string) => text.toLowerCase();
const same = (a: ReadonlySet<string>, b: ReadonlySet<string>) =>
  a.size === b.size && [...a].every((key) => b.has(key));
const keys = (owners: readonly string[]) => new Set(owners.map(lower));

/**
 * Owners toggled while the settings dialog is open, per account. Toggling
 * only stages; „Speichern“ sets each changed account's filter with `apply`.
 * The last watched owner of an account cannot be released.
 */
export class OwnerChoice {
  /** Watched owners per account (lower case), only where they differ. */
  staged = $state.raw(new Map<string, ReadonlySet<string>>());

  #watched = (row: OwnerRow) =>
    this.staged.get(row.accountId) ?? keys(row.watched);

  has = (row: OwnerRow, owner: string) => this.#watched(row).has(lower(owner));

  /** The only watched owner of its account: releasing it is refused. */
  locked = (row: OwnerRow, owner: string) =>
    this.has(row, owner) && this.#watched(row).size === 1;

  toggle = (row: OwnerRow, owner: string) => {
    if (this.locked(row, owner)) return;
    const watched = new Set(this.#watched(row));
    if (!watched.delete(lower(owner))) watched.add(lower(owner));
    const next = new Map(this.staged);
    if (same(watched, keys(row.watched))) next.delete(row.accountId);
    else next.set(row.accountId, watched);
    this.staged = next;
  };

  #changed = (row: OwnerRow) => {
    const staged = this.staged.get(row.accountId);
    return !!staged && !same(staged, keys(row.watched));
  };

  /** Whether a staged account still differs, so „Speichern“ has work. */
  pendingIn = (rows: readonly OwnerRow[]) => rows.some(this.#changed);

  /**
   * Sets the filter of every changed account, in turn: its watched owners,
   * or none when it watches them all. An account that was set is no longer
   * staged; the first failure rejects and keeps the rest staged.
   */
  apply = async (
    rows: readonly OwnerRow[],
    setOwners: (accountId: string, owners: string[]) => Promise<unknown>,
  ) => {
    for (const row of rows.filter(this.#changed)) {
      const owners = row.owners.filter((owner) => this.has(row, owner));
      await setOwners(
        row.accountId,
        owners.length === row.owners.length ? [] : owners,
      );
      const next = new Map(this.staged);
      next.delete(row.accountId);
      this.staged = next;
    }
  };
}
