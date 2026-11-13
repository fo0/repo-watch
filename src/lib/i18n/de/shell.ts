/** German texts: shell. Every key needs its English twin in ../en/shell.ts. */
export const shell = {
  appBar: {
    home: 'Zur Übersicht',
    refresh: 'Aktualisieren',
    refreshTitle: (demo: boolean) =>
      `${demo ? 'Demo' : 'GitHub'}-Status aktualisieren (F5)`,
    settings: 'Einstellungen',
  },
  footer: {
    stamp: (time: string) => `Stand ${time}`,
    offline: 'Offline',
    forbidden: 'Kein Zugriff',
    connected: 'Verbunden',
    demoSimulated: 'demo-dev · simuliert',
    disconnected: 'Nicht verbunden',
    connection: 'Verbindungsstatus',
    demoTitle:
      'Demo-Modus: keine echte GitHub-Verbindung. Alle Daten und Starts sind simuliert.',
    liveTitle: 'GitHub-Verbindung · Token im Betriebssystem-Tresor',
    staleDemo: 'Veralteter Demo-Stand',
    stale: 'Veralteter Stand',
    refreshDemo: 'Demo-Status aktualisieren',
    refreshLive: 'GitHub-Status aktualisieren',
    refreshStamp: (stamp: string) => `Aktualisieren · ${stamp}`,
    refreshing: 'Aktualisiert …',
    reload: 'Erneut laden',
    loading: 'Lädt …',
  },
  connect: {
    title: 'GitHub verbinden',
    connected: (login: string) => `Konto ${login} verbunden.`,
    demoConnected: 'demo-dev simuliert verbunden.',
  },
  closeDialog: 'Dialog schließen',
  closeNotice: 'Meldung schließen',
  banners: {
    update: (version: string) => `Update ${version} verfügbar`,
    view: 'Ansehen',
    closeHint: 'Hinweis schließen',
    demoUnavailable: 'Demo-Status nicht verfügbar.',
    stale: 'GitHub-Daten sind nicht aktuell. Klicke auf Aktualisieren.',
  },
  /** Titles of the maximized pages; the runs section reads „Läuft gerade“. */
  pages: {
    runs: 'Verlauf',
    favorites: 'Favoriten',
    repos: 'Repositories',
  },
  sections: {
    runs: 'Läuft gerade',
    back: 'Zurück zur Übersicht',
    backTitle: 'Zurück · Esc',
    maximize: (title: string) => `${title} maximieren`,
    maximizeTitle: (key: number) => `Maximieren · Strg+${key}`,
    /** „+3 weitere“; the accessible name names what. */
    more: (n: number) => `+${n} weitere`,
    moreLabel: {
      runs: (n: number) => `+${n} weitere Runs anzeigen`,
      favorites: (n: number) => `+${n} weitere Favoriten anzeigen`,
      repos: (n: number) => `+${n} weitere Repositories anzeigen`,
    },
  },
  overview: {
    noMatches: 'Keine Treffer',
    noLiveMatches: 'Keine passenden laufenden Runs.',
    nothingRunning: 'Gerade läuft nichts.',
    noFavoriteMatches: 'Keine passenden Favoriten.',
    noFavorites: 'Noch keine Favoriten.',
    noRepoMatches: 'Keine passenden Repositories.',
    noRepos: 'Keine Repositories gefunden.',
    startWorkflow: 'Workflow starten …',
  },
  loading: {
    title: 'Workflows werden geladen …',
    text: 'Dein Überblick ist gleich bereit.',
    demoFailed: 'Demo nicht geladen',
    demoFailedText:
      'Starte die Anwendung neu, um den lokalen Demo-Stand zu laden.',
    failed: 'Daten konnten nicht geladen werden',
    failedText: 'Prüfe deine Verbindung und lade erneut (F5).',
  },
  /** The missing setup step every page names alike. */
  setup: {
    pickTitle: 'Wähle ein aktives Konto',
    pickText: 'Im Kontomenü oben.',
    pickAction: 'Konto wählen',
    connectTitle: 'Verbinde dein GitHub-Konto',
    connectText: 'Per Token oder gh-Anmeldung.',
    connectAction: 'GitHub-Konto verbinden',
  },
  /** Why the active account has to be chosen (again). */
  account: {
    savedGone: 'Das gespeicherte Konto ist nicht mehr verbunden.',
    removed: 'Das aktive Konto wurde entfernt.',
    pickAgain: 'Wähle ein aktives Konto neu aus.',
    pickOther: 'Wähle ein anderes aktives Konto aus.',
    connectFirst: 'Verbinde ein GitHub-Konto, um fortzufahren.',
    notSaved:
      'Die Kontoauswahl konnte nicht gespeichert werden und gilt nur für diese Sitzung.',
  },
  refreshed: (demo: boolean): string =>
    demo ? 'Demo-Status aktualisiert.' : 'GitHub-Status aktualisiert.',
  demoReset: 'Demo zurückgesetzt.',
  /** The „Tastenkürzel“ tooltip, one shortcut per line. */
  shortcuts: [
    'Tastenkürzel',
    '/ oder Strg+F – Suche',
    '↓ / ↑ – nächster / vorheriger Treffer',
    'F5 oder Strg+R – Aktualisieren',
    'Strg+1, 2, 3 – Repositories, Verlauf, Favoriten maximieren; nochmal zurück',
    'Strg+, – Einstellungen',
    'Esc – Suche leeren oder verlassen, Menü oder Dialog schließen, zurück zur Übersicht',
  ],
};
