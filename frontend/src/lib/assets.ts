/** Owns asset URLs for one mounted application. Stale loads never publish URLs. */
export class AssetCache {
  private urls = new Map<string, string>();
  private generation = 0;
  private disposed = false;
  private active = 0;
  private waiting: (() => void)[] = [];

  private async acquire() {
    if (this.active < 4) {
      this.active++;
      return;
    }
    await new Promise<void>((resolve) => this.waiting.push(resolve));
  }
  private release() {
    const next = this.waiting.shift();
    if (next)
      next(); // Transfer the occupied slot directly to the next waiter.
    else this.active--;
  }

  constructor(
    private fetchAsset: (id: string) => Promise<string>,
    private revoke: (url: string) => void = URL.revokeObjectURL,
  ) {}

  async load(ids: string[]): Promise<{ urls: Record<string, string>; missing: string[] } | null> {
    const generation = ++this.generation;
    const unique = [...new Set(ids)];
    const next = new Map<string, string>();
    const created: string[] = [];
    const missing: string[] = [];
    let cursor = 0;
    const worker = async () => {
      while (cursor < unique.length) {
        if (this.disposed || generation !== this.generation) return;
        const id = unique[cursor++]!;
        const cached = this.urls.get(id);
        if (cached) {
          next.set(id, cached);
          continue;
        }
        await this.acquire();
        try {
          if (this.disposed || generation !== this.generation) return;
          const url = await this.fetchAsset(id);
          created.push(url);
          next.set(id, url);
        } catch {
          missing.push(id);
        } finally {
          this.release();
        }
      }
    };
    await Promise.all(Array.from({ length: Math.min(4, unique.length) }, worker));
    if (this.disposed || generation !== this.generation) {
      created.forEach((url) => this.revoke(url));
      return null;
    }
    for (const [id, url] of this.urls) if (!next.has(id)) this.revoke(url);
    this.urls = next;
    return { urls: Object.fromEntries(next), missing };
  }

  dispose() {
    this.disposed = true;
    this.generation++;
    this.urls.forEach((url) => this.revoke(url));
    this.urls.clear();
  }
}
