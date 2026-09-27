import type { BootContext } from '../framework/types';

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

function modal(): Element | null {
  return document.querySelector('.modal-container');
}

function addbuttonModal(): Element | null {
  return document.querySelector('.addbutton-modal-container');
}

export function show_modal(): void {
  lastModals.push('config-modal');
  const el = modal() as HTMLElement | null;
  if (!el) return;
  el.style.opacity = '100%';
  el.style.display = 'block';
}

export function hide_modal(): void {
  lastModals.splice(lastModals.indexOf('config-modal'), 1);
  const el = modal() as HTMLElement | null;
  if (!el) return;
  const intervalId = setInterval(function () {
    const currentOpacity = parseFloat(getComputedStyle(el).opacity);
    if (currentOpacity <= 0) {
      clearInterval(intervalId);
      setTimeout(function () {
        el.style.display = 'none';
      }, 100);
      return;
    }
    el.style.opacity = (currentOpacity - 3.5).toFixed(2);
  }, 10);
}

export function show_addbutton_modal(addFolder: string | null, addId: string | null): void {
  lastModals.push('addbutton-modal');
  const el = addbuttonModal() as HTMLElement | null;
  if (!el) return;
  el.style.opacity = '100%';
  el.style.display = 'block';
  modalFlags.is_addbutton_modal_opened = 1;
  document.querySelector('#addbutton-modal-content')?.setAttribute('add_FOLDER', addFolder ?? '');
  document.querySelector('#addbutton-modal-content')?.setAttribute('add_ID', addId ?? '');
}

export function hide_addbutton_modal(): void {
  lastModals.splice(lastModals.indexOf('addbutton-modal'), 1);
  document.querySelector('#addbutton-modal-content')?.removeAttribute('add_ID');
  document.querySelector('#addbutton-modal-content')?.removeAttribute('add_FOLDER');
  const el = addbuttonModal() as HTMLElement | null;
  if (!el) return;
  const intervalId = setInterval(function () {
    const currentOpacity = parseFloat(getComputedStyle(el).opacity);
    if (currentOpacity <= 0) {
      clearInterval(intervalId);
      setTimeout(function () {
        el.style.display = 'none';
      }, 100);
      return;
    }
    el.style.opacity = (currentOpacity - 3.5).toFixed(2);
  }, 10);
  modalFlags.is_addbutton_modal_opened = 0;
}

export function show_addbutton_args_modal(modalId: string): void {
  lastModals.push('addbuttonArgs-modal');
  const el = document.getElementById('modal-container-' + modalId);
  if (el) {
    el.style.opacity = '100%';
    el.style.display = 'block';
    modalFlags.is_addbutton_args_modal_opened = 1;
  }
}

export function hide_addbutton_args_modal(): void {
  lastModals.splice(lastModals.indexOf('addbuttonArgs-modal'), 1);
  const containers = document.querySelectorAll('.addbutton-modal-container-args');
  const first = containers[0] as HTMLElement | undefined;
  if (!first) return;
  const intervalId = setInterval(function () {
    const currentOpacity = parseFloat(getComputedStyle(first).opacity);
    if (currentOpacity <= 0) {
      clearInterval(intervalId);
      setTimeout(function () {
        for (const element of containers) {
          const modalId = element.getAttribute('arg_modal_ID') ?? '';
          const target = document.getElementById('modal-container-' + modalId);
          if (target) {
            (element as HTMLElement).style.display = 'none';
          }
        }
      }, 100);
      return;
    }
    for (const element of containers) {
      (element as HTMLElement).style.opacity = (currentOpacity - 3.5).toFixed(2);
    }
  }, 10);
  modalFlags.is_addbutton_args_modal_opened = 0;
}

export function show_editbutton_modal(modalId: string): void {
  lastModals.push('editbutton-modal');
  const el = document.getElementById('edit-modal-container-' + modalId);
  if (el) {
    el.style.opacity = '100%';
    el.style.display = 'block';
    modalFlags.is_editbutton_modal_opened = 1;
  }
}

export function hide_editbutton_modal(modalId?: string): void {
  lastModals.splice(lastModals.indexOf('editbutton-modal'), 1);
  const el = modalId ? document.getElementById('edit-modal-container-' + modalId) : null;
  if (el) {
    const intervalId = setInterval(function () {
      const currentOpacity = parseFloat(getComputedStyle(el).opacity);
      if (currentOpacity <= 0) {
        clearInterval(intervalId);
        setTimeout(function () {
          el.style.display = 'none';
        }, 100);
        return;
      }
      el.style.opacity = (currentOpacity - 3.5).toFixed(2);
    }, 10);
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
      hide_addbutton_args_modal();
      break;
    case 'editbutton-modal':
      hide_editbutton_modal();
      break;
  }
}

