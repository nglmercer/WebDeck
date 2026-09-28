import { byId } from '../query';

/**
 * Studio live-preview sync for button-config modals (v2-estudio sidebar):
 * keeps the meta rows (`Key / Size / Color`) and the size `%` badge in
 * step with the form. Best-effort chrome — every lookup is nullable and
 * a missing element simply skips its update.
 */

function setText(id: string, value: string): void {
  const el = byId(id).get(0) ?? null;
  if (el) el.textContent = value;
}

function inputValue(id: string): string {
  const el = byId<HTMLInputElement>(id).get(0) ?? null;
  return el ? String(el.value ?? '') : '';
}

/** One-shot refresh of every studio readout for one modal. */
export function refreshStudioPreview(modalId: string): void {
  const sizeRaw = inputValue(`image-size-value_${modalId}`).trim();
  const size = sizeRaw !== '' ? `${sizeRaw} %` : '—';
  setText(`size-pct_${modalId}`, size);
  setText(`meta-size_${modalId}`, size);

  const color = inputValue(`background-color-hex_${modalId}`).trim();
  setText(`meta-color_${modalId}`, color !== '' ? color : '—');

  const keyInput = inputValue(`key-input_${modalId}`).trim();
  let key = keyInput;
  if (key === '') {
    const first =
      document.querySelector(
        `form[arg_modal_ID="${modalId}"] .args-container input[type="text"],` +
          ` form[edit_modal_ID="${modalId}"] .args-container input[type="text"]`
      ) as HTMLInputElement | null;
    key = (first?.value ?? '').trim();
  }
  setText(`meta-key_${modalId}`, key !== '' ? key : '—');
}

/**
 * Live wiring: `input` covers typing/sliders/pickers, `change` covers
 * selects, and `click` (deferred a tick) covers chip/capture buttons
 * that assign `.value` programmatically without firing events.
 */
export function wireStudioPreview(modalId: string): void {
  const root =
    byId(`edit-modal-container-${modalId}`).get(0) ??
    byId(`modal-container-${modalId}`).get(0) ??
    null;
  if (!root) return;
  refreshStudioPreview(modalId);
  root.addEventListener('input', () => refreshStudioPreview(modalId));
  root.addEventListener('change', () => refreshStudioPreview(modalId));
  root.addEventListener('click', () => {
    setTimeout(() => refreshStudioPreview(modalId), 0);
  });
}
