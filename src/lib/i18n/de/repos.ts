/** German texts: repos. Every key needs its English twin in ../en/repos.ts. */
export const repos = {
  lastPush: 'Letzter Push',
  /** Prefix of the relative push time („Push vor 2 Std.“). */
  pushPrefix: 'Push ',
  openOnGitHub: (name: string) => `${name} auf GitHub öffnen`,
  demoRepoNoLink: 'Demo-Repositories haben keinen GitHub-Link',
  repoNoLink: 'Für dieses Repository gibt es keinen gültigen GitHub-Link',
  info: {
    defaultBranch: 'Standardbranch',
    defaultBranchTitle: (branch: string) => `Standardbranch: ${branch}`,
  },
  menu: {
    actionsFor: (name: string) => `Aktionen für ${name}`,
    actionsTitle: 'Aktionen · Rechtsklick',
    openLastRun: 'Letzten Run auf GitHub öffnen',
    demoRunNoLink: 'Demo-Runs haben keinen GitHub-Link',
    runNoLink: 'Für diesen Run gibt es keinen gültigen GitHub-Link',
    showRuns: 'Runs im Verlauf anzeigen',
    copyLink: 'Link kopieren',
    copyFailed: 'Link konnte nicht kopiert werden.',
    show: 'Einblenden',
    showTitle: 'Wieder in Repositories und Verlauf zeigen',
    hide: 'Ausblenden',
    hideTitle:
      'Nicht mehr in Repositories und Verlauf zeigen · Einstellungen → App',
  },
  page: {
    noMatches: 'Keine passenden Repositories',
    none: 'Keine Repositories gefunden.',
    manageAccess: 'Repository-Freigabe auf GitHub ändern',
    refreshAccess: 'Repository-Liste aktualisieren',
  },
  /** The search field per page: accessible name and placeholder. */
  search: {
    overview: {
      label: 'Übersicht durchsuchen',
      placeholder: 'Workflow, Repository oder Branch suchen …',
    },
    runs: {
      label: 'Verlauf durchsuchen',
      placeholder: 'Repository, Workflow oder Branch suchen …',
    },
    favorites: {
      label: 'Favoriten durchsuchen',
      placeholder: 'Favorit oder Repository suchen …',
    },
    repos: {
      label: 'Repositories durchsuchen',
      placeholder: 'Repository oder Workflow suchen …',
    },
  },
  list: {
    repoUncertain: 'Startstatus ungeklärt in diesem Repository',
    uncertain: 'Startstatus ungeklärt',
    lastRun: 'Letzter Run',
    workflows: (count: number) =>
      count === 1 ? '1 Workflow' : `${count} Workflows`,
    noRunsYet: 'Noch keine Runs',
    readOnly: 'Nur Lesen',
    noAccess: 'Kein Zugriff',
    loadingWorkflows: 'Workflows werden geladen …',
    noManualStart: 'Kein manueller Start',
    running: 'läuft',
    removeFavorite: (workflow: string) => `${workflow} aus Favoriten entfernen`,
    saveFavorite: (workflow: string) => `${workflow} als Favorit speichern`,
    favoriteUnavailable:
      'Favoriten brauchen einen manuell startbaren Workflow mit Schreibzugriff',
    removeFavoriteTitle: (label: string) => `Favorit „${label}“ entfernen`,
    saveFavoriteTitle: 'Mit Standardwerten als Favorit speichern',
    startLabel: (workflow: string) => `${workflow} starten…`,
    startTitle: 'Branch oder Tag und Eingaben wählen',
    start: 'Starten…',
    noMatchingWorkflows: 'Keine passenden Workflows.',
  },
};
