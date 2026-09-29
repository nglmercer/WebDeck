import { q, byId } from '../query';

// Modal open/close managers + URL params (index.jinja modal script block).

type ModalKind = 'config-modal' | 'addbutton-modal' | 'addbuttonArgs-modal' | 'editbutton-modal';

const lastModals: ModalKind[] = [];

const modalFlags = {
  is_addbutton_modal_opened: 0,
  is_addbutton_args_modal_opened: 0,
  is_editbutton_modal_opened: 0,
};

export function isAddbuttonModalOpened(): number {
  return modalFlags.is_addbutton_modal_opened;
}

export function isEditbuttonModalOpened(): number {
  return modalFlags.is_editbutton_modal_opened;
}

/** Reset transient modal state (what a page reload used to clear). */
export function resetModalState(): void {
  lastModals.length = 0;
  modalFlags.is_addbutton_modal_opened = 0;
  modalFlags.is_addbutton_args_modal_opened = 0;
  modalFlags.is_editbutton_modal_opened = 0;
}

function modal(): Element | null {
  return q('.modal-container').get(0) ?? null;
}

function addbuttonModal(): Element | null {
  return q('.addbutton-modal-container').get(0) ?? null;
}

/** Shared fade-out tick (opacity steps of 3.5 per 10ms, then hide). */
function fadeOut(el: Element, onHidden: () => void): void {
  const intervalId = setInterval(function () {
    const currentOpacity = parseFloat(q(el).css('opacity') ?? '');
    if (currentOpacity <= 0) {
      clearInterval(intervalId);
      setTimeout(function () {
        onHidden();
      }, 100);
      return;
    }
    q(el).css('opacity', (currentOpacity - 3.5).toFixed(2));
  }, 10);
}

export function show_modal(): void {
  lastModals.push('config-modal');
  const el = modal();
  if (!el) return;
  q(el).css({ opacity: '100%', display: 'block' });
}

export function hide_modal(): void {
  lastModals.splice(lastModals.indexOf('config-modal'), 1);
  const el = modal();
  if (!el) return;
  fadeOut(el, () => {
    q(el).css('display', 'none');
  });
}

export function show_addbutton_modal(addFolder: string | null, addId: string | null): void {
  lastModals.push('addbutton-modal');
  const el = addbuttonModal();
  if (!el) return;
  q(el).css({ opacity: '100%', display: 'block' });
  modalFlags.is_addbutton_modal_opened = 1;
  byId('addbutton-modal-content').attr('add_FOLDER', addFolder ?? '');
  byId('addbutton-modal-content').attr('add_ID', addId ?? '');
}

export function hide_addbutton_modal(): void {
  lastModals.splice(lastModals.indexOf('addbutton-modal'), 1);
  byId('addbutton-modal-content').removeAttr('add_ID');
  byId('addbutton-modal-content').removeAttr('add_FOLDER');
  const el = addbuttonModal();
  if (!el) return;
  fadeOut(el, () => {
    q(el).css('display', 'none');
  });
  modalFlags.is_addbutton_modal_opened = 0;
}

function show_addbutton_args_modal(modalId: string): void {
  lastModals.push('addbuttonArgs-modal');
  const el = byId('modal-container-' + modalId).get(0) ?? null;
  if (el) {
    q(el).css({ opacity: '100%', display: 'block' });
    modalFlags.is_addbutton_args_modal_opened = 1;
  }
}

export function hide_addbutton_args_modal(): void {
  lastModals.splice(lastModals.indexOf('addbuttonArgs-modal'), 1);
  const containers = q('.addbutton-modal-container-args');
  const first = containers.get(0);
  if (!first) return;
  const intervalId = setInterval(function () {
    const currentOpacity = parseFloat(q(first).css('opacity') ?? '');
    if (currentOpacity <= 0) {
      clearInterval(intervalId);
      setTimeout(function () {
        containers.toArray().forEach((element) => {
          const modalId = q(element).attr('arg_modal_ID') ?? '';
          const target = byId('modal-container-' + modalId).get(0) ?? null;
          if (target) {
            q(element).css('display', 'none');
          }
        });
      }, 100);
      return;
    }
    containers.toArray().forEach((element) => {
      q(element).css('opacity', (currentOpacity - 3.5).toFixed(2));
    });
  }, 10);
  modalFlags.is_addbutton_args_modal_opened = 0;
}

function show_editbutton_modal(modalId: string): void {
  lastModals.push('editbutton-modal');
  const el = byId('edit-modal-container-' + modalId).get(0) ?? null;
  if (el) {
    q(el).css({ opacity: '100%', display: 'block' });
    modalFlags.is_editbutton_modal_opened = 1;
  }
}

