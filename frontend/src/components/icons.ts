import { html, type Html } from '../framework/html';

// Shared inline-SVG icon paths: single source for the glyphs previously
// pasted into each view. All static icons render as Svelte components
// (`CloseIcon.svelte`, ...); only the add-slot plus keeps a string
// builder — `void.ts` parses it into runtime-created void slots.

export const CLOSE_PATH =
  'M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z';

export const PLUS_PATH = 'M12 4v16m8-8H4';

export const PENCIL_PATH =
  'M12 8.00012L4 16.0001V20.0001L8 20.0001L16 12.0001M12 8.00012L14.8686 5.13146L14.8704 5.12976C15.2652 4.73488 15.463 4.53709 15.691 4.46301C15.8919 4.39775 16.1082 4.39775 16.3091 4.46301C16.5369 4.53704 16.7345 4.7346 17.1288 5.12892L18.8686 6.86872C19.2646 7.26474 19.4627 7.46284 19.5369 7.69117C19.6022 7.89201 19.6021 8.10835 19.5369 8.3092C19.4628 8.53736 19.265 8.73516 18.8695 9.13061L18.8686 9.13146L16 12.0001M12 8.00012L16 12.0001';

export const X_PATH =
  'M18.8,16l5.5-5.5c0.8-0.8,0.8-2,0-2.8l0,0C24,7.3,23.5,7,23,7c-0.5,0-1,0.2-1.4,0.6L16,13.2l-5.5-5.5  c-0.8-0.8-2.1-0.8-2.8,0C7.3,8,7,8.5,7,9.1s0.2,1,0.6,1.4l5.5,5.5l-5.5,5.5C7.3,21.9,7,22.4,7,23c0,0.5,0.2,1,0.6,1.4  C8,24.8,8.5,25,9,25c0.5,0,1-0.2,1.4-0.6l5.5-5.5l5.5,5.5c0.8,0.8,2.1,0.8,2.8,0c0.8-0.8,0.8-2.1,0-2.8L18.8,16z';

export const FOLDER_X_PATH =
  'M11.742 4.258a1 1 0 0 0-1.414 0L8 6.586 5.672 4.258a1 1 0 1 0-1.414 1.414L6.586 8 4.258 10.328a1 1 0 0 0 1.414 1.414L8 9.414l2.328 2.328a1 1 0 0 0 1.414-1.414L9.414 8l2.328-2.328a1 1 0 0 0 0-1.414z';

export const TRASH_PATHS = [
  'M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5Zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5Zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6Z',
  'M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1ZM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118ZM2.5 3h11V2h-11v1Z',
];

/** Add-button plus glyph (empty grid slots). Parsed by `void.ts` at runtime. */
export function addPlusIcon(): Html {
  return html`<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"><path d="${PLUS_PATH}" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
}

// Section glyphs: one distinct inline icon per settings group so collapsed
// sections are scannable without opening them. 16x16 stroke set rendered
// by `SectionIcon.svelte` (inner SVG only; the component owns the shell).

export type SectionIconName =
  | 'sliders'
  | 'speaker'
  | 'music'
  | 'video'
  | 'grid'
  | 'image'
  | 'eye'
  | 'chart'
  | 'flask'
  | 'swatch'
  | 'upload'
  | 'check';

export const SECTION_ICONS: Record<SectionIconName, string> = {
  sliders:
    '<path d="M2 4.5h12M2 8h12M2 11.5h12"/>' +
    '<circle cx="10.5" cy="4.5" r="1.9"/><circle cx="5.5" cy="8" r="1.9"/><circle cx="11" cy="11.5" r="1.9"/>',
  speaker:
    '<path d="M2 6v4h2.5L8 13V3L4.5 6H2z"/>' + '<path d="M10 5.5a4 4 0 0 1 0 5M12 3.5a7 7 0 0 1 0 9"/>',
  music:
    '<circle cx="5" cy="11.8" r="2.4"/><circle cx="11" cy="10.8" r="2.4"/>' +
    '<path d="M7.4 11.8V3.6l6-1.1v8.3"/>',
  video:
    '<rect x="1.5" y="4" width="9" height="8" rx="2"/>' + '<path d="M10.5 7.5l4-2.8v6.6l-4-2.8"/>',
  grid:
    '<rect x="2" y="2" width="5" height="5" rx="1.2"/><rect x="9" y="2" width="5" height="5" rx="1.2"/>' +
    '<rect x="2" y="9" width="5" height="5" rx="1.2"/><rect x="9" y="9" width="5" height="5" rx="1.2"/>',
  image:
    '<rect x="2" y="2.5" width="12" height="11" rx="2"/>' +
    '<circle cx="5.6" cy="6.4" r="1.3"/><path d="M2.5 11.5l3.4-3.4 2.4 2.4 2-2 3.2 3.2"/>',
  eye: '<path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/>' + '<circle cx="8" cy="8" r="2"/>',
  chart: '<path d="M2 2v11.5h12"/>' + '<path d="M5.5 11V7M8.5 11V4.5M11.5 11V8" stroke-width="2.4"/>',
  flask:
    '<path d="M6.5 2h3M7 2v4.5L3 12a1 1 0 0 0 .9 1.5h8.2A1 1 0 0 0 13 12L9 6.5V2"/>' +
    '<path d="M5 10.5h6"/>',
  swatch:
    '<rect x="4.5" y="4.5" width="9" height="9" rx="2"/>' +
    '<path d="M11.5 4.5v-1A1.5 1.5 0 0 0 10 2H3.5A1.5 1.5 0 0 0 2 3.5V10a1.5 1.5 0 0 0 1.5 1.5h1"/>',
  upload:
    '<path d="M8 10V2.5M5 5l3-3 3 3"/>' + '<path d="M2.5 10.5v2A1.5 1.5 0 0 0 4 14h8a1.5 1.5 0 0 0 1.5-1.5v-2"/>',
  check: '<path d="M2.5 8.5l3.5 3.5 7.5-8"/>',
};
