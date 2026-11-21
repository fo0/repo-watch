/** German texts: updates. Every key needs its English twin in ../en/updates.ts. */
export const updates = {
  label: 'App-Update',
  version: (version: string) => `Version ${version}`,
  lastChecked: (time: string) => `Zuletzt geprüft ${time}`,
  check: 'Nach Updates suchen',
  demo: 'In der Demo nicht verfügbar.',
  current: 'Aktuell.',
  available: (version: string) => `Version ${version} verfügbar`,
  notes: 'Änderungen',
  downloading: 'Lädt …',
  verifying: 'Wird geprüft …',
  installing: 'Wird installiert – RepoWatch startet neu …',
  install: 'Installieren',
  /** Accessible name of the progress area that takes focus from „Installieren“. */
  progress: 'Installationsfortschritt',
};
