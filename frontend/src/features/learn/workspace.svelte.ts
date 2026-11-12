import { tick } from 'svelte';
import type { Button, Layout } from '../../lib/contracts';
import type { Translator } from '../../lib/translations';
import { clone } from '../editor/editor.svelte';
import type { EditorFlavor, GuideFrame, GuideStep, SettingsPage } from './guides';

const folderId = 'webdeck-tutorial-folder';
const imageId = 'webdeck-tutorial-image.svg';
const svg =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" rx="20" fill="#6654e8"/><path d="M28 70V30h44v40zM35 60l12-14 10 10 8-8" fill="none" stroke="white" stroke-width="5" stroke-linejoin="round"/><circle cx="63" cy="38" r="4" fill="white"/></svg>';
export const tutorialImages = { [imageId]: `data:image/svg+xml,${encodeURIComponent(svg)}` };
export type WorkspaceState = {
  folder: string;
  panel: 'deck' | 'settings';
  editing: boolean;
  settingsTab: string;
};

/** Navigation and disposable examples only; this adapter never mutates the deck editor. */
export class GuideWorkspace {
  frame = $state<GuideFrame | null>(null);
  step = $state('');
  button = $state<Button | null>(null);
  layout = $state<Layout | undefined>();
  tab = $state('content');
  audio = $state.raw<AudioBuffer | undefined>();
  private snapshot: WorkspaceState | undefined;
  private examples = new Map<EditorFlavor, Button>();
  private flavor: EditorFlavor | undefined;
  private folders: Layout | undefined;
  constructor(
    private options: {
      get: () => WorkspaceState;
      set: (state: WorkspaceState) => void;
      layout: () => Layout | undefined;
      controls: (mode: 'controls' | 'shortcuts' | null) => void;
      translate: Translator;
    },
  ) {}

  async prepare(step: GuideStep, signal: AbortSignal) {
    if (signal.aborted) return;
    this.snapshot ??= this.options.get();
    this.closeStep();
    const frame = step.frame;
    if (this.button && this.flavor) this.examples.set(this.flavor, this.button);
    this.frame = frame;
    this.step = step.id;
    this.layout = undefined;
    const state = { ...this.snapshot };
    this.options.controls(null);
    if (frame.surface === 'settings') {
      state.panel = 'settings';
      state.settingsTab = frame.page as SettingsPage;
    } else if (
      frame.surface === 'deck' ||
      frame.surface === 'controls' ||
      frame.surface === 'editor'
    ) {
      state.panel = 'deck';
      if (frame.surface === 'deck') state.editing = frame.editing ?? false;
    }
    if (frame.surface === 'deck' && frame.exampleFolders) {
      this.folders ??= this.exampleFolders();
      this.layout = this.folders;
      state.folder = ['child', 'rename'].includes(frame.detail ?? '')
        ? folderId
        : this.folders.folders[0]!.id;
    }
    if (frame.surface === 'editor') {
      const flavor = frame.flavor ?? 'basic';
      this.button = this.examples.get(flavor) ?? this.exampleButton(flavor);
      this.flavor = flavor;
      this.tab = frame.tab ?? 'action';
      if (flavor === 'audio' && !this.audio) this.audio = this.exampleAudio();
    } else {
      this.button = null;
      this.flavor = undefined;
    }
    this.options.set(state);
    if (frame.surface === 'controls')
      this.options.controls(frame.detail === 'shortcuts' ? 'shortcuts' : 'controls');
    await tick();
    if (signal.aborted) return;
    if (frame.surface === 'deck' && frame.detail === 'folder-menu')
      this.open('[data-guide="folder-actions"]');
    if (frame.surface === 'deck' && frame.detail === 'rename')
      this.open('[data-guide="rename-folder"]');
    await tick();
  }

  private open(selector: string) {
    const button = document.querySelector<HTMLButtonElement>(selector);
    if (button && button.getAttribute('aria-expanded') !== 'true') button.click();
  }

