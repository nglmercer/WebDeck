import { html, type Html } from '../framework/html';

// Shared inline-SVG icons: single source for the glyphs previously pasted
// into each view. Helpers emit single-line markup (inter-tag whitespace is
// insignificant); every attribute — including the upstream duplicated
// `class` quirk on the Bootstrap icons — is preserved byte-identical.

const CLOSE_PATH =
  'M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z';

const PLUS_PATH = 'M12 4v16m8-8H4';

const PENCIL_PATH =
  'M12 8.00012L4 16.0001V20.0001L8 20.0001L16 12.0001M12 8.00012L14.8686 5.13146L14.8704 5.12976C15.2652 4.73488 15.463 4.53709 15.691 4.46301C15.8919 4.39775 16.1082 4.39775 16.3091 4.46301C16.5369 4.53704 16.7345 4.7346 17.1288 5.12892L18.8686 6.86872C19.2646 7.26474 19.4627 7.46284 19.5369 7.69117C19.6022 7.89201 19.6021 8.10835 19.5369 8.3092C19.4628 8.53736 19.265 8.73516 18.8695 9.13061L18.8686 9.13146L16 12.0001M12 8.00012L16 12.0001';

const X_PATH =
  'M18.8,16l5.5-5.5c0.8-0.8,0.8-2,0-2.8l0,0C24,7.3,23.5,7,23,7c-0.5,0-1,0.2-1.4,0.6L16,13.2l-5.5-5.5  c-0.8-0.8-2.1-0.8-2.8,0C7.3,8,7,8.5,7,9.1s0.2,1,0.6,1.4l5.5,5.5l-5.5,5.5C7.3,21.9,7,22.4,7,23c0,0.5,0.2,1,0.6,1.4  C8,24.8,8.5,25,9,25c0.5,0,1-0.2,1.4-0.6l5.5-5.5l5.5,5.5c0.8,0.8,2.1,0.8,2.8,0c0.8-0.8,0.8-2.1,0-2.8L18.8,16z';

const FOLDER_X_PATH =
  'M11.742 4.258a1 1 0 0 0-1.414 0L8 6.586 5.672 4.258a1 1 0 1 0-1.414 1.414L6.586 8 4.258 10.328a1 1 0 0 0 1.414 1.414L8 9.414l2.328 2.328a1 1 0 0 0 1.414-1.414L9.414 8l2.328-2.328a1 1 0 0 0 0-1.414z';

const TRASH_PATHS = [
  'M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5Zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5Zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6Z',
  'M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1ZM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118ZM2.5 3h11V2h-11v1Z',
];

/**
 * Modal close glyph (bi-x-circle-fill, 19px), shared by the config, edit,
 * and both add-button modals. `cls` is the modal's close-icon class,
 * `dark` the pre-escaped theme suffix (`raw(dark)` at call sites).
 */
export function modalCloseIcon(cls: string, dark: Html): Html {
  return html`<svg class="${cls} ${dark}" xmlns="http://www.w3.org/2000/svg" width="19" height="19" fill="currentColor" class="bi bi-x-circle-fill" viewBox="0 0 16 16"><path d="${CLOSE_PATH}"/></svg>`;
}

/** Add-button plus glyph (empty grid slots). */
export function addPlusIcon(): Html {
  return html`<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"><path d="${PLUS_PATH}" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
}

/** Tile edit-badge pencil glyph. */
export function editPencilIcon(): Html {
  return html`<svg width="16px" height="16px" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><g id="Edit / Edit_Pencil_01"><path id="Vector" d="${PENCIL_PATH}" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></g></svg>`;
}

/** Tile delete-badge X glyph. */
export function deleteXIcon(): Html {
  return html`<svg fill="currentColor" width="16px" height="16px" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg"><path d="${X_PATH}"/></svg>`;
}

/**
 * Folder-tab delete glyph (bi-x-circle, 16px). `safeFolder` is the
 * caller-escaped folder id; it passes through the standard escaping,
 * exactly as the inline version did.
 */
export function folderDeleteIcon(safeFolder: string): Html {
  return html`<svg class="delete-icon" onclick="event.stopPropagation(); deleteFolder('${safeFolder}')" xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-x-circle" viewBox="0 0 16 16"><path d="${FOLDER_X_PATH}"/></svg>`;
}

/** Background-picker trash glyph (bi-trash, 16px) with a `<title>`. */
export function trashIcon(title: string): Html {
  return html`<svg class="choose-bg-delete-button" xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-trash" viewBox="0 0 16 16"><title> ${title} </title><path d="${TRASH_PATHS[0]}"></path><path d="${TRASH_PATHS[1]}"></path></svg>`;
}
