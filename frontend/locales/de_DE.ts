import type { Locale } from './schema';

export default {
  meta: {
    nativeName: 'Deutsch',
    englishName: 'German',
    langCode: 'de',
    authorName: 'Dtrieb',
    authorGithubUsername: 'Dtrieb',
  },
  actions: {
    workflow: 'Workflow',
  },
  runtime: {
    runtime: 'Runtime and plugins',
    runtimeStatus: 'Runtime status',
    reloadPlugins: 'Reload plugins',
    runtimeReady: 'Ready',
    runtimeFailed: 'Unavailable',
    runtimeCommands: 'Commands: {count}',
    runtimeNoPlugins: 'No external plugins installed',
    enablePlugin: 'Enable plugin',
    disablePlugin: 'Disable plugin',
    sandboxPlugin: 'Sandbox JavaScript',
    trustedNativePlugin: 'Trusted native process',
  },
  keyboard: {
    shortcut: 'Tastenkürzel',
    noShortcut: 'Kein Tastenkürzel ausgewählt',
    chooseKeys: 'Tasten auswählen',
    recordShortcut: 'Tastenkürzel aufnehmen',
    recording: 'Aufnahme…',
    recordPrompt: 'Drücke jetzt dein Tastenkürzel. Escape bricht die Aufnahme ab.',
    recorded: 'Tastenkürzel in diesem Browser erfasst.',
    recordTimeout: 'Zeitüberschreitung bei der Aufnahme. Versuche es erneut.',
    captureUnavailable:
      'Diese Taste kann hier nicht aufgenommen werden. Wähle sie im Tastenauswahlfeld aus.',
    altgraphUnavailable:
      'AltGr kann hier nicht zuverlässig reproduziert werden. Wähle die Tasten manuell aus.',
    captureLimits:
      'Manche Tastenkürzel sind vom Browser oder Betriebssystem reserviert. Die Aufnahme prüft nicht, ob der Host das Kürzel ausführen kann.',
    choosePrimaryKey: 'Haupttasten',
    selectedKeys: 'Ausgewählte Tasten',
    modifiers: 'Modifikatortasten',
    advancedHelp:
      'Die ausgewählten Tasten werden gleichzeitig gedrückt. Folgen wie Strg+K und danach Strg+C sind getrennte Tastenanschläge und werden hier nicht dargestellt.',
    searchKeys: 'Tasten suchen',
    category: 'Tastenkategorie',
    functionKeys: 'Funktion',
    lettersNumbers: 'Buchstaben und Zahlen',
    navigation: 'Navigation',
    editingSpecial: 'Bearbeitung und Sondertasten',
    numpad: 'Ziffernblock',
    hardware: 'Hardware',
    hostUnknown: 'Host-Unterstützung nicht überprüft. Es liegt keine Fähigkeitsantwort vor.',
    hostUnsupported: 'Tastatureingaben werden auf diesem Host nicht unterstützt.',
    permissionRequired:
      'Desktop-Berechtigung erforderlich. Wayland kann über das Systemportal danach fragen; die Unterstützung hängt weiterhin von Compositor und Backend ab.',
    readyToTry:
      'Bereit zum Testen. Statische Zuordnungen bestätigen weder Desktop-Berechtigungen noch eine tatsächliche Ausgabe.',
    keyUnavailable: 'Auf diesem Host oder Backend nicht verfügbar.',
    noResults: 'Keine passenden Tasten.',
    clearSelection: 'Auswahl löschen',
    useShortcut: 'Tastenkürzel verwenden',
    chooseBetween1And16Keys: 'Wähle zwischen 1 und 16 Tasten.',
    unknownKey: 'Unbekannte Taste. Wähle eine Taste aus der Auswahl.',
    duplicateKeys: 'Entferne doppelte Tasten oder Modifikator-Aliase.',
    unavailableKey:
      'Diese Taste ist nicht verfügbar. Fn wird normalerweise von der Tastaturfirmware verarbeitet. Verwende bei Bedarf eine eigene Medienaktion.',
    tooManyModifiers: 'Ein Tastenkürzel kann bis zu vier Modifikatoren verwenden.',
    selectPrimary: 'Wähle eine Haupttaste aus, um dieses Tastenkürzel zu verwenden.',
    enableAdvanced:
      'Aktiviere den erweiterten Modus für gleichzeitige Kombinationen, um mehrere Haupttasten zu verwenden.',
    remove: '{key} entfernen',
  },
  common: {
    cancel: 'Abbrechen',
    media: 'Medien',
  },
} satisfies Locale;
