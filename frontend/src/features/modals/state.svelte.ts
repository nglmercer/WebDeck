/** Each ModalShell owns its visibility and focus lifecycle. */
export const modalState = $state<{ stack: string[] }>({ stack: [] });
export function openModal(key: string): void {
  modalState.stack = [...modalState.stack.filter(item => item !== key), key];
}
export function closeModal(key: string): void {
  modalState.stack = modalState.stack.filter(item => item !== key);
}
export function modalKey(classes: string, id?: string): string {
  if (classes.split(' ').includes('modal-container')) return 'config';
  if (classes.split(' ').includes('addbutton-modal-container')) return 'add';
  return id ?? classes;
}
