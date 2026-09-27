// Effects engine: Web Animations API with honest fallbacks.
// - No WAAPI (`el.animate` missing) → final keyframe applied instantly.
// - `prefers-reduced-motion` → durations collapse to 0 (jump to end).
// - Every animation resolves (never rejects): `stop()` cancels cleanly.

export interface EffectOptions {
  duration?: number;
  easing?: string;
  complete?: () => void;
}

type Stylable = HTMLElement | SVGElement;

function asStylable(el: Element): Stylable | null {
  return el instanceof HTMLElement || el instanceof SVGElement ? el : null;
}

/** Display cache for hide/show round-trips (jQuery's old-display). */
const oldDisplay = new WeakMap<Element, string>();

/** Currently running animations per element (for stop()). */
const running = new WeakMap<Element, Animation[]>();

/** Per-tag default display cache (jQuery.defaultDisplay). */
const defaultDisplayCache = new Map<string, string>();

function defaultDisplay(tag: string): string {
  const cached = defaultDisplayCache.get(tag);
  if (cached !== undefined) return cached;
  const probe = document.createElement(tag);
  document.body.appendChild(probe);
  const display = getComputedStyle(probe).display;
  probe.remove();
  const resolved = display === 'none' ? 'block' : display;
  defaultDisplayCache.set(tag, resolved);
  return resolved;
}

/**
 * Hidden check. DEVIATION: display-based only (jQuery also requires empty
 * boxes), so it stays correct in layout-less environments (SSR, happy-dom).
 */
export function isHidden(el: Element): boolean {
  return getComputedStyle(el).display === 'none';
}

export function showInstant(el: Element): void {
  const target = asStylable(el);
  if (!target) return;
  if (target.style.display === 'none') {
    target.style.display = oldDisplay.get(el) ?? '';
  }
  if (getComputedStyle(target).display === 'none') {
    target.style.display = defaultDisplay(target.tagName);
  }
}

export function hideInstant(el: Element): void {
  const target = asStylable(el);
  if (!target) return;
  // Cache the exact inline value (even ''): show() must restore it
  // verbatim, exactly like jQuery's old-display.
  if (getComputedStyle(target).display !== 'none' && !oldDisplay.has(el)) {
    oldDisplay.set(el, target.style.display);
  }
  target.style.display = 'none';
}

