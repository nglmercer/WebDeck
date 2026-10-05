import { getContext, setContext } from 'svelte';
import { createTranslator, type Translator } from './translations';
const context = Symbol('webdeck.translations');
export function provideTranslations(dictionary: () => Record<string, string>): Translator {
  const translate = createTranslator(dictionary);
  setContext(context, translate);
  return translate;
}
export function useTranslations(): Translator {
  return getContext<Translator | undefined>(context) ?? createTranslator(() => ({}));
}
