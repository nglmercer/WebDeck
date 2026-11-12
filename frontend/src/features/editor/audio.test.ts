import { describe, expect, it } from 'vitest';
import { formatTime, parseTime, waveform } from './audio';
import { commandSchema, defaultValue } from '../../lib/schema';

describe('audio fragment editing', () => {
  it('round trips millisecond boundaries, long clips and comma input', () => {
    for (const time of [0, 1, 2500, 8700, 12400, 59999, 3600000]) {
      expect(parseTime(formatTime(time))).toBe(time);
    }
    expect(parseTime('00:02,5')).toBe(2500);
    for (const time of ['', '-1:00', '00:60', '00:02.9999', 'abc', 'Infinity:00']) {
      expect(parseTime(time)).toBeNull();
    }
  });

  it('shows peaks from every channel, including short and silent audio', () => {
    const channels = [new Float32Array(8), new Float32Array([0, 0, 0, 0, 0, 0.75, 0, 0])];
    const buffer = {
      numberOfChannels: 2,
      getChannelData: (channel: number) => channels[channel],
    } as AudioBuffer;
    expect(waveform(buffer, 4)).toEqual([0.025, 0.025, 1, 0.025]);
    expect(waveform(buffer, 12).every(Number.isFinite)).toBe(true);
  });

  it('defaults to the whole file without inventing optional trim endpoints', () => {
    const schema = commandSchema.oneOf!.find(
      (option) => option.properties?.type?.const === 'play_sound',
    )!;
    const command = defaultValue(schema) as Record<string, unknown>;
    expect(command).not.toHaveProperty('start_ms');
    expect(command).not.toHaveProperty('end_ms');
  });
});