  private closeStep() {
    if (this.frame?.detail === 'rename')
      document.querySelector<HTMLButtonElement>('[data-rename-cancel]')?.click();
    if (this.frame?.detail === 'folder-menu')
      document
        .querySelector<HTMLButtonElement>('[data-guide="folder-actions"][aria-expanded="true"]')
        ?.click();
  }

  async restore() {
    this.closeStep();
    const snapshot = this.snapshot;
    this.frame = null;
    this.button = null;
    this.layout = undefined;
    this.audio = undefined;
    this.examples.clear();
    this.folders = undefined;
    this.flavor = undefined;
    this.snapshot = undefined;
    this.options.controls(null);
    if (snapshot) this.options.set(snapshot);
    await tick();
  }

  private exampleFolders(): Layout {
    const layout = clone(this.options.layout()!);
    const root = layout.folders[0]!;
    root.buttons.push({
      id: `${folderId}-link`,
      label: this.options.translate('ui_guide_sample_folder'),
      icon: 'icon:folder',
      color: '#6654e8',
      action: { type: 'folder', folder_id: folderId },
      extensions: {},
    });
    layout.folders.push({
      id: folderId,
      label: this.options.translate('ui_guide_sample_folder'),
      extensions: {},
      buttons: [
        {
          id: `${folderId}-back`,
          label: this.options.translate('ui_back'),
          icon: 'icon:back',
          color: '#252936',
          action: { type: 'back' },
          extensions: {},
        },
      ],
    });
    return layout;
  }

  private exampleButton(flavor: EditorFlavor): Button {
    const button: Button = {
      id: 'webdeck-tutorial-button',
      label: this.options.translate('ui_guide_sample_button'),
      icon: 'icon:grid',
      color: '#6654e8',
      action: { type: 'command', command: { type: 'open', target: 'https://example.com' } },
      extensions: { appearance: { columns: 1, rows: 1, icon_size: 42 } },
    };
    if (flavor === 'key')
      button.action = { type: 'command', command: { type: 'key', keys: ['ctrl', 'k'] } };
    if (flavor === 'audio') {
      button.icon = 'icon:music';
      button.action = {
        type: 'command',
        command: {
          type: 'play_sound',
          source: { type: 'asset', id: 'webdeck-tutorial.wav' },
          volume: 65,
          output_device: '',
          microphone: false,
          local_only: true,
          start_ms: 2000,
          end_ms: 8500,
        },
      };
      button.extensions.media = {
        audio: { source: 'webdeck-tutorial.wav', name: 'example.wav', size: 192044 },
      };
    }
    if (flavor === 'images') {
      button.icon = `asset:${imageId}`;
      button.extensions.media = {
        image: { source: imageId, name: 'example.svg', size: svg.length },
      };
    }
    if (flavor === 'workflow')
      button.action = {
        type: 'workflow',
        workflow: {
          type: 'sequence',
          steps: [
            { type: 'command', command: { type: 'debug', data: { example: true } } },
            { type: 'delay', milliseconds: 500 },
          ],
        },
      };
    if (flavor === 'plugin')
      button.action = {
        type: 'command',
        command: {
          type: 'plugin',
          plugin_id: 'example.plugin',
          version: '1.0.0',
          action_id: 'example',
          args: {},
        },
      };
    if (flavor === 'folder')
      button.action = {
        type: 'folder',
        folder_id: this.options.layout()?.folders[0]?.id ?? 'home',
      };
    return button;
  }

  private exampleAudio(): AudioBuffer {
    const context = new OfflineAudioContext(1, 1, 8000);
    const buffer = context.createBuffer(1, 96000, 8000);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < data.length; i++)
      data[i] = 0.16 * Math.sin((2 * Math.PI * 330 * i) / 8000) * (0.6 + 0.4 * Math.sin(i / 1700));
    return buffer;
  }
}
