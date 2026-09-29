import { afterEach, describe, expect, it, vi } from 'vitest';
import { uploadButtonImage } from './button-upload';
import type { ButtonState } from './modalstyle';

const MODAL_ID = 't9';

function mountPreview(): void {
  document.body.innerHTML = `
    <img id="button-image_${MODAL_ID}" src="old.png" />
    <input id="image-size-slider_${MODAL_ID}" type="range" value="75" />`;
}

function fileInput(name: string | null): HTMLInputElement {
  const input = document.createElement('input');
  input.type = 'file';
  const files = name === null ? [] : [new File(['x'], name, { type: 'image/png' })];
  Object.defineProperty(input, 'files', { value: files, configurable: true });
  return input;
}

function okUpload() {
  return vi.fn(async () => ({
    ok: true,
    headers: { get: () => 'text/plain' },
    text: async () => 'ok',
  }));
}

describe('uploadButtonImage', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    document.body.innerHTML = '';
  });

  it('posts the file, swaps the preview, and records path + size', async () => {
    mountPreview();
    const fetchMock = okUpload();
    vi.stubGlobal('fetch', fetchMock);
    const state: ButtonState = {};

    await uploadButtonImage(MODAL_ID, fileInput('art.png'), state);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe('/upload_file');
    const posted = (init.body as FormData).get('file') as File;
    expect(posted.name).toBe('art.png');

    expect(
      (document.querySelector(`#button-image_${MODAL_ID}`) as HTMLImageElement).getAttribute('src')
    ).toBe('.config/user_uploads/art.png');
    expect(
      (document.querySelector(`#image-size-slider_${MODAL_ID}`) as HTMLInputElement).value
    ).toBe('70');
    expect(state['image']).toBe('**uploaded/art.png');
    expect(state['image_size']).toBe('70');
  });

  it('does nothing without a chosen file', async () => {
    mountPreview();
    const fetchMock = okUpload();
    vi.stubGlobal('fetch', fetchMock);
    const state: ButtonState = {};

    await uploadButtonImage(MODAL_ID, fileInput(null), state);

    expect(fetchMock).not.toHaveBeenCalled();
    expect(state).toEqual({});
  });

  it('keeps the previous preview and state when the upload fails', async () => {
    mountPreview();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new Error('network down');
      })
    );
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const state: ButtonState = { image: 'keep.png', image_size: '75%' };

    await uploadButtonImage(MODAL_ID, fileInput('art.png'), state);

    expect(errorSpy).toHaveBeenCalled();
    expect(state).toEqual({ image: 'keep.png', image_size: '75%' });
    expect(
      (document.querySelector(`#button-image_${MODAL_ID}`) as HTMLImageElement).getAttribute('src')
    ).toBe('old.png');
  });
});
