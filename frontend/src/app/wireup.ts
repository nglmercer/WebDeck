import { text } from '../framework/i18n';
import { asBool, asString, get, type BootContext } from '../framework/types';
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
import { pageState, socketHolder } from './state';
import { showError } from './toast';
import { updateUsageTiles } from './usage';
import { auto_resize, wireZoomControls } from './zoom';
import { loadConfig, send_data } from './send';

/** Global helpers (top-level functions in the inline script). */
export function installGlobals(): void {
  window.folder = function (folder_id: string): void {
    const elements = document.querySelectorAll('.buttons-center');
    elements.forEach(function (element) {
      if (!element.classList.contains('invisible')) {
        element.classList.add('invisible');
      }
    });

    const folderElement = document.getElementById('folder-' + folder_id);
    if (!folderElement) return;
    if (folderElement.classList.contains('invisible')) {
      folderElement.classList.remove('invisible');
    } else {
      folderElement.classList.add('invisible');
    }
  };

  window.togglePasswordVisibility = function (id: string, iconid: string): void {
    const passwordInput = document.getElementById(id) as HTMLInputElement | null;
    const showPasswordIcon = document.getElementById(iconid);
    if (!passwordInput || !showPasswordIcon) return;
    if (passwordInput.type === 'password') {
      passwordInput.type = 'text';
      showPasswordIcon.classList.add('active');
    } else {
      passwordInput.type = 'password';
      showPasswordIcon.classList.remove('active');
    }
  };

  window.send_data = send_data;
}

function wireVideos(): void {
  const videos = document.querySelectorAll('video');
  videos.forEach((video) => {
    video.addEventListener('loadedmetadata', () => {
      videos.forEach((otherVideo) => {
        if (otherVideo !== video) {
          otherVideo.currentTime = 0;
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
    | { connect: (url: string) => { on: (e: string, cb: (d?: unknown) => void) => void; emit: (e: string, d?: unknown) => void } }
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
  const forms = document.querySelectorAll('form');
  forms.forEach((form) => {
    form.addEventListener('submit', function (event) {
      event.preventDefault();

      if (pageState.editorMode === 1 && isSwapMode()) {
        return;
      }

      const messageElement = form.querySelector('.message') as HTMLInputElement | null;
      if (!messageElement) {
        if (form.classList.contains('config-form')) {
          console.log('sending config-form...');

          const config_dataTemp: Record<string, unknown> = {};
          const inputs = document.querySelectorAll('#config-form input, #config-form select');
          inputs.forEach(function (input) {
            const el = input as HTMLInputElement | HTMLSelectElement;
            const name = el.name;
            let value: unknown;
            if ((el as HTMLInputElement).type === 'checkbox') {
              value = (el as HTMLInputElement).checked ? true : false;
            } else {
              value = (el as HTMLInputElement).value;
              if (el.id === 'language') {
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
              if (!Object.prototype.hasOwnProperty.call(obj, keys[i])) {
                obj[keys[i] as string] = {};
              }

              if (i === keys.length - 1) {
                obj[keys[i] as string] = config_dataTemp[key];
              }

              obj = obj[keys[i] as string] as Record<string, unknown>;
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
        const message = messageElement.value;
        if (!message.startsWith('/usage') && !message.startsWith('/reload') && !message.startsWith('/folder')) {
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
  document.addEventListener('keydown', function (event) {
    if (
      isEditbuttonModalOpened() === 0 &&
      isAddbuttonModalOpened() === 0 &&
      (document.getElementById('modal-container') as HTMLElement | null)?.style.opacity !== '1'
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
    const modal = document.querySelector('.modal-container') as HTMLElement | null;

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
