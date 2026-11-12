import type { Locale } from './schema';

export default {
  meta: {
    nativeName: 'Français',
    englishName: 'French',
    langCode: 'fr',
    authorName: 'Lenoch',
    authorGithubUsername: 'Lenochxd',
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
    shortcut: 'Raccourci',
    noShortcut: 'Aucun raccourci sélectionné',
    chooseKeys: 'Choisir les touches',
    recordShortcut: 'Enregistrer un raccourci',
    recording: 'Enregistrement…',
    recordPrompt: 'Appuyez maintenant sur le raccourci. Échap annule l’enregistrement.',
    recorded: 'Raccourci capturé dans ce navigateur.',
    recordTimeout: 'Le délai d’enregistrement a expiré. Réessayez.',
    captureUnavailable:
      'Cette touche ne peut pas être enregistrée ici. Choisissez-la dans le sélecteur.',
    altgraphUnavailable:
      'AltGr ne peut pas être reproduit de manière fiable ici. Choisissez les touches manuellement.',
    captureLimits:
      'Certains raccourcis sont réservés par le navigateur ou le système. L’enregistrement ne vérifie pas que l’ordinateur peut exécuter le raccourci.',
    choosePrimaryKey: 'Touches principales',
    selectedKeys: 'Touches sélectionnées',
    modifiers: 'Modificateurs',
    advancedHelp:
      'Les touches sélectionnées sont pressées en même temps. Les séquences comme Ctrl+K, puis Ctrl+C sont des frappes distinctes et ne sont pas représentées ici.',
    searchKeys: 'Rechercher des touches',
    category: 'Catégorie de touches',
    functionKeys: 'Fonction',
    lettersNumbers: 'Lettres et chiffres',
    navigation: 'Navigation',
    editingSpecial: 'Édition et touches spéciales',
    numpad: 'Pavé numérique',
    hardware: 'Matériel',
    hostUnknown:
      'La prise en charge par l’ordinateur n’est pas vérifiée. Aucune réponse de capacité n’est disponible.',
    hostUnsupported: 'La saisie au clavier n’est pas prise en charge sur cet ordinateur.',
    permissionRequired:
      'Autorisation du bureau requise. Wayland peut la demander via le portail système ; la prise en charge dépend aussi du compositeur et du backend.',
    readyToTry:
      'Prêt à essayer. Les correspondances statiques ne vérifient ni l’autorisation du bureau ni la sortie réelle.',
    keyUnavailable: 'Indisponible sur cet ordinateur ou backend.',
    noResults: 'Aucune touche correspondante.',
    clearSelection: 'Effacer la sélection',
    useShortcut: 'Utiliser le raccourci',
    chooseBetween1And16Keys: 'Choisissez entre 1 et 16 touches.',
    unknownKey: 'Touche inconnue. Choisissez une touche dans le sélecteur.',
    duplicateKeys: 'Supprimez les touches en double ou les alias de modificateurs.',
    unavailableKey:
      'Cette touche est indisponible. Fn est généralement gérée par le firmware du clavier. Utilisez une action média dédiée si elle existe.',
    tooManyModifiers: 'Un raccourci peut utiliser jusqu’à quatre modificateurs.',
    selectPrimary: 'Sélectionnez une touche principale pour utiliser ce raccourci.',
    enableAdvanced:
      'Activez le mode avancé de combinaison simultanée pour utiliser plusieurs touches principales.',
    remove: 'Retirer {key}',
  },
  common: {
    cancel: 'Annuler',
    media: 'Média',
  },
} satisfies Locale;
