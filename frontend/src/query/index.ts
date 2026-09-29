// qdom public API: the chainable wrapper, statics, and shared types.

// Side-effect imports: Q method groups attach via declaration merging.
// (Every consumer imports through this barrel, so the full Q always lands.)
import './attributes';
import './classes-css';
import './events';
import './factory';
import './manipulate';
import './traverse';

export { Q, type Content, type ElementPredicate } from './core';
export { q, byId, type QueryContext } from './factory';
export type { EventHandler } from './events';
