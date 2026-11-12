import { getContext, setContext } from 'svelte';
import type { GuideId } from './guides';
const context = Symbol('webdeck.guide-help');
export function provideGuideHelp(open: (id: GuideId) => void) {
  setContext(context, open);
}
export function useGuideHelp() {
  return getContext<((id: GuideId) => void) | undefined>(context);
}
