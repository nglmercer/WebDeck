import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import Preview from './Preview.svelte';
import { previewImageLink, type PreviewData } from './preview';

function base(overrides: Partial<PreviewData> = {}): PreviewData {
  return {
    id: 'p1',
    buttonId: true,
    buttonStyle: 'overflow: hidden; overflow-y: hidden; max-height: 89.6px;',
    media: { kind: 'img', src: null, alt: '', removeOnError: false, widthPx: 59, fill: '' },
    usageFill: '',
    text: 'Name',
    textStyle: null,
    ...overrides,
  };
}

describe('Preview', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  afterEach(async () => {
    if (app) {
      await unmount(app);
      app = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
  });

  function render(data: PreviewData): HTMLElement {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(Preview, { target: host, props: { data } }) as unknown as Record<string, never>;
    return host;
  }

  it('renders the add-style tile with usage overlay and text', () => {
    const el = render(base());
    expect(el.querySelector('#button-element_p1')).not.toBeNull();
    const img = el.querySelector('#button-image_p1') as HTMLImageElement;
    expect(img.hasAttribute('src')).toBe(false);
    expect(img.getAttribute('alt')).toBe('');
    expect(img.style.width).toBe('59px');
    expect(el.querySelector('#usage-title_p1')).not.toBeNull();
    expect(el.querySelector('#usage-value_p1')).not.toBeNull();
    expect(el.querySelector('#button-text-preview_p1')?.textContent?.trim()).toBe('Name');
  });

  it('omits the button id and usage block for the empty edit tile', () => {
    const el = render(base({ buttonId: false, usageFill: null, text: 'Button name' }));
    expect(el.querySelector('#button-element_p1')).toBeNull();
    expect(el.querySelector('.wd_button')).not.toBeNull();
    expect(el.querySelector('.usage')).toBeNull();
  });

  it('renders src images with optional alt and self-removal', () => {
    const el = render(
      base({
        media: {
          kind: 'img',
          src: 'static/img/x.png',
          alt: 'static/img/x.png',
          removeOnError: true,
          widthPx: 87,
          fill: 'fill:red; color:red;',
        },
      })
    );
    const img = el.querySelector('#button-image_p1') as HTMLImageElement;
    expect(img.getAttribute('src')).toBe('static/img/x.png');
    expect(img.getAttribute('alt')).toBe('static/img/x.png');
    img.dispatchEvent(new Event('error'));
    expect(el.querySelector('#button-image_p1')).toBeNull();
  });

  it('omits alt when the variant has none', () => {
    const el = render(
      base({ media: { kind: 'img', src: 's', alt: null, removeOnError: false, widthPx: 1, fill: '' } })
    );
    expect((el.querySelector('#button-image_p1') as HTMLImageElement).hasAttribute('alt')).toBe(false);
  });

  it('passes svg slots through and styles the buttontext', () => {
    const el = render(
      base({
        media: { kind: 'svg', slot: 7 },
        usageFill: null,
        textStyle: 'color:#fff;',
      })
    );
    expect(el.querySelector('span[data-svg-slot="7"]')).not.toBeNull();
    expect((el.querySelector('#button-text-preview_p1') as HTMLElement).style.color).toBe('#fff');
  });
});

describe('previewImageLink', () => {
  it('maps every style-image form to a URL', () => {
    expect(previewImageLink('http://x/y.png')).toBe('http://x/y.png');
    expect(previewImageLink('C:\\a\\b.png')).toBe('static/img/b.png');
    expect(previewImageLink('**uploaded/a.png')).toBe('.config/user_uploads/a.png');
    expect(previewImageLink('a.png')).toBe('static/img/a.png');
  });
});
