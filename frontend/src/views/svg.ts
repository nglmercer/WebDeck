import { getText } from '../api/client';
import { q } from '../query';

// Replaces `open(path).read()` SVG inlining: the markup renders a
// placeholder span, then `hydrateSvgs` fetches each file and applies the
// exact same `<svg` string surgery the templates did.

interface SvgSlot {
  path: string;
  attrs: string;
  replaceNeedle: '<svg ' | '<svg';
}

let slots: SvgSlot[] = [];
let slotSeq = 0;
// One shared fetch per icon path: grids repeat the same few icons dozens
// of times, and refetching per placeholder costs ~100 HTTP round trips.
const svgCache = new Map<string, Promise<string | null>>();

/**
 * Drop all registered slots (call before a fresh mount: slot ids are
 * assigned during render, so re-renders must start from zero instead of
 * leaking stale entries on every refresh).
 */
export function resetSvgSlots(): void {
  slots = [];
  slotSeq = 0;
}

async function fetchSvg(path: string): Promise<string | null> {
  let pending = svgCache.get(path);
  if (!pending) {
    pending = (async () => {
      try {
        const svg = await getText(path);
        return svg.includes('<svg') ? svg : null;
      } catch {
        // Missing file renders nothing, like the isfile guards.
        return null;
      }
    })();
    svgCache.set(path, pending);
  }
  return pending;
}

/**
 * Inline size style for an inlined SVG icon. Both dimensions carry `px`
 * units — a unitless `height` is dropped by the CSS parser and the SVG
 * falls back to its tiny intrinsic height (e.g. 16px Bootstrap icons).
 */
export function svgInlineStyle(px: number, fill: string): string {
  return `style="width:${px}px; height:${px}px; ${fill}"`;
}

/**
 * Register an inlined-SVG placeholder (resolved by `hydrateSvgs`) and
 * return its slot id. Views render the marker natively (`SvgSlot.svelte`).
 */
export function svgSlotId(path: string, attrs: string, needle: '<svg ' | '<svg' = '<svg '): number {
  const id = slotSeq++;
  slots.push({ path, attrs, replaceNeedle: needle });
  return id;
}

/** Fetch all Solo placeholders and inline them (same surgery as Jinja). */
export async function hydrateSvgs(root: ParentNode = document): Promise<void> {
  const spans = q('span[data-svg-slot]', root).toArray();
  await Promise.all(
    spans.map(async (span) => {
      const id = Number(q(span).attr('data-svg-slot'));
      const slot = slots[id];
      if (!slot) return;
      const svg = await fetchSvg(slot.path);
      if (svg === null) return;
      const inlined =
        slot.replaceNeedle === '<svg '
          ? svg.replace('<svg ', `<svg ${slot.attrs} `)
          : svg.replace('<svg', `<svg${slot.attrs} `);
      q(span).replaceWith(inlined.trim());
    })
  );
}

/** Brand SVG icon used by several config links (info.svg + icon class). */
export function infoSlotId(darkTheme: string): number {
  return svgSlotId('static/img/info.svg', `class="info-icon${darkTheme}"`);
}
