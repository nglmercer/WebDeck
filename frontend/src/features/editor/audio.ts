export function formatTime(milliseconds: number): string {
  const ms = Math.max(0, Math.round(milliseconds));
  return `${String(Math.floor(ms / 60000)).padStart(2, '0')}:${String(Math.floor(ms / 1000) % 60).padStart(2, '0')}.${String(ms % 1000).padStart(3, '0')}`;
}

export function parseTime(value: string): number | null {
  const match = /^(\d+):([0-5]\d)(?:[.,](\d{1,3}))?$/.exec(value.trim());
  if (!match) return null;
  const result =
    Number(match[1]) * 60000 + Number(match[2]) * 1000 + Number((match[3] ?? '').padEnd(3, '0'));
  return Number.isSafeInteger(result) ? result : null;
}

export function waveform(buffer: AudioBuffer, count = 120): number[] {
  const peaks = Array.from({ length: count }, () => 0);
  for (let channel = 0; channel < buffer.numberOfChannels; channel++) {
    const samples = buffer.getChannelData(channel);
    for (let i = 0; i < count; i++) {
      const end = Math.floor(((i + 1) * samples.length) / count);
      for (let j = Math.floor((i * samples.length) / count); j < end; j++) {
        peaks[i] = Math.max(peaks[i]!, Math.abs(samples[j]!));
      }
    }
  }
  const maximum = Math.max(...peaks, 0.001);
  return peaks.map((peak) => Math.max(0.025, peak / maximum));
}

export async function decodeAudio(bytes: ArrayBuffer): Promise<AudioBuffer> {
  const context = new OfflineAudioContext(1, 1, 44100);
  const buffer = await context.decodeAudioData(bytes);
  if (!Number.isFinite(buffer.duration) || buffer.duration <= 0)
    throw new Error('audio.unreadable');
  return buffer;
}

export type MediaMetadata = { source: string; name: string; size?: number };
export const sourceKey = (
  source: { type: 'asset'; id: string } | { type: 'external'; path: string },
) => (source.type === 'asset' ? source.id : source.path);
export const fileSize = (size: number) =>
  size < 1024
    ? `${size} B`
    : size < 1024 * 1024
      ? `${Math.round(size / 1024)} KB`
      : `${(size / 1024 / 1024).toFixed(1)} MB`;
