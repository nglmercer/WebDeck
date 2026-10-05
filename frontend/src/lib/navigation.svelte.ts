import type { Layout } from './contracts';
export class Navigation {
  active = $state('home');
  panel = $state<'deck' | 'settings'>('deck');
  constructor(
    private layout: () => Layout | undefined,
    private canEdit: () => boolean,
  ) {}
  get root() {
    return this.layout()?.folders[0]?.id ?? 'home';
  }
  navigate(folder: string, replace = false) {
    if (!this.layout()?.folders.some((item) => item.id === folder)) return;
    this.active = folder;
    this.panel = 'deck';
    const hash = `#folder=${encodeURIComponent(folder)}`;
    if (replace) history.replaceState(null, '', hash);
    else history.pushState(null, '', hash);
  }
  route() {
    if (location.hash === '#settings' && this.canEdit()) {
      this.panel = 'settings';
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
    history.pushState(null, '', '#settings');
  }
}
