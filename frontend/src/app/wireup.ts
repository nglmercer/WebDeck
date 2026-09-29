import { saveConfig } from '../api/config';
import { fetchUsage } from '../api/usage';
import { text } from '../framework/i18n';
import { asString, get, type BootContext } from '../framework/types';
import { q, byId } from '../query';
import {
  SaveExitEditor,
  isSwapMode,
  reloadEditorEvents,
  swapEditorButtonFunction,
  toggleEditorMode,
  undoSwap,
  undoUNSwap,
  wireEditorChrome,
} from './editor';
import {
  hide_last_modal,
  hide_modal,
  isAddbuttonModalOpened,
  isEditbuttonModalOpened,
  show_modal,
  wireModals,
} from './modals';
import { io } from 'socket.io-client';
import { pageState, socketHolder, type AppSocket } from './state';
import { emitAppEvent } from './events';
import { refreshApp } from './refresh';
import { showError } from './toast';
import { showAlert } from '../components/dialog';
import { updateUsageTiles } from './usage';
import { auto_resize, wireZoomControls } from './zoom';
import { send_data } from './send';

/** Global helpers (top-level functions in the inline script). */
export function installGlobals(): void {
  window.folder = function (folder_id: string): void {
    q('.buttons-center')
      .toArray()
      .forEach(function (element) {
        if (!q(element).hasClass('invisible')) {
          q(element).addClass('invisible');
        }
      });

    const folderElement = byId('folder-' + folder_id).get(0) ?? null;
    if (!folderElement) return;
    if (q(folderElement).hasClass('invisible')) {
      q(folderElement).removeClass('invisible');
    } else {
      q(folderElement).addClass('invisible');
    }
  };

  window.togglePasswordVisibility = function (id: string, iconid: string): void {
    const passwordInput = byId<HTMLInputElement>(id).get(0) ?? null;
    const showPasswordIcon = byId(iconid).get(0) ?? null;
    if (!passwordInput || !showPasswordIcon) return;
    if (q(passwordInput).prop('type') === 'password') {
      q(passwordInput).prop('type', 'text');
      q(showPasswordIcon).addClass('active');
    } else {
      q(passwordInput).prop('type', 'password');
      q(showPasswordIcon).removeClass('active');
    }
  };

  window.send_data = send_data;
}

function wireVideos(): void {
  const videos = q('video');
  videos.toArray().forEach((video) => {
    q(video).on('loadedmetadata', () => {
      videos.toArray().forEach((otherVideo) => {
        if (otherVideo !== video) {
          q(otherVideo).prop('currentTime', 0);
        }
      });
    });
  });
}

function wireSocket(transferMethod: string): void {
  // Re-render safe: drop the previous connection (if any) so refreshes
  // never stack sockets, and a config change away from socket mode
  // disconnects cleanly.
  socketHolder.socket?.disconnect();
  socketHolder.socket = null;
  if (transferMethod !== 'socket') return;
  const socket: AppSocket = io('http://' + document.domain + ':' + location.port);
  socketHolder.socket = socket;

  socket.on('connect', function () {
    console.log('Connected');
  });
}

function wireSubmits(transferMethod: string): void {
  q('form')
    .toArray()
    .forEach((form) => {
      q(form).on('submit', function (event) {
        event.preventDefault();

        if (pageState.editorMode === 1 && isSwapMode()) {
          return;
        }

        const messageElement = q(form).find('.message').get(0) ?? null;
        if (!messageElement) {
          if (q(form).hasClass('config-form')) {
            const config_dataTemp: Record<string, unknown> = {};
            q('#config-form input, #config-form select')
              .toArray()
              .forEach(function (input) {
                const el = input as HTMLInputElement | HTMLSelectElement;
                const field = el as HTMLInputElement;
                const name = q(el).prop('name') ?? '';
                let value: unknown;
                if (q(field).prop('type') === 'checkbox') {
                  value = q(field).prop('checked') === true;
                } else {
                  value = String(q(field).val() ?? '');
                  if (q(el).prop('id') === 'language') {
                    value = String(value).toLowerCase();
                  }
                }
                config_dataTemp[name] = value;
              });

            const config_data: Record<string, unknown> = {};

            for (const key in config_dataTemp) {
              if (key === '') continue;

              const keys = key.split('.');
              let obj: Record<string, unknown> = config_data;

              for (let i = 0; i < keys.length; i++) {
                const k = keys[i];
                if (k === undefined) continue;
                if (!Object.prototype.hasOwnProperty.call(obj, k)) {
                  obj[k] = {};
                }

                if (i === keys.length - 1) {
                  obj[k] = config_dataTemp[key];
                }

                obj = obj[k] as Record<string, unknown>;
              }
            }

            saveConfig(config_data)
              .then(function (response) {
                if (response.success) {
                  emitAppEvent('save:completed', { flow: 'config' });
                  // Re-render so grid size, language, theme, and background
                  // apply immediately (no manual reload needed).
                  void refreshApp();
                  void showAlert(text('settings_save_success'));
                } else {
                  // Single notification per failure (was toast + alert together).
                  if (response.message && response.message !== '') {
                    void showAlert(response.message);
                  } else {
                    void showAlert(text('settings_save_error'));
                  }
                }
              })
              .catch(function (error: Error) {
                showError(error.message);
              });
          }
        } else {
          const message = String(q(messageElement).val() ?? '');
          if (
            !message.startsWith('/usage') &&
            !message.startsWith('/reload') &&
            !message.startsWith('/folder')
          ) {
            if (transferMethod === 'socket') {
              socketHolder.socket?.emit('message_from_socket', message);
            } else {
              send_data(message);
            }
          } else if (message.startsWith('/reload') && !isSwapMode()) {
            location.reload();
          } else {
            fetchUsage({ message })
              .then((usage_dict) => {
                updateUsageTiles(usage_dict);
              })
              .catch((error) => console.error(error));
          }
        }
      });
    });
}

