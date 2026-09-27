import { beforeEach, describe, expect, it } from 'vitest';
import type { BootContext } from '../framework/types';
import { applyHead } from './shell';

function testCtx(themes: string[]): BootContext {
  return {
    config: { front: { themes }, settings: {} },
    commands: {},
    versions: {},
    random_bg: '',
    usage_example: {},
    langs: [],
    svgs: [],
    themes: [],
    parsed_themes: {},
    is_exe: true,
    portrait_rotate: 0,
    lang: {},
    audio_devices: {},
    dark_theme: '',
  };
}

function stylesheetHrefs(): string[] {
  return [...document.querySelectorAll('link[rel="stylesheet"]')].map(
    (link) => (link as HTMLLinkElement).getAttribute('href') ?? ''
  );
}

beforeEach(() => {
  document.head.innerHTML = '';
});

describe('applyHead themes', () => {
  it('links the built-in stylesheet directly instead of prefixing .config/themes', () => {
    applyHead(testCtx(['static/css/style.css']));
    expect(stylesheetHrefs()).toEqual(['/static/css/style.css']);
  });

  it('keeps prefixing user themes and skipping commented entries', () => {
    applyHead(testCtx(['mytheme.css', '//static/css/style.css']));
    expect(stylesheetHrefs()).toEqual(['.config/themes/mytheme.css']);
  });

  it('loads the base theme before user overrides (cascade order)', () => {
    applyHead(testCtx(['mytheme.css', 'static/css/style.css']));
    expect(stylesheetHrefs()).toEqual(['/static/css/style.css', '.config/themes/mytheme.css']);
  });
});
