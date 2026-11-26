/** German texts: settings. Every key needs its English twin in ../en/settings.ts. */
export const settings = {
  dialog: {
    title: 'Einstellungen',
    tabsLabel: 'Einstellungsbereiche',
    noChanges: 'Keine Änderungen',
    /** Tab badge of the updates tab. */
    updateAvailable: 'Update verfügbar',
  },
  tabs: {
    accounts: 'Konten',
    app: 'App',
    updates: 'Updates',
  },
  language: {
    heading: 'Sprache',
    system: 'System',
    /** Tooltip of „System“: which language the system setting gives. */
    systemDetected: (language: string) => `Systemsprache: ${language}`,
    saveFailed:
      'Sprache ließ sich nicht speichern und gilt bis zum nächsten Start.',
  },
  theme: {
    heading: 'Darstellung',
    system: 'System',
    light: 'Hell',
    dark: 'Dunkel',
    saveFailed:
      'Darstellung ließ sich nicht speichern und gilt bis zum nächsten Start.',
  },
  refresh: {
    heading: 'Aktualisierung',
    /** Why „Speichern“ is off: the own value is out of range. */
    invalid: 'Aktualisierung ungültig',
    error: 'Aktualisierung muss zwischen 15 s und 60 min liegen.',
    recommendedHint: 'Empfohlen: 1 min · laufende Runs alle 15 s',
    recommended: 'Empfohlen',
    custom: 'Eigener Wert',
  },
  owners: {
    heading: 'Organisationen',
    /** Group label of one account's owners. */
    account: (login: string) => `Organisationen von ${login}`,
    /** Tooltip of an owner toggle. */
    watched: (owner: string) => `${owner}: beobachtet`,
    ignored: (owner: string) => `${owner}: ausgeblendet`,
    /** Tooltip of the last watched owner, which cannot be released. */
    last: 'Mindestens eine bleibt beobachtet',
    notFound: 'Konto nicht gefunden.',
  },
  hidden: {
    heading: 'Ausgeblendete Repositories',
    show: 'Einblenden',
    showRepo: (repo: string) => `${repo} einblenden`,
    none: 'Keine',
    notFound: 'Repository nicht gefunden.',
  },
  log: {
    heading: 'Protokoll',
    info: 'Info',
    debug: 'Debug',
    /** Immediate action: shows the folder of the log file. */
    openFolder: 'Ordner öffnen',
  },
  startup: {
    heading: 'Autostart',
    demo: 'Die Browser-Demo ändert keine Windows-Einstellungen.',
    withWindows: 'Mit Windows starten',
    withWindowsHint: 'Startet nach der Anmeldung im Tray.',
    checkStatus: 'Status prüfen',
    openWindows: 'Windows-Autostart öffnen',
    removeEntry: 'Autostarteintrag entfernen',
    reading: 'Status wird gelesen …',
    unknown: 'Status unbekannt.',
    disabledByWindows:
      'In Windows deaktiviert – unter Einstellungen → Apps → Autostart einschalten.',
    unknownApproval:
      'Windows-Freigabe unbekannt – unter Einstellungen → Apps → Autostart prüfen.',
    conflict:
      'Ein anderer Eintrag nutzt diesen Autostart. Es wird nichts überschrieben.',
    unsupported: 'Windows-Autostart ist nur unter Windows verfügbar.',
    notApplied: 'Windows-Autostart wurde nicht übernommen.',
  },
};
