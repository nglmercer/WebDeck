// Q class/style/visibility methods (extracted from core.ts).

import { Q } from './core';
import {
  animateShowHide,
  hideInstant,
  isHidden,
  normalizeEffectOptions,
  showInstant,
} from './effects';
import type { EffectOptions } from './effects';

declare module './core' {
  interface Q<T extends Element> {
    /** Add classes (string, list, or per-element function). */
    addClass(value: string | string[] | ((this: T, index: number, current: string) => string)): this;
    /** Remove classes (omitted clears all). */
    removeClass(
      value?: string | string[] | ((this: T, index: number, current: string) => string)
    ): this;
    /** Toggle classes (`force` pins the outcome). */
    toggleClass(
      value: string | string[] | ((this: T, index: number, current: string) => string),
      force?: boolean
    ): this;
    /** True when the first element carries `name`. */
    hasClass(name: string): boolean;
    /** Computed style value of the first element. */
    css(property: string): string | undefined;
    /** Set one style property (numbers gain `px` unless unitless). */
    css(property: string, value: string | number): this;
    /** Set several style properties at once. */
    css(values: Record<string, string | number>): this;
    /** Show every element (restores the pre-hide display). */
    show(): this;
    /** Animated show (opacity). Resolves when done. */
    show(duration: number | EffectOptions): Promise<void>;
    /** Hide every element (caches display for show()). */
    hide(): this;
    /** Animated hide (opacity, then display:none). Resolves when done. */
    hide(duration: number | EffectOptions): Promise<void>;
    /** Toggle visibility instantly (`force` pins the outcome). */
    toggle(force?: boolean): this;
    /** Animated toggle. Resolves when done. */
    toggle(duration: number | EffectOptions): Promise<void>;
  }
}

/** jQuery.cssNumber: properties that stay unitless when set with a number. */
const UNITLESS = new Set([
  'animationIterationCount',
  'aspectRatio',
  'borderImageOutset',
  'borderImageSlice',
  'borderImageWidth',
  'boxFlex',
  'boxFlexGroup',
  'boxOrdinalGroup',
  'columnCount',
  'columns',
  'flex',
  'flexGrow',
  'flexPositive',
  'flexShrink',
  'flexNegative',
  'flexOrder',
  'fontWeight',
  'gridArea',
  'gridColumn',
  'gridColumnEnd',
  'gridColumnStart',
  'gridRow',
  'gridRowEnd',
  'gridRowStart',
  'lineClamp',
  'lineHeight',
  'opacity',
  'order',
  'orphans',
  'scale',
  'widows',
  'zIndex',
  'zoom',
  'fillOpacity',
  'floodOpacity',
  'stopOpacity',
  'strokeMiterlimit',
  'strokeOpacity',
  'strokeWidth',
]);

/** setProperty/getPropertyValue need kebab-case (custom props as-is). */
function cssPropertyName(property: string): string {
  if (property.startsWith('--')) return property;
  return property.replace(/[A-Z]/g, (ch) => `-${ch.toLowerCase()}`);
}

function addPx(property: string, value: number): string {
  if (UNITLESS.has(property)) return String(value);
  return `${value}px`;
}

function eachClassName<T extends Element>(
  els: readonly T[],
  value: string | string[] | ((this: T, index: number, current: string) => string),
  apply: (el: T, names: string[]) => void
): void {
  let index = 0;
  for (const el of els) {
    const raw = typeof value === 'function' ? value.call(el, index, el.className) : value;
    const names = (Array.isArray(raw) ? raw.join(' ') : raw).split(/\s+/).filter((n) => n !== '');
    apply(el, names);
    index++;
  }
}

Q.prototype.addClass = function <T extends Element>(
  this: Q<T>,
  value: string | string[] | ((this: T, index: number, current: string) => string)
): Q<T> {
  eachClassName(this.els, value, (el, names) => {
    if (names.length > 0) el.classList.add(...names);
  });
  return this;
};

Q.prototype.removeClass = function <T extends Element>(
  this: Q<T>,
  value?: string | string[] | ((this: T, index: number, current: string) => string)
): Q<T> {
  if (value === undefined) {
    for (const el of this.els) el.className = '';
    return this;
  }
  eachClassName(this.els, value, (el, names) => {
    if (names.length > 0) el.classList.remove(...names);
  });
  return this;
};

Q.prototype.toggleClass = function <T extends Element>(
  this: Q<T>,
  value: string | string[] | ((this: T, index: number, current: string) => string),
  force?: boolean
): Q<T> {
  eachClassName(this.els, value, (el, names) => {
    for (const name of names) el.classList.toggle(name, force);
  });
  return this;
};

Q.prototype.hasClass = function (this: Q, name: string): boolean {
  const first = this.els[0];
  return first !== undefined && first.classList.contains(name);
};

function cssImpl<T extends Element>(
  this: Q<T>,
  propertyOrValues: string | Record<string, string | number>,
  value?: string | number
): string | undefined | Q<T> {
  if (typeof propertyOrValues === 'string' && value === undefined) {
    const first = this.els[0];
    if (first === undefined) return undefined;
    return getComputedStyle(first).getPropertyValue(cssPropertyName(propertyOrValues));
  }
  const entries: Array<[string, string | number]> =
    typeof propertyOrValues === 'string'
      ? [[propertyOrValues, value ?? '']]
      : Object.entries(propertyOrValues);
  for (const el of this.els) {
    const target = el instanceof HTMLElement || el instanceof SVGElement ? el : null;
    if (!target) continue;
    for (const [property, v] of entries) {
      target.style.setProperty(cssPropertyName(property), typeof v === 'number' ? addPx(property, v) : v);
    }
  }
  return this;
}

Q.prototype.css = cssImpl as Q<Element>['css'];

function showImpl<T extends Element>(this: Q<T>, duration?: number | EffectOptions): Q<T> | Promise<void> {
  if (duration === undefined) {
    for (const el of this.els) showInstant(el);
    return this;
  }
  const options = normalizeEffectOptions(duration);
  return Promise.all(this.els.map((el) => animateShowHide(el, true, options))).then(() => undefined);
}

Q.prototype.show = showImpl as Q<Element>['show'];

function hideImpl<T extends Element>(this: Q<T>, duration?: number | EffectOptions): Q<T> | Promise<void> {
  if (duration === undefined) {
    for (const el of this.els) hideInstant(el);
    return this;
  }
  const options = normalizeEffectOptions(duration);
  return Promise.all(this.els.map((el) => animateShowHide(el, false, options))).then(() => undefined);
}

Q.prototype.hide = hideImpl as Q<Element>['hide'];

function toggleImpl<T extends Element>(
  this: Q<T>,
  forceOrDuration?: boolean | number | EffectOptions
): Q<T> | Promise<void> {
  if (typeof forceOrDuration === 'number' || typeof forceOrDuration === 'object') {
    const options = normalizeEffectOptions(forceOrDuration);
    return Promise.all(
      this.els.map((el) => animateShowHide(el, isHidden(el), options))
    ).then(() => undefined);
  }
  for (const el of this.els) {
    const show = forceOrDuration ?? isHidden(el);
    if (show) showInstant(el);
    else hideInstant(el);
  }
  return this;
}

Q.prototype.toggle = toggleImpl as Q<Element>['toggle'];
