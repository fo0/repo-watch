import type { RunTone } from '../../runs';

/** German texts: runs. Every key needs its English twin in ../en/runs.ts. */
export const runs = {
  status: {
    in_progress: 'Läuft',
    queued: 'In Warteschlange',
    waiting: 'Wartet',
    success: 'Erfolgreich',
    failure: 'Fehlgeschlagen',
    cancelled: 'Abgebrochen',
    stuck: 'Hängt',
  },
  /** A repository row with several open runs: „2 aktiv“. */
  active: (n: number) => `${n} aktiv`,
  /** Run strip summary, e.g. „Letzte 8 Runs: 6 erfolgreich, 1 läuft“. */
  strip: {
    none: 'Noch keine Runs',
    last: (word: string) => `Letzter Run: ${word}`,
    lastN: (n: number, parts: string) => `Letzte ${n} Runs: ${parts}`,
    /** Singular and plural word per tone, in the order the summary names them. */
    tones: {
      ok: ['erfolgreich', 'erfolgreich'],
      fail: ['fehlgeschlagen', 'fehlgeschlagen'],
      cancel: ['abgebrochen', 'abgebrochen'],
      run: ['läuft', 'laufen'],
      wait: ['wartet', 'warten'],
      queue: ['in Warteschlange', 'in Warteschlange'],
      stuck: ['hängt', 'hängen'],
    } as Record<RunTone, [one: string, many: string]>,
  },
  activity: {
    filters: 'Aktivitätsfilter',
    all: 'Alle',
    active: 'Aktiv',
    failed: 'Fehlgeschlagen',
    recent: 'Abgeschlossen',
    now: 'Jetzt',
    done: 'Abgeschlossen',
    note: 'Hinweis zum Verlauf',
    noteLive: 'Offene Runs werden automatisch aktualisiert.',
    noteLimit: (n: number) =>
      `Bis zu ${n} abgeschlossene Runs bleiben im Verlauf.`,
    noMatches: 'Keine passenden Runs',
    quiet: 'Hier ist gerade Ruhe',
    noneForFilter: 'Für diesen Filter gibt es keine Runs.',
    showAll: 'Alle anzeigen',
  },
  cancel: {
    run: 'Run abbrechen',
    pending: 'Wird abgebrochen',
    pendingTitle: 'Wird abgebrochen …',
    confirmTitle: 'Zum Abbrechen erneut klicken',
    armed: 'Abbrechen?',
    armedLong: 'Wirklich abbrechen?',
    label: (run: string) => `${run} abbrechen`,
    armedLabel: (run: string) => `${run} wirklich abbrechen?`,
    pendingLabel: (run: string) => `${run} wird abgebrochen`,
  },
  row: {
    since: (elapsed: string) => `seit ${elapsed}`,
    account: 'Konto',
    updated: 'Aktualisiert',
    status: 'Status',
    /** Tooltip on a stuck run's status: why it stays and what to do. */
    stuckHint:
      'Seit über 24 h in der Warteschlange – GitHub startet den Run nicht und nimmt keinen Abbruch an. Auf GitHub löschen.',
    open: 'Auf GitHub öffnen',
    copy: 'Link kopieren',
    /** Opens the start dialog with the run's workflow and branch. */
    startAgain: 'Erneut starten …',
    startAgainTitle: 'Startdialog mit diesem Workflow und Branch öffnen',
    noLinkDemo: 'Demo-Runs haben keinen GitHub-Link',
    noLink: 'Für diesen Run gibt es keinen gültigen GitHub-Link',
    copyFailed:
      'Link konnte nicht kopiert werden. Öffne den Run auf GitHub und kopiere ihn dort.',
  },
};
