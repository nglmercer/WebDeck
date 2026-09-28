// Per-modal wire-up (extracted from addbutton.ts; the old inline <script> block).

import { text } from '../../framework/i18n';
import {
  asArray,
  asBool,
  asObject,
  asString,
  get,
  rep,
  type BootContext,
} from '../../framework/types';
import { q, byId, post } from '../../query';
import { wireKeyField } from '../../components/keyfield';
import { buildCommand, catKey, cmdKey, registerShowArg } from '../args';
import { swapPreviewImage, updateButtonBackgroundColor, updateImageSize, type ButtonState } from '../modalstyle';
import { wireStudioPreview } from '../studio-preview';
import { addButtonName } from './argsmodal';
import type { AddModalContext } from './types';

const addModalStates = new Map<string, { button: ButtonState; command: string }>();

export function collectAddModals(ctx: BootContext): AddModalContext[] {
  const out: AddModalContext[] = [];
  const commands = asObject(ctx.commands);
  Object.entries(commands).forEach(([category, categoryValue], catIndex) => {
    Object.entries(asObject(categoryValue)).forEach(([command, commandValue]) => {
      if (command === 'CATEGORY-SETTINGS') return;
      const cmdObj = asObject(commandValue);
      const cmdIndex = Object.keys(asObject(categoryValue)).indexOf(command);
      const argModalId = `${catIndex}X${cmdIndex}`;
      if (asString(cmdObj['TYPE']) !== 'multiple') {
        const titleQuery = `${catKey(category)}_${cmdKey(command)}__btn_name`;
        let buttonTitle = text(titleQuery);
        if (buttonTitle === titleQuery || buttonTitle === '') buttonTitle = command;
        out.push({
          argModalId,
          category,
          command,
          parentCommand: '',
          subId: 0,
          commandValue: cmdObj,
          commandId: asString(cmdObj['command']),
          buttonTitle,
        });
      } else {
        asArray(cmdObj['commands']).forEach((subValue, subIndex) => {
          const subObj = asObject(subValue);
          const subArgId = `${argModalId}X${subIndex}`;
          const subCommand = rep(asString(subObj['command']), '/', '');
          const subBtn = asString(subObj['TYPE']).includes('multiple') ? 'category' : 'btn';
          const subTitleQuery = `${catKey(category)}_${cmdKey(command)}_sub${subIndex + 1}__${subBtn}_name`;
          let subTitle = text(subTitleQuery);
          if (subTitle === subTitleQuery || subTitle === '') subTitle = subCommand;
          out.push({
            argModalId: subArgId,
            category,
            command: subCommand,
            parentCommand: command,
            subId: subIndex + 1,
            commandValue: subObj,
            commandId: asString(subObj['command']),
            buttonTitle: subTitle,
          });
        });
      }
    });
  });
  return out;
}

/** Collect arg values in DOM order (exported for tests; powers submit + usage preview). */
export function getCommand(command: string, argModalId: string): string {
  const container = q(`form[arg_modal_ID="${argModalId}"] .args-container`).get(0);
  if (!container) return command + ' ';
  return buildCommand(command, container);
}

