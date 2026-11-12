import englishCatalog from '../../locales/en_US';
import { flattenMessages, type MessageKey } from '../../locales/schema';
export const messages = flattenMessages(englishCatalog);
type TranslationValues = Record<string, string | number>;
export type TranslationKey = MessageKey;
export type Translator = {
  (key: TranslationKey, values?: TranslationValues): string;
  /** Runtime errors, schema labels and extension-provided text may not have known keys. */
  message: (message: string, values?: TranslationValues) => string;
};
const english = messages as Record<string, string>;
/** Only explicit message paths are localized; external server errors remain readable. */
export function createTranslator(dictionary: () => Record<string, string>): Translator {
  const translate: Translator['message'] = (message, values = {}) => {
    const localized = dictionary();
    const translated = Object.hasOwn(localized, message) ? localized[message] : undefined;
    const fallback = Object.hasOwn(english, message) ? english[message] : undefined;
    const text = (typeof translated === 'string' && translated) || fallback || message;
    return text.replace(
      /\{([a-zA-Z0-9_]+)\}|%([a-zA-Z0-9_]+)%/g,
      (placeholder, brace: string | undefined, percent: string | undefined) => {
        const name = brace ?? percent ?? '';
        return Object.hasOwn(values, name) ? String(values[name]) : placeholder;
      },
    );
  };
  return Object.assign(translate, { message: translate });
}
