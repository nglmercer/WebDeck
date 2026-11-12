import type { Locale } from './schema';

export default {
  meta: {
    nativeName: 'Polski',
    englishName: 'Polish',
    langCode: 'pl',
    authorName: 'rini',
    authorGithubUsername: '11rini',
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
    shortcut: 'Skrót klawiszowy',
    noShortcut: 'Nie wybrano skrótu',
    chooseKeys: 'Wybierz klawisze',
    recordShortcut: 'Nagraj skrót',
    recording: 'Nagrywanie…',
    recordPrompt: 'Naciśnij teraz skrót. Escape anuluje nagrywanie.',
    recorded: 'Skrót przechwycono w tej przeglądarce.',
    recordTimeout: 'Przekroczono limit czasu nagrywania. Spróbuj ponownie.',
    captureUnavailable: 'Nie można tu nagrać tego klawisza. Wybierz go z listy klawiszy.',
    altgraphUnavailable: 'Nie można tu niezawodnie odtworzyć AltGr. Wybierz klawisze ręcznie.',
    captureLimits:
      'Niektóre skróty są zarezerwowane przez przeglądarkę lub system operacyjny. Nagranie nie potwierdza, że host może wykonać skrót.',
    choosePrimaryKey: 'Klawisze główne',
    selectedKeys: 'Wybrane klawisze',
    modifiers: 'Klawisze modyfikujące',
    advancedHelp:
      'Wybrane klawisze są naciskane jednocześnie. Sekwencje, takie jak Ctrl+K, a potem Ctrl+C, to osobne naciśnięcia i nie są tu przedstawiane.',
    searchKeys: 'Szukaj klawiszy',
    category: 'Kategoria klawiszy',
    lettersNumbers: 'Litery i cyfry',
    navigation: 'Nawigacja',
    editingSpecial: 'Edycyjne i specjalne',
    numpad: 'Klawiatura numeryczna',
    hardware: 'Sprzęt',
    hostUnknown: 'Nie zweryfikowano obsługi hosta. Brak odpowiedzi o możliwościach.',
    hostUnsupported: 'Ten host nie obsługuje wprowadzania z klawiatury.',
    permissionRequired:
      'Wymagane uprawnienie pulpitu. Wayland może poprosić o nie przez portal systemowy; obsługa zależy też od kompozytora i backendu.',
    readyToTry:
      'Można spróbować. Statyczne mapowania nie potwierdzają uprawnień pulpitu ani rzeczywistego działania.',
    keyUnavailable: 'Niedostępny na tym hoście lub w tym backendzie.',
    noResults: 'Brak pasujących klawiszy.',
    clearSelection: 'Wyczyść wybór',
    useShortcut: 'Użyj skrótu',
    chooseBetween1And16Keys: 'Wybierz od 1 do 16 klawiszy.',
    unknownKey: 'Nieznany klawisz. Wybierz klawisz z listy.',
    duplicateKeys: 'Usuń powielone klawisze lub aliasy modyfikatorów.',
    unavailableKey:
      'Ten klawisz jest niedostępny. Fn jest zwykle obsługiwany przez firmware klawiatury. Użyj dedykowanej akcji multimedialnej, jeśli jest dostępna.',
    tooManyModifiers: 'Skrót może używać maksymalnie czterech modyfikatorów.',
    selectPrimary: 'Wybierz jeden klawisz główny, aby użyć tego skrótu.',
    enableAdvanced:
      'Włącz zaawansowany tryb kombinacji jednoczesnej, aby użyć wielu klawiszy głównych.',
    remove: 'Usuń {key}',
  },
  common: {
    cancel: 'Anuluj',
    media: 'Multimedia',
  },
} satisfies Locale;