// The keydown listener binds <html>, which survives re-renders: wire it
// exactly once, or every refresh would stack another toggle handler.
let keydownWired = false;

function wireKeydown(): void {
  if (keydownWired) return;
  keydownWired = true;
  // Keydowns bubble through <html>, like the document listener did.
  q('html').on('keydown', function (event) {
    // NOTE: opacity probes read the *inline* style; qdom's `.css()` getter
    // is computed-only, so these reads stay native.
    const modalOpacity = byId<HTMLElement>('modal-container').get(0)?.style.opacity;
    if (
      isEditbuttonModalOpened() === 0 &&
      isAddbuttonModalOpened() === 0 &&
      modalOpacity !== '1'
    ) {
      if (event.key.toLowerCase() === 'e') {
        SaveExitEditor(pageState.tempEditorConfig);
      }
      if (event.key.toLowerCase() === 'q') {
        toggleEditorMode();
      }
      if (pageState.editorMode === 1) {
        if (event.key.toLowerCase() === 's') {
          swapEditorButtonFunction();
        }
        if (event.key.toLowerCase() === 'z' && event.ctrlKey && isSwapMode()) {
          undoSwap();
        }
        if (event.ctrlKey && event.shiftKey && event.key === 'Z' && isSwapMode()) {
          undoUNSwap();
        }
        if (event.key.toLowerCase() === 'y' && event.ctrlKey && isSwapMode()) {
          undoUNSwap();
        }
      }
    }
    if (event.key === 'Escape') {
      const noModalOpen =
        isEditbuttonModalOpened() === 0 &&
        isAddbuttonModalOpened() === 0 &&
        modalOpacity !== '1';
      if (noModalOpen && pageState.editorMode === 1 && isSwapMode()) {
        swapEditorButtonFunction();
      } else {
        hide_last_modal();
      }
    }
    const modal = q<HTMLElement>('.modal-container').get(0) ?? null;

    const focusedElement = document.activeElement as HTMLElement | null;
    if (!(focusedElement?.tagName === 'INPUT' || focusedElement?.tagName === 'TEXTAREA')) {
      if (event.key.toLowerCase() === 'c' || event.key.toLowerCase() === 'm') {
        if (isEditbuttonModalOpened() === 0 && isAddbuttonModalOpened() === 0) {
          if (modal && modal.style.opacity !== '1') {
            show_modal();
          } else if (modal) {
            hide_modal();
          }
        }
      }
    }
  });
}

/** DOMContentLoaded body, in upstream registration order. */
export function wireApp(ctx: BootContext): void {
  const transferMethod = asString(get(ctx.config, 'settings', 'data_transfer_method'));

  // NOTE: tempEditorConfig is seeded synchronously from the boot payload
  // in renderApp (both boot and refresh) — no /get_config fetch here.
  // Editor entry still takes its own fresh copy (toggleEditorMode).

  wireVideos();
  // NOTE: collapse persistence/link handling lives in Collapse.svelte
  // ($effect per section); no global wiring needed (and double-wiring
  // would open tutorial links twice).
  wireEditorChrome();
  reloadEditorEvents();
  wireSocket(transferMethod);
  wireSubmits(transferMethod);
  wireModals(() => toggleEditorMode(), isSwapMode);
  wireKeydown();
  wireZoomControls(isSwapMode, asString(get(ctx.config, 'front', 'width')));
}