export function wireAddModal(ctx: BootContext, mctx: AddModalContext): void {
  const id = mctx.argModalId;
  const devMode = asBool(get(ctx.config, 'settings', 'dev_mode'));
  const style = asObject(mctx.commandValue['style']);
  const hasStyle = Object.keys(mctx.commandValue).includes('style') && Object.keys(style).length > 0;
  const btn: 'btn' | 'category' = asString(mctx.commandValue['TYPE']).includes('multiple') ? 'category' : 'btn';
  const buttonState: ButtonState = hasStyle ? { ...style } : {};
  const buttonName = addButtonName(ctx, mctx, btn, hasStyle);
  buttonState['name'] = buttonName;
  // NOTE: upstream references an undefined `command_X` global (ReferenceError
  // breaks usage add-modals); the intent is unambiguously the command id.
  addModalStates.set(id, { button: buttonState, command: mctx.commandId });

  registerShowArg(id, 'arg_modal_ID');

  wireFoldernameForm(id);
  wireUsagePreview(id);
  wireKeyField(id);
  wireStudioPreview(id);

  byId(`image-input_${id}`).on('change', function () {
    const input = this as unknown as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const formData = new FormData();
    formData.append('file', file);

    void post('/upload_file', formData).then(
      () => {
        console.log('File downloaded successfully!');
        const swapped = swapPreviewImage(id, input);
        if (!swapped) return;
        buttonState['image_size'] = '70';
        updateImageSize(swapped.slider, swapped.value, swapped.image, buttonState);
        buttonState['image'] = '**uploaded/' + (input.files?.[0]?.name ?? '');
      },
      () => {
        console.error('Failed to download file.');
      }
    );
  });

  const image = byId<HTMLElement>(`button-image_${id}`).get(0) ?? null;
  const imageSizeSlider = byId<HTMLInputElement>(`image-size-slider_${id}`).get(0) ?? null;
  const imageSizeValue = byId<HTMLInputElement>(`image-size-value_${id}`).get(0) ?? null;
  if (image && imageSizeSlider && imageSizeValue) {
    updateImageSize(imageSizeSlider, imageSizeValue, image, buttonState);
  }

  const buttonElement = byId<HTMLElement>(`button-element_${id}`).get(0) ?? null;
  const bgInput = byId<HTMLInputElement>(`background-color-input_${id}`).get(0) ?? null;
  const bgHex = byId<HTMLInputElement>(`background-color-hex_${id}`).get(0) ?? null;
  if (buttonElement && bgInput && bgHex) {
    updateButtonBackgroundColor(buttonElement, bgInput, bgHex, buttonState);
  }

  if (devMode) {
    byId(`command_${id}`).on('input', function () {
      buttonState['message'] = String(q(this).val() ?? '');
    });
  }

  const buttonText = byId<HTMLInputElement>(`button-text-input_${id}`).get(0) ?? null;
  const buttonPreview = byId(`button-text-preview_${id}`).get(0) ?? null;
  q(buttonText).on('input', function () {
    const textValue = String(q(buttonText).val() ?? '');
    if (buttonPreview) q(buttonPreview).text(textValue);
    buttonState['name'] = textValue;
  });

  byId(`${id}_submit`).on('click', function (event) {
    if (id !== 'NONE') {
      event.preventDefault();
    }
    setTimeout(function () {
      buttonCommandAdd(id, mctx.commandId);
    }, 1000);
  });
}

/** Folder-creation form (per-modal binding; created folders update every folder list). */
export function wireFoldernameForm(modalId: string): void {
  byId(`submitButton_${modalId}`).on('click', function (event) {
    event.preventDefault();
    const folderName = String(byId<HTMLInputElement>(`folderName_${modalId}`).val() ?? '');
    if (folderName.trim() !== '') {
      const parentFolderEl = q('.buttons-center:not(.invisible)').get(0) ?? null;
      const parentFolder = (q(parentFolderEl).prop('id') ?? '').replace(/^folder-/, '');
      const data = {
        name: folderName.replace(/"/g, ''),
        parent_folder: parentFolder,
      };
      console.log(data);

      fetch('/create_folder', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
      })
        .then(function (response) {
          if (response.ok) {
            return response.json();
          } else {
            console.log("Une erreur s'est produite lors de la création du dossier.");
          }
        })
        .then(function (data: { success?: boolean } | undefined) {
          if (data && data.hasOwnProperty('success')) {
            if (data.success) {
              console.log(text('folder_created_successfully'));
              q('.webdeck_foldername_ALL')
                .toArray()
                .forEach(function (div) {
                  const newDiv = q('<div>').attr('class', 'webdeck_foldername');
                  const input = q<HTMLInputElement>('<input>').attr({
                    type: 'radio',
                    name: 'file',
                    value: folderName,
                    required: '',
                  });
                  const label = q('<label>').attr('for', folderName).text(folderName);
                  newDiv.append(input).append(label);
                  q(div).append(newDiv);
                });
            } else {
              console.log('Folder creation failed because the folder already exists.');
            }
          }
        })
        .catch(function (error) {
          console.log('Erreur : ' + error);
        });
    }
  });
}

