import type { favorites as German } from '../de/favorites';

/** English texts: favorites. Same shape as the German catalog. */
export const favorites: typeof German = {
  names: {
    open: (name: string) => `Start ${name}…`,
    start: (name: string) => `Start ${name} now`,
    edit: (name: string) => `Edit ${name}`,
    remove: (name: string) => `Remove ${name} from favorites`,
    uncertain: (name: string) => `Start status uncertain: ${name}`,
  },
  tiles: {
    running: 'running',
    edit: 'Edit defaults',
    remove: 'Remove favorite',
    start: 'Start',
  },
  page: {
    noMatch: 'No matching favorites',
    noMatchHint: 'Try a different search term.',
    clearSearch: 'Clear search',
    empty: 'Your frequent starts, one click closer.',
    emptyHint:
      'In the overview, click the heart next to a repository’s workflow to save it here.',
  },
  save: {
    summary: 'Save as favorite',
    name: 'Favorite name',
    placeholder: 'e.g. Windows build',
    help: 'Stores account, repository, workflow, branch or tag and non-secret inputs locally.',
    saving: 'Saving…',
    update: 'Update favorite',
    save: 'Save favorite',
    saveAsNew: 'Save as new favorite',
    incomplete: 'Inputs incomplete',
    nameMissing: 'Name missing',
  },
  notices: {
    saved: (label: string) => `“${label}” saved as a favorite.`,
    removed: 'Favorite removed.',
    removedNamed: (label: string) => `Favorite “${label}” removed.`,
    alreadySaved: 'Favorite is already saved again.',
    restored: (label: string) => `Favorite “${label}” restored.`,
  },
};