export function hide_editbutton_modal(modalId?: string): void {
  lastModals.splice(lastModals.indexOf('editbutton-modal'), 1);
  const el = modalId ? byId('edit-modal-container-' + modalId).get(0) ?? null : null;
  if (el) {
    fadeOut(el, () => {
      q(el).css('display', 'none');
    });
    modalFlags.is_editbutton_modal_opened = 0;
  }
}

export function hide_last_modal(): void {
  switch (lastModals[lastModals.length - 1]) {
    case 'config-modal':
      hide_modal();
      break;
    case 'addbutton-modal':
      hide_addbutton_modal();
      break;
    case 'addbuttonArgs-modal':
      hide_addbutton_modal();
      break;
    case 'editbutton-modal':
      hide_editbutton_modal();
      break;
  }
}

export function wireModals(onEditParam: () => void, isSwapMode: () => boolean): void {
  const urlParams = new URLSearchParams(window.location.search);

  const modalParam = urlParams.get('config');
  if (modalParam === 'show' || modalParam === 'true') {
    q('.modal-container').css({ opacity: '1', display: 'block' });
    lastModals.push('config-modal');
    window.history.replaceState({}, document.title, window.location.pathname);
  }

  const editParams = urlParams.get('edit');
  if (editParams === 'true') {
    onEditParam();
    window.history.replaceState({}, document.title, window.location.pathname);
  }

  const open_modal_buttons = q('.open-config-modal');
  const modalEl = q('.modal-container').get(0) ?? null;
  const modal_close_button = q('.modal-close').get(0) ?? null;

  open_modal_buttons.toArray().forEach((button) => {
    q(button).on('click', function () {
      if (!isSwapMode()) {
        show_modal();
      }
    });
  });

  q(modal_close_button).on('click', function () {
    hide_modal();
  });

  q(modalEl).on('click', function (event) {
    if (event.target === modalEl) {
      hide_modal();
    }
  });

  const open_modal_addbutton = q('div.add-button');
  const addbutton_modal = q('.addbutton-modal-container').get(0) ?? null;
  const addbutton_modal_close_button = q('.addbutton-modal-close').get(0) ?? null;

  open_modal_addbutton.toArray().forEach((button) => {
    q(button).on('click', function () {
      if (!isSwapMode()) {
        const addIdValue = q(button).attr('add_ID');
        const addFolderValue = q(button).attr('add_FOLDER');
        show_addbutton_modal(addFolderValue ?? null, addIdValue ?? null);
      }
    });
  });

  q(addbutton_modal_close_button).on('click', function () {
    hide_addbutton_modal();
  });

  q(addbutton_modal).on('click', function (event) {
    if (event.target === addbutton_modal) {
      hide_addbutton_modal();
    }
  });

  q('button.no-dropdown')
    .toArray()
    .forEach(function (button) {
      q(button).on('click', function () {
        const modalId = q(button).attr('arg_modal_ID') ?? '';
        if (!isSwapMode()) {
          show_addbutton_args_modal(modalId);
        }
      });
    });

  q('.addbutton-modal-container-args')
    .toArray()
    .forEach(function (modalContainer) {
      q(modalContainer).on('click', function (event) {
        if (event.target === modalContainer) {
          hide_addbutton_args_modal();
        }
      });
    });

  q('.addbutton-modal-close-args')
    .toArray()
    .forEach(function (close_button) {
      q(close_button).on('click', function () {
        hide_addbutton_args_modal();
      });
    });

  q('.edit-button')
    .toArray()
    .forEach(function (button) {
      q(button).on('click', function () {
        if (modalFlags.is_editbutton_modal_opened === 0) {
          const modalId = q(button).attr('edit_modal_id') ?? '';
          show_editbutton_modal(modalId);
        }
      });
    });

  q('.editbutton-modal-container')
    .toArray()
    .forEach(function (modalContainer) {
      const modalId = q(modalContainer).attr('edit_modal_id') ?? '';
      q(modalContainer).on('click', function (event) {
        if (event.target === modalContainer) {
          hide_editbutton_modal(modalId);
        }
      });
    });

  q('.editbutton-modal-close')
    .toArray()
    .forEach(function (close_button) {
      q(close_button).on('click', function () {
        const container = q(close_button).closest('.editbutton-modal-container');
        const modalId = container.attr('edit_modal_id') ?? '';
        hide_editbutton_modal(modalId);
      });
    });
}
