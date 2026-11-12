import type { Locale } from './schema';

export default {
  meta: {
    nativeName: 'Русский',
    englishName: 'Russian',
    langCode: 'ru',
    authorName: 'BigFloppa9,Asuma228',
    authorGithubUsername: 'BigFloppa9,Asuma228',
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
    shortcut: 'Сочетание клавиш',
    noShortcut: 'Сочетание не выбрано',
    chooseKeys: 'Выбрать клавиши',
    recordShortcut: 'Записать сочетание',
    recording: 'Запись…',
    recordPrompt: 'Нажмите сочетание клавиш. Escape отменяет запись.',
    recorded: 'Сочетание перехвачено в этом браузере.',
    recordTimeout: 'Время записи истекло. Попробуйте ещё раз.',
    captureUnavailable: 'Эту клавишу нельзя записать здесь. Выберите её в списке клавиш.',
    altgraphUnavailable: 'Здесь нельзя надёжно воспроизвести AltGr. Выберите клавиши вручную.',
    captureLimits:
      'Некоторые сочетания зарезервированы браузером или операционной системой. Запись не проверяет, сможет ли хост выполнить сочетание.',
    choosePrimaryKey: 'Основные клавиши',
    selectedKeys: 'Выбранные клавиши',
    modifiers: 'Модификаторы',
    advancedHelp:
      'Выбранные клавиши нажимаются одновременно. Последовательности вроде Ctrl+K, затем Ctrl+C состоят из отдельных нажатий и здесь не задаются.',
    searchKeys: 'Поиск клавиш',
    category: 'Категория клавиш',
    functionKeys: 'Функциональные',
    lettersNumbers: 'Буквы и цифры',
    navigation: 'Навигация',
    editingSpecial: 'Редактирование и специальные',
    numpad: 'Цифровой блок',
    hardware: 'Оборудование',
    hostUnknown: 'Поддержка хоста не проверена. Ответ о возможностях отсутствует.',
    hostUnsupported: 'Ввод с клавиатуры не поддерживается на этом хосте.',
    permissionRequired:
      'Требуется разрешение рабочего стола. Wayland может запросить его через системный портал; поддержка также зависит от композитора и backend.',
    readyToTry:
      'Можно попробовать. Статические сопоставления не проверяют разрешение рабочего стола и фактический ввод.',
    keyUnavailable: 'Недоступно на этом хосте или backend.',
    noResults: 'Подходящие клавиши не найдены.',
    clearSelection: 'Очистить выбор',
    useShortcut: 'Использовать сочетание',
    chooseBetween1And16Keys: 'Выберите от 1 до 16 клавиш.',
    unknownKey: 'Неизвестная клавиша. Выберите клавишу из списка.',
    duplicateKeys: 'Удалите дублирующиеся клавиши или псевдонимы модификаторов.',
    unavailableKey:
      'Эта клавиша недоступна. Fn обычно обрабатывается прошивкой клавиатуры. При наличии используйте отдельное мультимедийное действие.',
    tooManyModifiers: 'Сочетание может содержать не более четырёх модификаторов.',
    selectPrimary: 'Выберите основную клавишу для этого сочетания.',
    enableAdvanced:
      'Включите расширенный режим одновременной комбинации, чтобы использовать несколько основных клавиш.',
    remove: 'Удалить {key}',
  },
  common: {
    cancel: 'Отмена',
    media: 'Мультимедиа',
  },
} satisfies Locale;
