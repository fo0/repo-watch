import type { shell as German } from '../de/shell';

/** English texts: shell. Same shape as the German catalog. */
export const shell: typeof German = {
  appBar: {
    home: 'Go to overview',
    refresh: 'Refresh',
    refreshTitle: (demo: boolean) =>
      `Refresh ${demo ? 'demo' : 'GitHub'} status (F5)`,
    settings: 'Settings',
  },
  footer: {
    stamp: (time: string) => `As of ${time}`,
    offline: 'Offline',
    forbidden: 'No access',
    connected: 'Connected',
    demoSimulated: 'demo-dev · simulated',
    disconnected: 'Not connected',
    connection: 'Connection status',
    demoTitle:
      'Demo mode: no real GitHub connection. All data and starts are simulated.',
    liveTitle: 'GitHub connection · token in the OS vault',
    staleDemo: 'Outdated demo data',
    stale: 'Outdated data',
    refreshDemo: 'Refresh demo status',
    refreshLive: 'Refresh GitHub status',
    refreshStamp: (stamp: string) => `Refresh · ${stamp}`,
    refreshing: 'Refreshing…',
    reload: 'Reload',
    loading: 'Loading…',
  },
  connect: {
    title: 'Connect GitHub',
    connected: (login: string) => `Account ${login} connected.`,
    demoConnected: 'demo-dev connected (simulated).',
  },
  closeDialog: 'Close dialog',
  closeNotice: 'Close message',
  banners: {
    update: (version: string) => `Update ${version} available`,
    view: 'View',
    closeHint: 'Close notice',
    demoUnavailable: 'Demo status unavailable.',
    stale: 'GitHub data is out of date. Click Refresh.',
  },
  pages: {
    runs: 'History',
    favorites: 'Favorites',
    repos: 'Repositories',
  },
  sections: {
    runs: 'Running now',
    back: 'Back to overview',
    backTitle: 'Back · Esc',
    maximize: (title: string) => `Maximize ${title}`,
    maximizeTitle: (key: number) => `Maximize · Ctrl+${key}`,
    more: (n: number) => `+${n} more`,
    moreLabel: {
      runs: (n: number) =>
        n === 1 ? 'Show 1 more run' : `Show ${n} more runs`,
      favorites: (n: number) =>
        n === 1 ? 'Show 1 more favorite' : `Show ${n} more favorites`,
      repos: (n: number) =>
        n === 1 ? 'Show 1 more repository' : `Show ${n} more repositories`,
    },
  },
  overview: {
    noMatches: 'No matches',
    noLiveMatches: 'No matching running runs.',
    nothingRunning: 'Nothing running.',
    noFavoriteMatches: 'No matching favorites.',
    noFavorites: 'No favorites yet.',
    noRepoMatches: 'No matching repositories.',
    noRepos: 'No repositories found.',
    startWorkflow: 'Start workflow…',
  },
  loading: {
    title: 'Loading workflows…',
    text: 'Your overview is almost ready.',
    demoFailed: 'Demo not loaded',
    demoFailedText: 'Restart the app to load the local demo data.',
    failed: 'Data could not be loaded',
    failedText: 'Check your connection and reload (F5).',
  },
  setup: {
    pickTitle: 'Choose an active account',
    pickText: 'In the account menu above.',
    pickAction: 'Choose account',
    connectTitle: 'Connect your GitHub account',
    connectText: 'With a token or gh sign-in.',
    connectAction: 'Connect GitHub account',
  },
  account: {
    savedGone: 'The saved account is no longer connected.',
    removed: 'The active account was removed.',
    pickAgain: 'Choose an active account again.',
    pickOther: 'Choose another active account.',
    connectFirst: 'Connect a GitHub account to continue.',
    notSaved:
      'The account choice could not be saved and applies to this session only.',
  },
  refreshed: (demo: boolean) =>
    demo ? 'Demo status refreshed.' : 'GitHub status refreshed.',
  demoReset: 'Demo reset.',
  shortcuts: [
    'Keyboard shortcuts',
    '/ or Ctrl+F – Search',
    '↓ / ↑ – Next / previous result',
    'F5 or Ctrl+R – Refresh',
    'Ctrl+1, 2, 3 – Maximize Repositories, History, Favorites; again to go back',
    'Ctrl+, – Settings',
    'Esc – Clear or leave search, close menu or dialog, back to overview',
  ],
};
