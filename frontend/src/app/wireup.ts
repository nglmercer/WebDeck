import { deviceToken } from '../features/security/session';
import { showAlert } from '../components/dialog';
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
} from './modals';
import { io } from 'socket.io-client';
import { pageState, socketHolder, type AppSocket } from './state';
import { auto_resize, wireZoomControls } from './zoom';

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
  const socket: AppSocket = io(location.origin, { auth: { token: deviceToken() } });
  socketHolder.socket = socket;

  socket.on('command_error', (error) => { void showAlert(error.message); });
  socket.on('connect_error', () => { void showAlert('Connection denied. Check that this device token has not expired or been revoked.'); });
  socket.on('connect', function () {
    console.log('Connected');
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
  wireKeydown();
  wireZoomControls(isSwapMode, asString(get(ctx.config, 'front', 'width')));
}
