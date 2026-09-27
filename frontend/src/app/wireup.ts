import { text } from '../framework/i18n';
import { asBool, asString, get, type BootContext } from '../framework/types';
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
import { pageState, socketHolder, type SocketIoClient } from './state';
import { showError } from './toast';
import { updateUsageTiles } from './usage';
import { auto_resize, wireZoomControls } from './zoom';
import { loadConfig, send_data } from './send';

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
  if (transferMethod !== 'socket') return;
  // NOTE: upstream dereferences `io` unguarded (the script 404s, so socket
  // mode aborts page wiring entirely there); degrade gracefully instead.
  const client = (window as unknown as Record<string, unknown>)['io'] as
    | SocketIoClient
    | undefined;
  if (!client) {
    console.error('socket.io client missing; socket mode unavailable');
    return;
  }
  const socket = client.connect('http://' + document.domain + ':' + location.port);
  socketHolder.socket = socket;

  socket.on('connect', function () {
    console.log('Connected');
  });

  socket.on('json_data', function (data) {
    console.log((data as { message?: unknown } | undefined)?.message);
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
            console.log('sending config-form...');

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

            console.log(config_dataTemp);

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

            console.log(config_data);

            fetch('/save_config', {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify(config_data),
            })
              .then(function (response) {
                if (response.ok) {
                  return response.json();
                } else {
                  throw new Error(text('settings_save_error'));
                }
              })
              .then(function (response: { success?: boolean; message?: string }) {
                if (response.success) {
                  alert(text('settings_save_success'));
                } else {
                  if (response.message && response.message !== '') {
                    showError(response.message);
                    alert(response.message);
                  } else {
                    showError('Error :/');
                    alert(text('settings_save_error'));
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
            console.log('reloading...');
            location.reload();
          } else {
            fetch('/usage', {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({ message }),
            })
              .then((response) => response.json())
              .then((usage_dict) => {
                updateUsageTiles(usage_dict);
              })
              .catch((error) => console.error(error));
          }
        }
      });
    });
}

function wireKeydown(): void {
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
          console.log('Ctrl + Z');
          undoSwap();
        }
        if (event.ctrlKey && event.shiftKey && event.key === 'Z' && isSwapMode()) {
          console.log('Ctrl + Shift + Z');
          undoUNSwap();
        }
        if (event.key.toLowerCase() === 'y' && event.ctrlKey && isSwapMode()) {
          console.log('Ctrl + Y');
          undoUNSwap();
        }
      }
    }
    if (event.key === 'Escape') {
      hide_last_modal();
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

  loadConfig()
    .then(function (configData) {
      pageState.tempEditorConfig = configData;
    })
    .catch(function (error) {
      console.error(error);
    });

  wireVideos();
  wireEditorChrome();
  reloadEditorEvents();
  wireSocket(transferMethod);
  wireSubmits(transferMethod);
  wireModals(ctx, () => toggleEditorMode(), isSwapMode);
  wireKeydown();
  wireZoomControls(
    isSwapMode,
    asString(get(ctx.config, 'front', 'width')),
    asString(get(ctx.config, 'front', 'height'))
  );

  void asBool;
}
