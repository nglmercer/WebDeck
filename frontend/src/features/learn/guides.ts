import data from './guide-data.json';
import type { TranslationKey } from '../../lib/translations';

export type GuideId =
  | 'overview'
  | 'controls'
  | 'editing'
  | 'content'
  | 'button-appearance'
  | 'actions'
  | 'keyboard'
  | 'audio'
  | 'appearance'
  | 'integrations'
  | 'connections'
  | 'network'
  | 'backups'
  | 'runtime'
  | 'pairing'
  | 'practice';
export type SettingsPage = 'appearance' | 'integrations' | 'devices' | 'backups' | 'runtime';
export type EditorFlavor = 'basic' | 'images' | 'key' | 'audio' | 'workflow' | 'plugin' | 'folder';
export type GuideFrame = {
  surface: 'deck' | 'controls' | 'settings' | 'editor' | 'pairing' | 'practice';
  editing?: boolean;
  exampleFolders?: boolean;
  detail?: string;
  page?: SettingsPage;
  flavor?: EditorFlavor;
  tab?: 'content' | 'appearance' | 'action';
};
export type GuideStep = {
  id: string;
  target: string;
  title: TranslationKey;
  description: TranslationKey;
  frame: GuideFrame;
  admin?: boolean;
  optional?: 'folders';
  interactive?: boolean;
};
export type Guide = {
  id: GuideId;
  group: 'start' | 'buttons' | 'settings' | 'connection';
  access: 'all' | 'admin' | 'pairing';
  icon: string;
  title: TranslationKey;
  description: TranslationKey;
  steps: GuideStep[];
};
export const guides = data as Guide[];
export const availableGuides = (canEdit: boolean, pairing: boolean) =>
  guides.filter((guide) =>
    pairing
      ? ['pairing', 'practice'].includes(guide.id)
      : guide.access !== 'pairing' && (guide.access !== 'admin' || canEdit),
  );
export const guideSteps = (guide: Guide, canEdit: boolean, hasFolders: boolean) =>
  guide.steps.filter(
    (step) => (!step.admin || canEdit) && (step.optional !== 'folders' || hasFolders),
  );
