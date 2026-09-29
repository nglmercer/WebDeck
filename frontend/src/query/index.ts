// qdom public API: the chainable wrapper, statics, and shared types.

// Side-effect imports: Q method groups attach via declaration merging.
// (Every consumer imports through this barrel, so the full Q always lands.)
import './attributes';
import './classes-css';
import './data';
import './effects';
import './events';
import './factory';
import './manipulate';
import './traverse';

export { Q, type Content, type ElementPredicate } from './core';
export { q, byId, ready, type QueryContext } from './factory';
export { each, map, extend, contains } from './utils';
export type { EffectOptions } from './effects';
export type { EventHandler } from './events';

/** `$`-style alias for code migrating off jQuery (`$q('div')`). */
export { q as $q } from './factory';
