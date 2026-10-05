import { messages } from './messages';
type TranslationValues = Record<string, string | number>;
export type Translator = (key: string, values?: TranslationValues) => string;
const english = messages as Record<string, string>;
const keysByText = new Map<string, string>(
  Object.entries(messages).map(([key, text]) => [text, key]),
);
/** Known English copy also resolves to its stable key; unknown server errors remain readable. */
export function createTranslator(dictionary: () => Record<string, string>): Translator {
  return (message, values = {}) => {
    const key = keysByText.get(message) ?? message;
    const translated = Object.hasOwn(dictionary(), key) ? dictionary()[key] : undefined;
    const fallback = Object.hasOwn(english, key) ? english[key] : undefined;
    const text = (typeof translated === 'string' && translated) || fallback || message;
    return text.replace(
      /\{([a-zA-Z0-9_]+)\}|%([a-zA-Z0-9_]+)%/g,
      (placeholder, brace: string | undefined, percent: string | undefined) => {
        const name = brace ?? percent ?? '';
        return Object.hasOwn(values, name) ? String(values[name]) : placeholder;
      },
    );
  };
}
