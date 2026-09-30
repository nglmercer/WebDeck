// Compatibility entry points; visibility belongs to ModalShell components.
import { modalState, openModal, closeModal } from '../features/modals/state.svelte';
import { byId } from '../query';
import { pageState } from './state';
export function isAddbuttonModalOpened(): number { return Number(modalState.stack.some(key => key === 'add' || key.startsWith('modal-container-'))); }
export function isEditbuttonModalOpened(): number { return Number(modalState.stack.some(key => key.startsWith('edit-modal-container-'))); }
export function resetModalState(): void { modalState.stack = []; }
export function show_modal(): void { if (pageState.canEdit) openModal('config'); }
export function hide_modal(): void { closeModal('config'); }
function show_addbutton_modal(folder: string | null, id: string | null): void {
  byId('addbutton-modal-content').attr('data-add-folder', folder ?? '').attr('data-add-id', id ?? '');
  openModal('add');
}
export function hide_addbutton_modal(): void { closeModal('add'); }
export function hide_addbutton_args_modal(): void {
  for (const key of [...modalState.stack]) if (key.startsWith('modal-container-')) closeModal(key);
}
export function hide_editbutton_modal(id?: string): void {
  if (id !== undefined) closeModal('edit-modal-container-' + id);
  else for (const key of [...modalState.stack]) if (key.startsWith('edit-modal-container-')) closeModal(key);
}
export function hide_last_modal(): void {
  const key = modalState.stack[modalState.stack.length - 1];
  if (key !== undefined) closeModal(key);
}
let teardown: (() => void) | undefined;
/** One delegated listener, explicitly owned and removed by App on teardown. */
export function wireModals(onEditParam: () => void, isSwapMode: () => boolean): () => void {
  teardown?.();
  const params = new URLSearchParams(location.search);
  if (['show','true'].includes(params.get('config') ?? '')) show_modal();
  if (params.get('edit') === 'true') onEditParam();
  if (params.has('config') || params.has('edit')) history.replaceState({}, document.title, location.pathname);
  const click = (event: MouseEvent): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const closest = (selector: string): HTMLElement | null => target.closest<HTMLElement>(selector);
    if (closest('.open-config-modal') && !isSwapMode()) show_modal();
    else if (closest('.modal-close')) hide_modal();
    else if (closest('.addbutton-modal-close')) hide_addbutton_modal();
    else if (closest('.addbutton-modal-close-args')) hide_addbutton_args_modal();
    else if (closest('.editbutton-modal-close')) hide_editbutton_modal(closest('.editbutton-modal-container')?.dataset.editModalId);
    else if (closest('.edit-button') && !isSwapMode()) openModal('edit-modal-container-' + closest('.edit-button')?.dataset.editModalId);
    else if (closest('div.add-button') && !isSwapMode()) {
      const slot = closest('div.add-button'); show_addbutton_modal(slot?.dataset.addFolder ?? null, slot?.dataset.addId ?? null);
    } else if (closest('button.no-dropdown') && !isSwapMode()) openModal('modal-container-' + closest('button.no-dropdown')?.dataset.argModalId);
    else if (target.matches('.modal-container')) hide_modal();
    else if (target.matches('.addbutton-modal-container')) hide_addbutton_modal();
    else if (target.matches('.addbutton-modal-container-args')) hide_addbutton_args_modal();
    else if (target.matches('.editbutton-modal-container')) hide_editbutton_modal((target as HTMLElement).dataset.editModalId);
  };
  document.addEventListener('click', click);
  teardown = () => document.removeEventListener('click', click);
  return teardown;
}
