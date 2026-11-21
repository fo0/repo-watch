import type { updates as German } from '../de/updates';

/** English texts: updates. Same shape as the German catalog. */
export const updates: typeof German = {
  label: 'App update',
  version: (version: string) => `Version ${version}`,
  lastChecked: (time: string) => `Last checked ${time}`,
  check: 'Check for updates',
  demo: 'Not available in the demo.',
  current: 'Up to date.',
  available: (version: string) => `Version ${version} available`,
  notes: 'Changes',
  downloading: 'Downloading…',
  verifying: 'Verifying…',
  installing: 'Installing – RepoWatch restarts…',
  install: 'Install',
  progress: 'Installation progress',
};
