/** A display suggestion only. Browser hints never establish a device's identity. */
export type DeviceNavigator = {
  userAgent: string;
  platform?: string;
  maxTouchPoints?: number;
  userAgentData?: {
    platform?: string;
    getHighEntropyValues?: (hints: string[]) => Promise<{ model?: string }>;
  };
};
const clean = (value: string) =>
  value
    .replace(/[\u0000-\u001f\u007f]/g, '')
    .trim()
    .slice(0, 128);

export function browserDeviceName(source: DeviceNavigator): string {
  const ua = source.userAgent;
  const platform =
    /iPad/i.test(ua) || (source.platform === 'MacIntel' && (source.maxTouchPoints ?? 0) > 1)
      ? 'iPad'
      : /iPhone/i.test(ua)
        ? 'iPhone'
        : /Android/i.test(ua)
          ? 'Android'
          : /Windows/i.test(ua)
            ? 'Windows'
            : /Macintosh|Mac OS X/i.test(ua)
              ? 'macOS'
              : /CrOS/i.test(ua)
                ? 'ChromeOS'
                : /Linux/i.test(ua)
                  ? 'Linux'
                  : clean(source.userAgentData?.platform ?? '');
  const browser = /Edg(?:e|A|iOS)?\//i.test(ua)
    ? 'Edge'
    : /(?:OPR|OPiOS)\//i.test(ua)
      ? 'Opera'
      : /SamsungBrowser\//i.test(ua)
        ? 'Samsung Internet'
        : /(?:Firefox|FxiOS)\//i.test(ua)
          ? 'Firefox'
          : /(?:Chrome|CriOS)\//i.test(ua)
            ? 'Chrome'
            : /Safari\//i.test(ua)
              ? 'Safari'
              : '';
  return [browser, platform].filter(Boolean).join(' · ');
}

export async function suggestDeviceName(
  source: DeviceNavigator,
  fallback: string,
): Promise<string> {
  const generic = browserDeviceName(source) || fallback;
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    if (!source.userAgentData?.getHighEntropyValues) return generic;
    const hints = await Promise.race([
      source.userAgentData.getHighEntropyValues(['model']),
      new Promise<undefined>((resolve) => {
        timer = setTimeout(() => resolve(undefined), 400);
      }),
    ]);
    // Reduced Android user agents often use "K" as a placeholder, not a model.
    const model = clean(hints?.model ?? '');
    return model && model !== 'K' ? model : generic;
  } catch {
    return generic;
  } finally {
    clearTimeout(timer);
  }
}