/** Second class token (equivalent to `classList.item(1)`). */
function secondClass(element: Element): string | undefined {
  return q(element)
    .attr('class')
    ?.split(/\s+/)
    .filter((token) => token !== '')[1];
}

function wireUsagePreview(id: string): void {
  const usageInput = byId<HTMLInputElement>(`usage-title-input_${id}`).get(0) ?? null;
  if (!usageInput) return;
  const usageTitlePreview = byId(`usage-title_${id}`).get(0) ?? null;
  const usageValuePreview = byId(`usage-value_${id}`).get(0) ?? null;
  if (!usageTitlePreview || !usageValuePreview) return;

  const initial = String(q(usageInput).val() ?? '');
  q(usageTitlePreview).text(initial);
  q(usageValuePreview).text('-');

  q(usageInput).on('input', function () {
    const state = addModalStates.get(id);
    const current = String(q(usageInput).val() ?? '');
    q(usageTitlePreview).text(current);
    if (state) state.button['name'] = current;
  });

  reloadUsagePreview(id);
  byId(`disk-letter_${id}`).on('change', function () {
    console.log('changed!');
    q(usageValuePreview).text('-');
    const titleSecond = secondClass(usageTitlePreview);
    const valueSecond = secondClass(usageValuePreview);
    if (titleSecond) q(usageTitlePreview).removeClass(titleSecond);
    if (valueSecond) q(usageValuePreview).removeClass(valueSecond);
    reloadUsagePreview(id);
  });
}

function reloadUsagePreview(id: string): void {
  const state = addModalStates.get(id);
  if (!state) return;
  const commandString = getCommand(state.command, id)
    .replace(/<\|§\|>/g, '')
    .replace(/\[object HTMLInputElement\]/g, '');
  const message = "/usage '" + commandString;
  let path = message.replace(/\]\['/g, '.').replace(/\['/g, '.').replace(/'\]/g, '');
  path = path.substring(path.lastIndexOf("'") + 1).replace(/ /g, '');
  // NOTE: `.addClass('')` is a no-op where `classList.add('')` would throw;
  // `path` is never empty in practice (it always contains the command).
  byId(`usage-title_${id}`).addClass(`${path}`);
  byId(`usage-value_${id}`).addClass(`${path}`);
}

function buttonCommandAdd(argModalId: string, command: string): void {
  const state = addModalStates.get(argModalId);
  if (!state) return;
  const commandString = argModalId === 'NONE' ? command : getCommand(command, argModalId);
  console.log(commandString);

  state.button['message'] = commandString;
  console.log(state.button);
  console.log('buttonCommand received, from: add');
  const element = byId('addbutton-modal-content').get(0) ?? null;
  const locationFolder = q(element).attr('add_FOLDER') ?? '';
  const locationId = q(element).attr('add_ID') ?? '';

  fetch('/get_config')
    .then(function (response) {
      if (response.ok) {
        return response.json();
      } else {
        throw new Error(text('settings_load_error'));
      }
    })
    .then(function (configData: Record<string, unknown>) {
      const front = (configData['front'] as Record<string, unknown> | undefined) ?? {};
      const buttons = (front['buttons'] as Record<string, unknown[]> | undefined) ?? {};
      const folder = buttons[locationFolder];
      if (folder) folder[Number(locationId)] = state.button as unknown as never;
      return fetch('/save_buttons_only', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(configData),
      });
    })
    .then(function (response) {
      if (response.ok) {
        return response.json();
      } else {
        throw new Error(text('settings_save_error'));
      }
    })
    .then(function (response: { success?: boolean }) {
      if (response.success) {
        alert(text('settings_save_success'));
        window.location.href = window.location.href + '?edit=true';
      } else {
        throw new Error(text('settings_save_error'));
      }
    })
    .catch(function (error: Error) {
      alert(error.message);
      throw new Error(error.message);
    });

  q(element).removeAttr('add_ID');
  q(element).removeAttr('add_FOLDER');
}

