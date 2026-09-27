import { html, raw, type Html } from '../framework/html';

// Replaces `open(path).read()` SVG inlining: the markup renders a
// placeholder span, then `hydrateSvgs` fetches each file and applies the
// exact same `<svg` string surgery the templates did.

export interface SvgSlot {
  path: string;
  attrs: string;
  replaceNeedle: '<svg ' | '<svg';
}

const slots: SvgSlot[] = [];
let slotSeq = 0;

/** Render a placeholder for an inlined SVG (resolved by `hydrateSvgs`). */
export function svgSlot(path: string, attrs: string, needle: '<svg ' | '<svg' = '<svg '): Html {
  const id = slotSeq++;
  slots.push({ path, attrs, replaceNeedle: needle });
  return html`<span data-svg-slot="${String(id)}" style="display:contents"></span>`;
}

/** Fetch all Solo placeholders and inline them (same surgery as Jinja). */
export async function hydrateSvgs(root: ParentNode = document): Promise<void> {
  const spans = Array.from(root.querySelectorAll('span[data-svg-slot]'));
  await Promise.all(
    spans.map(async (span) => {
      const id = Number(span.getAttribute('data-svg-slot'));
      const slot = slots[id];
      if (!slot) return;
      try {
        const response = await fetch(slot.path);
        if (!response.ok) return; // mirrors {% if isfile(...) %} guards
        const svg = await response.text();
        if (!svg.includes('<svg')) return;
        const inlined =
          slot.replaceNeedle === '<svg '
            ? svg.replace('<svg ', `<svg ${slot.attrs} `)
            : svg.replace('<svg', `<svg${slot.attrs} `);
        const template = document.createElement('template');
        template.innerHTML = inlined.trim();
        span.replaceWith(template.content.cloneNode(true));
      } catch {
        // Missing file renders nothing, like the isfile guards.
      }
    })
  );
}

/** Brand SVG icon used by several config links (info.svg + icon class). */
export function infoIcon(darkTheme: string): Html {
  return svgSlot('static/img/info.svg', `class="info-icon${darkTheme}"`);
}

export function rawHtml(value: string): Html {
  return raw(value);
}
