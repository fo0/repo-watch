import type { repos as German } from '../de/repos';

/** English texts: repos. Same shape as the German catalog. */
export const repos: typeof German = {
  lastPush: 'Last push',
  pushPrefix: 'Push ',
  openOnGitHub: (name: string) => `Open ${name} on GitHub`,
  demoRepoNoLink: 'Demo repositories have no GitHub link',
  repoNoLink: 'This repository has no valid GitHub link',
  info: {
    defaultBranch: 'Default branch',
    defaultBranchTitle: (branch: string) => `Default branch: ${branch}`,
  },
  menu: {
    actionsFor: (name: string) => `Actions for ${name}`,
    actionsTitle: 'Actions · right-click',
    openLastRun: 'Open last run on GitHub',
    demoRunNoLink: 'Demo runs have no GitHub link',
    runNoLink: 'This run has no valid GitHub link',
    showRuns: 'Show runs in History',
    copyLink: 'Copy link',
    copyFailed: 'Could not copy the link.',
    show: 'Show',
    showTitle: 'Show again in Repositories and History',
    hide: 'Hide',
    hideTitle: 'Stop showing in Repositories and History · Settings → App',
  },
  page: {
    noMatches: 'No matching repositories',
    none: 'No repositories found.',
    manageAccess: 'Change repository access on GitHub',
    refreshAccess: 'Refresh repository list',
  },
  search: {
    overview: {
      label: 'Search overview',
      placeholder: 'Search workflow, repository or branch…',
    },
    runs: {
      label: 'Search history',
      placeholder: 'Search repository, workflow or branch…',
    },
    favorites: {
      label: 'Search favorites',
      placeholder: 'Search favorite or repository…',
    },
    repos: {
      label: 'Search repositories',
      placeholder: 'Search repository or workflow…',
    },
  },
  list: {
    repoUncertain: 'Start status uncertain in this repository',
    uncertain: 'Start status uncertain',
    lastRun: 'Last run',
    workflows: (count: number) =>
      count === 1 ? '1 workflow' : `${count} workflows`,
    noRunsYet: 'No runs yet',
    readOnly: 'Read only',
    noAccess: 'No access',
    loadingWorkflows: 'Loading workflows…',
    noManualStart: 'No manual start',
    running: 'running',
    removeFavorite: (workflow: string) => `Remove ${workflow} from favorites`,
    saveFavorite: (workflow: string) => `Save ${workflow} as favorite`,
    favoriteUnavailable:
      'Favorites need a manually startable workflow with write access',
    removeFavoriteTitle: (label: string) => `Remove favorite “${label}”`,
    saveFavoriteTitle: 'Save as favorite with default values',
    startLabel: (workflow: string) => `Start ${workflow}…`,
    startTitle: 'Choose branch or tag and inputs',
    start: 'Start…',
    noMatchingWorkflows: 'No matching workflows.',
  },
};