/**
 * Filter the add-button browser by label/command text. Leaf buttons that
 * match (or sit under a matching branch) stay visible; empty branches
 * hide, and containers holding matches open. An empty query restores
 * every button but leaves containers as they are.
 */
export function filterAddBrowser(root: Element, query: string): void {
  const needle = query.trim().toLowerCase();
  const leaves = [...root.querySelectorAll('.dropdown-btn.no-dropdown')] as HTMLElement[];
  const branches = [...root.querySelectorAll('.dropdown-btn:not(.no-dropdown)')] as HTMLElement[];
  const descs = [...root.querySelectorAll('.addbutton-description')] as HTMLElement[];
  if (needle === '') {
    for (const el of [...leaves, ...branches, ...descs]) el.style.display = '';
    return;
  }
  for (const leaf of leaves) {
    const hay =
      `${leaf.textContent ?? ''} ${leaf.getAttribute('dropdown-commandTag') ?? ''}`.toLowerCase();
    leaf.style.display = hay.includes(needle) ? '' : 'none';
  }
  // Descriptions follow their button (the next sibling).
  for (const desc of descs) {
    const next = desc.nextElementSibling as HTMLElement | null;
    desc.style.display = next !== null && next.style.display === 'none' ? 'none' : '';
  }
  // Reversed document order is bottom-up here: every branch panel follows
  // its button, so descendants always settle before their ancestors.
  for (const branch of branches.reverse()) {
    const panel = branch.nextElementSibling as HTMLElement | null;
    const selfMatch = (branch.textContent ?? '').toLowerCase().includes(needle);
    if (selfMatch && panel !== null) {
      for (const el of panel.querySelectorAll('.dropdown-btn, .addbutton-description')) {
        (el as HTMLElement).style.display = '';
      }
    }
    const visible =
      selfMatch ||
      (panel !== null &&
        [...panel.querySelectorAll('.dropdown-btn')].some(
          (b) => (b as HTMLElement).style.display !== 'none'
        ));
    branch.style.display = visible ? '' : 'none';
    if (panel !== null && panel.classList.contains('dropdown-container')) {
      panel.style.display = visible ? 'block' : 'none';
    }
  }
}

/** Live-filter wiring for the add-button browser search box. */
export function wireBrowserSearch(): void {
  const input = byId<HTMLInputElement>('addbutton-search').get(0) ?? null;
  const root = q('.all-commands').get(0) ?? null;
  if (input === null || root === null) return;
  q(input).on('input', function () {
    filterAddBrowser(root, String(q(input).val() ?? ''));
  });
}

/** Add-modal dropdown toggles (index.jinja inline script after commands). */
export function wireBrowserDropdowns(): void {
  for (const btn of q('.dropdown-btn').toArray()) {
    q(btn).on('click', function (this: Element) {
      if (!(q(btn).hasClass('final-btn') || q(btn).hasClass('no-dropdown'))) {
        q(this).toggleClass('active');
      }
      // NOTE: the visibility probe reads the *inline* style; qdom's `.css()`
      // getter is computed-only, so this one read stays native.
      const dropdownContent = q(this).next().get(0) as HTMLElement | undefined;
      try {
        if (dropdownContent?.style.display === 'block') {
          if (!q(dropdownContent).hasClass('addbutton-description')) {
            q(dropdownContent).css('display', 'none');
          }
        } else if (dropdownContent) {
          q(dropdownContent).css('display', 'block');
        }
      } catch {
        // Ne rien faire (pass)
      }
    });
  }
}
