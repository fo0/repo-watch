// Owners of an account the user watches (#108): matching, the rows of the
// setting, and the browser demo's stored copy of the native setting.
import type { PreferenceStorage } from './bridgeTypes';
import { m } from './i18n/index.svelte';
import type { Account, OwnerFilter, Repo, Snapshot } from './types';

/** The demo's owner filters; the native shell keeps its own. */
export const OWNERS_KEY = 'repowatch.demo.owners.v1';

const lower = (text: string) => text.toLowerCase();
const byName = (a: string, b: string) =>
  lower(a) < lower(b) ? -1 : lower(a) > lower(b) ? 1 : 0;

/** `owners` without case-insensitive repeats, the first spelling kept. */
function distinct(owners: readonly string[]): string[] {
  const seen = new Set<string>();
  return owners.filter((owner) => {
    const key = lower(owner);
    if (!owner || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

/**
 * Whether the account's owner filter leaves this repository out. No filter
 * (or an empty one) watches every owner; owners match case-insensitively.
 * A left-out repository behaves like a hidden one (ADR-0008).
 */
export function isFilteredOut(
  filters: readonly OwnerFilter[] | undefined,
  repo: Pick<Repo, 'accountId' | 'owner'>,
) {
  const filter = filters?.find((entry) => entry.accountId === repo.accountId);
  return (
    !!filter?.owners.length &&
    !filter.owners.some((owner) => lower(owner) === lower(repo.owner))
  );
}

/** One account's owners in Settings → App, and the ones watched now. */
export interface OwnerRow {
  accountId: string;
  login: string;
  /** Sorted case-insensitively, the account's own login first. */
  owners: string[];
  /** Every owner without a filter. */
  watched: string[];
}

/**
 * The accounts that have a choice: more than one owner among their
 * repositories and their stored filter.
 */
export function ownerRows(snapshot: Snapshot | null): OwnerRow[] {
  if (!snapshot) return [];
  return snapshot.accounts.flatMap((account: Account) => {
    const filter = snapshot.ownerFilters?.find(
      (entry) => entry.accountId === account.id,
    );
    const owners = distinct([
      ...snapshot.repos
        .filter((repo) => repo.accountId === account.id)
        .map((repo) => repo.owner),
      ...(filter?.owners ?? []),
    ]).sort(
      (a, b) =>
        Number(lower(b) === lower(account.login)) -
          Number(lower(a) === lower(account.login)) || byName(a, b),
    );
    if (owners.length < 2) return [];
    const watched = owners.filter(
      (owner) =>
        !isFilteredOut(snapshot.ownerFilters, {
          accountId: account.id,
          owner,
        }),
    );
    return [{ accountId: account.id, login: account.login, owners, watched }];
  });
}

function valid(entry: unknown): entry is OwnerFilter {
  if (!entry || typeof entry !== 'object') return false;
  const { accountId, owners } = entry as Record<string, unknown>;
  return (
    typeof accountId === 'string' &&
    Array.isArray(owners) &&
    owners.length > 0 &&
    owners.every((owner) => typeof owner === 'string' && owner !== '')
  );
}

/** The stored filters; corrupt or unavailable storage filters nothing. */
function readOwners(storage?: PreferenceStorage): OwnerFilter[] {
  try {
    const stored: unknown = JSON.parse(storage?.getItem(OWNERS_KEY) ?? '[]');
    return Array.isArray(stored) ? stored.filter(valid) : [];
  } catch {
    return [];
  }
}

function writeOwners(
  storage: PreferenceStorage | undefined,
  filters: readonly OwnerFilter[],
) {
  try {
    if (filters.length) storage?.setItem(OWNERS_KEY, JSON.stringify(filters));
    else storage?.removeItem(OWNERS_KEY);
  } catch {
    /* The filters still apply for this session. */
  }
}

/**
 * The filters after one change, as the native `set_owner_filter` makes them:
 * a known account only; no owners removes its filter.
 */
export function changeOwners(
  accounts: readonly Account[],
  filters: readonly OwnerFilter[],
  accountId: string,
  owners: readonly string[],
): OwnerFilter[] {
  if (!accounts.some((account) => account.id === accountId))
    throw new Error(m().settings.owners.notFound);
  const rest = filters.filter((entry) => entry.accountId !== accountId);
  const kept = distinct(owners);
  return kept.length ? [...rest, { accountId, owners: kept }] : rest;
}

/** The browser demo's owner filters, kept in `OWNERS_KEY`. */
export function demoOwners(storage?: PreferenceStorage) {
  let list = readOwners(storage);
  return {
    get list(): OwnerFilter[] {
      return list;
    },
    /**
     * Applies one change and returns the repositories under `select` (the
     * automatic rule). Repositories opened from the search stay selected,
     * unless the new filter leaves them out.
     */
    change(
      accounts: readonly Account[],
      repos: readonly Repo[],
      accountId: string,
      owners: readonly string[],
      select: (filters: readonly OwnerFilter[]) => Repo[],
    ): Repo[] {
      const auto = select(list);
      const next = changeOwners(accounts, list, accountId, owners);
      const opened = new Set(
        repos
          .filter((repo, index) => repo.selected && !auto[index]?.selected)
          .filter((repo) => !isFilteredOut(next, repo))
          .map((repo) => repo.id),
      );
      list = next;
      writeOwners(storage, list);
      return select(list).map((repo) =>
        opened.has(repo.id) ? { ...repo, selected: true } : repo,
      );
    },
    reset() {
      list = [];
      writeOwners(storage, list);
    },
  };
}
