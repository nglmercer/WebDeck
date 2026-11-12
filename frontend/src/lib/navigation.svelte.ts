import type { Layout } from './contracts';
export class Navigation {
  active = $state('home');
  panel = $state<'deck' | 'settings'>('deck');
  constructor(
    private layout: () => Layout | undefined,
    private canEdit: () => boolean,
    private settingsLinks: {
      section: (section: string) => void;
      tutorials: () => void;
    } = { section: () => {}, tutorials: () => {} },
  ) {}
  get root() {
    return this.layout()?.folders[0]?.id ?? 'home';
  }
  navigate(folder: string, replace = false) {
    if (!this.layout()?.folders.some((item) => item.id === folder)) return;
    this.active = folder;
    this.panel = 'deck';
    this.write(`#folder=${encodeURIComponent(folder)}`, replace);
  }
  route() {
    const query = new URLSearchParams(location.search);
    if (
      (location.hash === '#settings' || (!location.hash && query.get('view') === 'settings')) &&
      this.canEdit()
    ) {
      this.panel = 'settings';
      if (!location.hash) {
        const section = query.get('section') ?? 'appearance';
        this.settingsLinks.section(
          ['appearance', 'integrations', 'devices', 'backups', 'runtime'].includes(section)
            ? section
            : 'appearance',
        );
        if (section === 'tutorials') this.settingsLinks.tutorials();
      }
      return;
    }
    this.panel = 'deck';
    let folder = this.root;
    try {
      if (location.hash.startsWith('#folder=')) folder = decodeURIComponent(location.hash.slice(8));
    } catch {
      folder = this.root;
    }
    this.active = this.layout()?.folders.some((item) => item.id === folder) ? folder : this.root;
  }
  back() {
    this.navigate(this.root);
  }
  settings() {
    if (!this.canEdit()) return;
    this.panel = 'settings';
    this.write('#settings');
  }
  private write(hash: string, replace = false) {
    const url = new URL(location.href);
    url.searchParams.delete('view');
    url.searchParams.delete('section');
    url.hash = hash;
    if (replace) history.replaceState(null, '', url);
    else history.pushState(null, '', url);
  }
}