export function wireModals(ctx: BootContext, onEditParam: () => void, isSwapMode: () => boolean): void {
  void ctx;
  const urlParams = new URLSearchParams(window.location.search);

  const modalParam = urlParams.get('config');
  if (modalParam === 'show' || modalParam === 'true') {
    const el = document.querySelector('.modal-container') as HTMLElement | null;
    if (el) {
      el.style.opacity = '1';
      el.style.display = 'block';
    }
    lastModals.push('config-modal');
    window.history.replaceState({}, document.title, window.location.pathname);
  }

  const editParams = urlParams.get('edit');
  if (editParams === 'true') {
    onEditParam();
    window.history.replaceState({}, document.title, window.location.pathname);
  }

  const open_modal_buttons = document.querySelectorAll('.open-config-modal');
  const modalEl = document.querySelector('.modal-container');
  const modal_close_button = document.querySelector('.modal-close');

  for (const button of open_modal_buttons) {
    button.addEventListener('click', function () {
      if (!isSwapMode()) {
        show_modal();
      }
    });
  }

  modal_close_button?.addEventListener('click', function () {
    hide_modal();
  });

  modalEl?.addEventListener('click', function (event) {
    if (event.target === modalEl) {
      hide_modal();
    }
  });

  const open_modal_addbutton = document.querySelectorAll('div.add-button');
  const addbutton_modal = document.querySelector('.addbutton-modal-container');
  const addbutton_modal_close_button = document.querySelector('.addbutton-modal-close');

  for (const button of open_modal_addbutton) {
    button.addEventListener('click', function () {
      if (!isSwapMode()) {
        const addIdValue = button.getAttribute('add_ID');
        const addFolderValue = button.getAttribute('add_FOLDER');
        show_addbutton_modal(addFolderValue, addIdValue);
      }
    });
  }

  addbutton_modal_close_button?.addEventListener('click', function () {
    hide_addbutton_modal();
  });

  addbutton_modal?.addEventListener('click', function (event) {
    if (event.target === addbutton_modal) {
      hide_addbutton_modal();
    }
  });

  const open_modal_addbutton_args = document.querySelectorAll('button.no-dropdown');

  open_modal_addbutton_args.forEach(function (button) {
    button.addEventListener('click', function () {
      const modalId = button.getAttribute('arg_modal_ID') ?? '';
      if (!isSwapMode()) {
        show_addbutton_args_modal(modalId);
      }
    });
  });

  const addbutton_args_modal_containers = document.querySelectorAll('.addbutton-modal-container-args');
  addbutton_args_modal_containers.forEach(function (modalContainer) {
    modalContainer.addEventListener('click', function (event) {
      if (event.target === modalContainer) {
        hide_addbutton_args_modal();
      }
    });
  });

  const addbutton_args_modal_close_buttons = document.querySelectorAll('.addbutton-modal-close-args');
  addbutton_args_modal_close_buttons.forEach(function (close_button) {
    close_button.addEventListener('click', function () {
      hide_addbutton_args_modal();
    });
  });

  const open_modal_editbutton = document.querySelectorAll('.edit-button');
  const editbutton_modal_containers = document.querySelectorAll('.editbutton-modal-container');

  open_modal_editbutton.forEach(function (button) {
    button.addEventListener('click', function () {
      if (modalFlags.is_editbutton_modal_opened === 0) {
        const modalId = button.getAttribute('edit_modal_id') ?? '';
        show_editbutton_modal(modalId);
      }
    });
  });

  editbutton_modal_containers.forEach(function (modalContainer) {
    const modalId = modalContainer.getAttribute('edit_modal_id') ?? '';
    modalContainer.addEventListener('click', function (event) {
      if (event.target === modalContainer) {
        hide_editbutton_modal(modalId);
      }
    });
  });

  const editbutton_modal_close_buttons = document.querySelectorAll('.editbutton-modal-close');
  editbutton_modal_close_buttons.forEach(function (close_button) {
    close_button.addEventListener('click', function () {
      const container = close_button.closest('.editbutton-modal-container');
      const modalId = container?.getAttribute('edit_modal_id') ?? '';
      hide_editbutton_modal(modalId);
    });
  });
}
