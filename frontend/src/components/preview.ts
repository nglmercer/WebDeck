import { rep } from '../framework/types';

/**
 * Shared button-preview data (`.fakeform` content of the editor-style
 * block). Add and edit modals render the same tile shape with small
 * per-side differences (usage overlay, img attributes, button style);
 * the builders in `views/addbutton/preview` and `views/editmodal` fill
 * this model and `Preview.svelte` renders it.
 */

export type PreviewMedia =
  | {
      kind: 'img';
      /** `null` renders no `src` attribute (empty placeholder image). */
      src: string | null;
      /** `null` omits the attribute (add side never sets it on src imgs). */
      alt: string | null;
      /** Broken uploads remove the element (`onerror="this.remove()"`). */
      removeOnError: boolean;
      widthPx: number;
      fill: string;
    }
  | { kind: 'svg'; slot: number };

export interface PreviewData {
  id: string;
  /** Whether the button carries `id="button-element_${id}"`. */
  buttonId: boolean;
  buttonStyle: string;
  media: PreviewMedia;
  /** Usage-overlay fill; `null` when the variant has no usage block (edit). */
  usageFill: string | null;
  text: string;
  /** Raw `style` attribute for the buttontext, or `null` when absent. */
  textStyle: string | null;
}

/**
 * Style-image path → URL. Add and edit renderers carried identical copies
 * of this mapping; upstream reads an out-of-scope config path for the
 * `**uploaded/` branch (renders broken), so both use the style image
 * itself and uploaded art actually previews.
 */
export function previewImageLink(image: string): string {
  if (image.startsWith('http')) return image;
  if (image.includes(':')) return 'static/img/' + (image.split('\\').pop() ?? image);
  if (image.startsWith('**uploaded/')) {
    return '.config/user_uploads/' + rep(image, '**uploaded/', '');
  }
  return 'static/img/' + image;
}
