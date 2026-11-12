import { describe, expect, it } from 'vitest';
import { availableGuides, guides, guideSteps } from './guides';
import english from '../../../../webdeck/translations/en_US.json';
import spanish from '../../../../webdeck/translations/es_ES.json';

const required = [
  'overview',
  'controls',
  'editing',
  'content',
  'button-appearance',
  'actions',
  'keyboard',
  'audio',
  'appearance',
  'integrations',
  'connections',
  'network',
  'backups',
  'runtime',
  'pairing',
  'practice',
];

describe('tutorial coverage contract', () => {
  it('covers every audited workflow with distinct, anchored steps in both languages', () => {
    expect(guides.map((guide) => guide.id).sort()).toEqual(required.sort());
    for (const guide of guides) {
      expect(guide.steps.length).toBeGreaterThanOrEqual(3);
      expect(new Set(guide.steps.map((step) => step.id)).size).toBe(guide.steps.length);
      for (const key of [
        guide.title,
        guide.description,
        ...guide.steps.flatMap((step) => [step.title, step.description]),
      ]) {
        expect((english as Record<string, string>)[key], key).toBeTruthy();
        expect((spanish as Record<string, string>)[key], key).toBeTruthy();
      }
      for (const step of guide.steps) {
        expect(step.target).toBeTruthy();
        expect(['deck', 'controls', 'settings', 'editor', 'pairing', 'practice']).toContain(
          step.frame.surface,
        );
        if (step.interactive) expect(['editor', 'practice']).toContain(step.frame.surface);
      }
    }
  });
  it('restricts viewers and unpaired clients to guides their screens support', () => {
    expect(availableGuides(false, false).map((guide) => guide.id)).toEqual([
      'overview',
      'controls',
      'practice',
    ]);
    expect(availableGuides(false, true).map((guide) => guide.id)).toEqual(['pairing', 'practice']);
    const overview = guides.find((guide) => guide.id === 'overview')!;
    expect(guideSteps(overview, false, true).map((step) => step.id)).toEqual([
      'deck',
      'folders',
      'replay',
    ]);
    expect(guideSteps(overview, true, false).map((step) => step.id)).not.toContain('folders');
  });
});
