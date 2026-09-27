// qdom public API: the chainable wrapper, statics, and shared types.

export { Q, q, ready, type Content, type ElementPredicate, type QueryContext } from './core';
export { ajax, get, getJSON, post, HttpError, type AjaxOptions } from './ajax';
export { each, map, extend, contains } from './utils';
export type { EffectOptions } from './effects';
export type { EventHandler } from './events';

/** `$`-style alias for code migrating off jQuery (`$q('div')`). */
export { q as $q } from './core';
