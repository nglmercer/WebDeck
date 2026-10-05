import { createTranslator } from './translations';
import type { DeckBoot, CatalogResponse, TranslationsResponse, ConfigResponse } from './contracts';
import { boot, config, request, asset, connect, disconnect, ApiError } from './api/client';
import { AssetCache } from './assets';
import { Editor } from '../features/editor/editor.svelte';
export type SessionApi = {
  boot: () => Promise<DeckBoot>;
  config: () => Promise<ConfigResponse>;
  translations: () => Promise<TranslationsResponse>;
  catalog: () => Promise<CatalogResponse>;
  asset: (id: string) => Promise<string>;
  connect: () => void;
  disconnect: () => void;
};
const defaults: SessionApi = {
  boot,
  config,
  translations: () => request('translations', 'TranslationsResponse'),
  catalog: () => request('commands', 'CatalogResponse'),
  asset,
  connect,
  disconnect,
};
export class Session {
  deck = $state<DeckBoot | null>(null);
  editor = $state<Editor | null>(null);
  loading = $state(true);
  pairing = $state(false);
  translations = $state<Record<string, string>>({});
  languages = $state<string[]>([]);
  catalog = $state<CatalogResponse | null>(null);
  themeUrls = $state<string[]>([]);
  backgroundUrls = $state<string[]>([]);
  assetUrls = $state<Record<string, string>>({});
  private generation = 0;
  private refreshGeneration = 0;
  private cache: AssetCache;
  constructor(
    private feedback: { error: (message: string) => void; notice: (message: string) => void },
    private api = defaults,
    private confirmDiscard = () =>
      window.confirm(
        createTranslator(() => this.translations)('Discard unsaved changes and reload?'),
      ),
  ) {
    this.cache = new AssetCache(api.asset);
  }
  get layout() {
    return this.editor?.draft.layout ?? this.deck?.layout;
  }
  async load(): Promise<boolean> {
    if (this.editor?.saving) throw new Error('Wait for saving to finish before reloading.');
    if (this.editor?.dirty && !this.confirmDiscard()) return false;
    const generation = ++this.generation;
    this.refreshGeneration++;
    this.loading = true;
    try {
      const deck = await this.api.boot();
      const [translations, catalog, snapshot] = await Promise.all([
        this.api.translations(),
        this.api.catalog().catch((error: unknown) => {
          if (error instanceof ApiError && error.status === 401) throw error;
          if (generation === this.generation)
            this.feedback.error(error instanceof Error ? error.message : String(error));
          return null;
        }),
        deck.can_edit ? this.api.config() : Promise.resolve(null),
      ]);
      if (generation !== this.generation) return false;
      this.deck = deck;
      this.editor = snapshot ? new Editor(snapshot) : null;
      this.pairing = false;
      this.translations = translations.translations;
      this.languages = translations.languages;
      this.catalog = catalog;
      await this.assets();
      if (generation !== this.generation) return false;
      this.api.connect();
      return true;
    } catch (error) {
      if (generation !== this.generation) return false;
      if (error instanceof ApiError && error.status === 401) this.pairing = true;
      this.feedback.error(error instanceof Error ? error.message : String(error));
      return false;
    } finally {
      if (generation === this.generation) this.loading = false;
    }
  }
  async assets() {
    if (!this.layout) return;
    const themes = [...this.layout.themes],
      backgrounds = [...this.layout.backgrounds];
    const icons = this.layout.folders
      .flatMap((folder) => folder.buttons)
      .filter((button) => button.icon.startsWith('asset:'))
      .map((button) => button.icon.slice(6));
    const result = await this.cache.load([...themes, ...backgrounds, ...icons]);
    if (!result) return;
    this.themeUrls = themes.flatMap((id) => (result.urls[id] ? [result.urls[id]!] : []));
    this.backgroundUrls = backgrounds.flatMap((id) => (result.urls[id] ? [result.urls[id]!] : []));
    this.assetUrls = result.urls;
    if (result.missing.length) this.feedback.notice('Some deck assets are unavailable');
  }
  async refreshAfterSave() {
    const generation = this.generation;
    const refreshGeneration = ++this.refreshGeneration;
    try {
      const [deck, translations] = await Promise.all([this.api.boot(), this.api.translations()]);
      if (generation !== this.generation || refreshGeneration !== this.refreshGeneration) return;
      this.deck = deck;
      this.translations = translations.translations;
      this.languages = translations.languages;
      await this.assets();
    } catch {
      if (generation === this.generation && refreshGeneration === this.refreshGeneration)
        this.feedback.notice('Saved. Display refresh failed; reload when ready.');
    }
  }
  dispose() {
    this.generation++;
    this.refreshGeneration++;
    this.api.disconnect();
    this.cache.dispose();
  }
}
