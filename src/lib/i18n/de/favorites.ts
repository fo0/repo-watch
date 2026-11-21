/** German texts: favorites. Every key needs its English twin in ../en/favorites.ts. */
export const favorites = {
  /**
   * Accessible names of a favorite's controls; `name` is „repository ·
   * favorite“. „…“ marks a control that opens the start dialog, „jetzt“ the
   * one that sends at once.
   */
  names: {
    open: (name: string) => `${name} starten…`,
    start: (name: string) => `${name} jetzt starten`,
    edit: (name: string) => `${name} bearbeiten`,
    remove: (name: string) => `${name} aus Favoriten entfernen`,
    uncertain: (name: string) => `Startstatus ungeklärt: ${name}`,
  },
  tiles: {
    running: 'läuft',
    edit: 'Standardwerte bearbeiten',
    remove: 'Favorit entfernen',
    start: 'Starten',
  },
  page: {
    noMatch: 'Keine passenden Favoriten',
    noMatchHint: 'Versuche einen anderen Suchbegriff.',
    clearSearch: 'Suche leeren',
    empty: 'Deine häufigen Starts, einen Klick näher.',
    emptyHint:
      'Klicke in der Übersicht beim Workflow eines Repositorys auf das Herz, um ihn hier zu speichern.',
  },
  save: {
    summary: 'Als Favorit speichern',
    name: 'Name des Favoriten',
    placeholder: 'z. B. Windows build',
    help: 'Speichert Konto, Repository, Workflow, Branch oder Tag und nicht geheime Eingaben lokal.',
    saving: 'Speichert …',
    update: 'Favorit aktualisieren',
    save: 'Favorit speichern',
    saveAsNew: 'Als neuen Favoriten speichern',
    /** Tooltips of the off save buttons. */
    incomplete: 'Eingaben unvollständig',
    nameMissing: 'Name fehlt',
  },
  notices: {
    saved: (label: string) => `„${label}“ als Favorit gespeichert.`,
    removed: 'Favorit entfernt.',
    removedNamed: (label: string) => `Favorit „${label}“ entfernt.`,
    alreadySaved: 'Favorit ist bereits wieder gespeichert.',
    restored: (label: string) => `Favorit „${label}“ wiederhergestellt.`,
  },
};
