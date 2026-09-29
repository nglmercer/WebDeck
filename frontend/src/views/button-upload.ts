import { byId, post } from '../query';
import { swapPreviewImage, updateImageSize, type ButtonState } from './modalstyle';

/**
 * Shared image-upload flow for the add/edit button modals: post the
 * chosen file, swap the preview to the uploaded art, and record the
 * new image path + size on the button state. Upload failures only log
 * (the modal stays open with its previous preview).
 */
export async function uploadButtonImage(
  id: string,
  input: HTMLInputElement,
  state: ButtonState
): Promise<void> {
  const file = input.files?.[0];
  if (!file) return;
  const formData = new FormData();
  formData.append('file', file);
  try {
    await post('/upload_file', formData);
  } catch {
    console.error('Failed to download file.');
    return;
  }
  const swapped = swapPreviewImage(id, input);
  if (!swapped) return;
  state['image_size'] = '70';
  updateImageSize(swapped.slider, swapped.image, state);
  state['image'] = '**uploaded/' + (input.files?.[0]?.name ?? '');
}

/** Bind a modal's image file input to the shared upload flow. */
export function wireButtonImageUpload(id: string, state: ButtonState): void {
  byId(`image-input_${id}`).on('change', function () {
    void uploadButtonImage(id, this as unknown as HTMLInputElement, state);
  });
}
