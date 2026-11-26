import type { settings as German } from '../de/settings';

/** English texts: settings. Same shape as the German catalog. */
export const settings: typeof German = {
  dialog: {
    title: 'Settings',
    tabsLabel: 'Settings sections',
    noChanges: 'No changes',
    updateAvailable: 'Update available',
  },
  tabs: {
    accounts: 'Accounts',
    app: 'App',
    updates: 'Updates',
  },
  language: {
    heading: 'Language',
    system: 'System',
    systemDetected: (language: string) => `System language: ${language}`,
    saveFailed:
      'The language could not be saved and applies until the next start.',
  },
  theme: {
    heading: 'Appearance',
    system: 'System',
    light: 'Light',
    dark: 'Dark',
    saveFailed:
      'The appearance could not be saved and applies until the next start.',
  },
  refresh: {
    heading: 'Refresh',
    invalid: 'Refresh invalid',
    error: 'Refresh must be between 15 s and 60 min.',
    recommendedHint: 'Recommended: 1 min · running runs every 15 s',
    recommended: 'Recommended',
    custom: 'Custom value',
  },
  owners: {
    heading: 'Organizations',
    account: (login: string) => `Organizations of ${login}`,
    watched: (owner: string) => `${owner}: watched`,
    ignored: (owner: string) => `${owner}: hidden`,
    last: 'At least one stays watched',
    notFound: 'Account not found.',
  },
  hidden: {
    heading: 'Hidden repositories',
    show: 'Show',
    showRepo: (repo: string) => `Show ${repo}`,
    none: 'None',
    notFound: 'Repository not found.',
  },
  log: {
    heading: 'Log',
    info: 'Info',
    debug: 'Debug',
    openFolder: 'Open folder',
  },
  startup: {
    heading: 'Autostart',
    demo: 'The browser demo does not change Windows settings.',
    withWindows: 'Start with Windows',
    withWindowsHint: 'Starts in the tray after sign-in.',
    checkStatus: 'Check status',
    openWindows: 'Open Windows autostart',
    removeEntry: 'Remove autostart entry',
    reading: 'Reading status…',
    unknown: 'Status unknown.',
    disabledByWindows:
      'Disabled in Windows – turn it on under Settings → Apps → Startup.',
    unknownApproval:
      'Windows approval unknown – check under Settings → Apps → Startup.',
    conflict: 'Another entry uses this autostart. Nothing is overwritten.',
    unsupported: 'Windows autostart is only available on Windows.',
    notApplied: 'Windows autostart was not applied.',
  },
};