function reducedMotion(): boolean {
  return (
    typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

function mapEasing(easing: string): string {
  // jQuery's default 'swing' as a cubic-bezier; anything else passes
  // through as a native CSS easing string.
  if (easing === 'swing') return 'cubic-bezier(0.02, 0.01, 0.47, 1)';
  return easing;
}

/** Keys of Keyframe that are not CSS properties. */
const KEYFRAME_META = new Set(['offset', 'easing', 'composite', 'computedOffset', 'endDelay']);

function applyKeyframe(el: Stylable, frame: Keyframe): void {
  for (const [prop, value] of Object.entries(frame)) {
    if (KEYFRAME_META.has(prop) || value === null || value === undefined) continue;
    const text = String(value);
    if (prop.startsWith('--')) {
      el.style.setProperty(prop, text);
    } else if (prop in el.style) {
      el.style.setProperty(prop.replace(/[A-Z]/g, (ch) => `-${ch.toLowerCase()}`), text);
    }
  }
}

/** Track an animation so stop() can reach it. */
function track(el: Element, animation: Animation): void {
  const list = running.get(el) ?? [];
  list.push(animation);
  running.set(el, list);
}

function untrack(el: Element, animation: Animation): void {
  const list = running.get(el);
  if (!list) return;
  const next = list.filter((a) => a !== animation);
  if (next.length > 0) running.set(el, next);
  else running.delete(el);
}

/**
 * Run keyframes on one element and resolve when done. Resolves (never
 * rejects): cancellation via stop() settles quietly like jQuery.
 */
export function runAnimation(
  el: Element,
  keyframes: Keyframe[] | PropertyIndexedKeyframes,
  options: EffectOptions
): Promise<void> {
  const target = asStylable(el);
  const duration = reducedMotion() ? 0 : (options.duration ?? 400);
  // The end state is always applied explicitly (never via commitStyles,
  // which some engines implement as a no-op), so promises settle with
  // the documented style in every environment.
  const applyFinal = (): void => {
    if (target === null) return;
    if (Array.isArray(keyframes)) {
      const last = keyframes[keyframes.length - 1];
      if (last !== undefined) applyKeyframe(target, last);
    } else {
      applyIndexedKeyframe(target, keyframes);
    }
  };
  if (duration <= 0 || target === null || typeof target.animate !== 'function') {
    applyFinal();
    options.complete?.();
    return Promise.resolve();
  }
  const easing = mapEasing(options.easing ?? 'ease');
  const animation = target.animate(keyframes, { duration, easing });
  track(el, animation);
  return animation.finished.then(
    () => {
      untrack(el, animation);
      applyFinal();
      try {
        animation.cancel();
      } catch {
        // Already settled underneath us; end state stands.
      }
      options.complete?.();
    },
    () => {
      untrack(el, animation);
      options.complete?.();
    }
  );
}

/** Final values of a property-indexed keyframe set (fallback path). */
function applyIndexedKeyframe(el: Stylable, frames: PropertyIndexedKeyframes): void {
  for (const [prop, value] of Object.entries(frames)) {
    if (KEYFRAME_META.has(prop) || value === null || value === undefined) continue;
    const text = Array.isArray(value) ? value[value.length - 1] : value;
    if (text === undefined || text === null) continue;
    if (prop.startsWith('--')) {
      el.style.setProperty(prop, String(text));
    } else {
      el.style.setProperty(prop.replace(/[A-Z]/g, (ch) => `-${ch.toLowerCase()}`), String(text));
    }
  }
}

/** Opacity tween; `from: null` starts at the current computed opacity. */
export function animateOpacity(
  el: Element,
  from: number | null,
  to: number,
  options: EffectOptions
): Promise<void> {
  let start = from;
  if (start === null) {
    const computed = Number.parseFloat(getComputedStyle(el).opacity);
    start = Number.isNaN(computed) ? (to === 0 ? 1 : 0) : computed;
  }
  return runAnimation(el, [{ opacity: start }, { opacity: to }], options);
}

/**
 * Animated show/hide. DEVIATION: opacity-based (jQuery also tweens
 * dimensions); pair with slideDown/slideUp when height motion is wanted.
 */
export function animateShowHide(el: Element, show: boolean, options: EffectOptions): Promise<void> {
  if (show) {
    showInstant(el);
    return animateOpacity(el, 0, 1, options);
  }
  return animateOpacity(el, null, 0, options).then(() => {
    hideInstant(el);
  });
}

/**
 * Height tween (plus vertical padding). DEVIATION: margins are left
 * alone (jQuery tweens those too).
 */
export function animateSlide(el: Element, down: boolean, options: EffectOptions): Promise<void> {
  const target = asStylable(el);
  if (!target) {
    options.complete?.();
    return Promise.resolve();
  }
  if (down) {
    showInstant(target);
    const height = target.scrollHeight;
    const style = getComputedStyle(target);
    const paddingTop = style.paddingTop;
    const paddingBottom = style.paddingBottom;
    const previousOverflow = target.style.overflow;
    target.style.overflow = 'hidden';
    return runAnimation(
      target,
      [
        { height: '0px', paddingTop: '0px', paddingBottom: '0px' },
        { height: `${height}px`, paddingTop, paddingBottom },
      ],
      options
    ).then(() => {
      target.style.height = '';
      target.style.paddingTop = '';
      target.style.paddingBottom = '';
      target.style.overflow = previousOverflow;
    });
  }
  const rectHeight = target.getBoundingClientRect().height;
  const style = getComputedStyle(target);
  const paddingTop = style.paddingTop;
  const paddingBottom = style.paddingBottom;
  const previousOverflow = target.style.overflow;
  target.style.overflow = 'hidden';
  return runAnimation(
    target,
    [
      { height: `${rectHeight}px`, paddingTop, paddingBottom },
      { height: '0px', paddingTop: '0px', paddingBottom: '0px' },
    ],
    options
  ).then(() => {
    hideInstant(target);
    target.style.height = '';
    target.style.paddingTop = '';
    target.style.paddingBottom = '';
    target.style.overflow = previousOverflow;
  });
}

/** Cancel running animations (`gotoEnd` jumps them to their end state). */
export function stopAnimations(el: Element, gotoEnd: boolean): void {
  const list = running.get(el) ?? [];
  running.delete(el);
  for (const animation of list) {
    try {
      if (gotoEnd) animation.finish();
      else animation.cancel();
    } catch {
      // Already settled; nothing to do.
    }
  }
}
