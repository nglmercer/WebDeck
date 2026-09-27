import { html, join, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import {
  asArray,
  asBool,
  asObject,
  asString,
  get,
  rep,
  type BootContext,
  type JsonObject,
} from '../framework/types';
import { argField, type ArgsRenderContext } from './args';
import { swapPreviewImage, updateButtonBackgroundColor, updateImageSize, type ButtonState } from './modalstyle';
import { svgSlot } from './svg';

export interface AddModalContext {
  argModalId: string;
  category: string;
  command: string;
  parentCommand: string;
  subId: number;
  commandValue: JsonObject;
  commandId: string;
  buttonTitle: string;
}

function catKey(category: string): string {
  return category.replace(/ /g, '').toUpperCase();
}

function cmdKey(command: string): string {
  return command.replace(/ /g, '_').replace(/'/g, '').toLowerCase();
}

/** Command browser (index.jinja `.all-commands` block). */
export function addBrowserView(ctx: BootContext): Html {
  const commands = asObject(ctx.commands);
  const dark = ctx.dark_theme;

  const categories = Object.entries(commands).map(([category, categoryValue], catIndex) => {
    const catObj = asObject(categoryValue);
    const catQuery = `${catKey(category)}_CATEGORY_NAME`;
    let categoryName = text(catQuery);
    if (categoryName === catQuery || categoryName === '') categoryName = category;

    const items = Object.entries(catObj)
      .map(([command, commandValue], cmdIndex) => {
        if (command === 'CATEGORY-SETTINGS') return raw('');
        const cmdObj = asObject(commandValue);
        const argModalId = `${catIndex}X${cmdIndex}`;
        const type = asString(cmdObj['TYPE']);

        const descQuery = `${catKey(category)}_${cmdKey(command)}__${type.includes('multiple') ? 'category' : 'btn'}_description`;
        let buttonDescription = text(descQuery);
        if (buttonDescription === descQuery) buttonDescription = '';
        const descBlock =
          buttonDescription !== ''
            ? html`<div class="addbutton-description"><p>${buttonDescription}</p></div>`
            : raw('');

        if (type !== 'multiple') {
          const titleQuery = `${catKey(category)}_${cmdKey(command)}__btn_name`;
          let buttonTitle = text(titleQuery);
          if (buttonTitle === titleQuery || buttonTitle === '') buttonTitle = command;
          const mctx: AddModalContext = {
            argModalId,
            category,
            command,
            parentCommand: '',
            subId: 0,
            commandValue: cmdObj,
            commandId: asString(cmdObj['command']),
            buttonTitle,
          };
          return join([
            descBlock,
            html`<button arg_modal_ID="${argModalId}" class="dropdown-btn no-dropdown ${raw(dark)}" id="open-button-${argModalId}" dropdown-commandTag="${command}">
              ${buttonTitle}
            </button>`,
            addArgsModal(ctx, mctx),
          ]);
        }

        // TYPE == multiple: sub-command dropdown.
        const titleQuery = `${catKey(category)}_${cmdKey(command)}__category_name`;
        let buttonTitle = text(titleQuery);
        if (buttonTitle === titleQuery || buttonTitle === '') buttonTitle = category;
        const subs = asArray(cmdObj['commands']).map((subValue, subIndex) => {
          const subObj = asObject(subValue);
          const subArgId = `${argModalId}X${subIndex}`;
          const subId = subIndex + 1;
          const subCommand = rep(asString(subObj['command']), '/', '');
          const subDescQuery = `${catKey(category)}_${cmdKey(command)}__category_description`;
          let subDesc = text(subDescQuery);
          if (subDesc === subDescQuery) subDesc = '';
          const subBtn = asString(subObj['TYPE']).includes('multiple') ? 'category' : 'btn';
          const subTitleQuery = `${catKey(category)}_${cmdKey(command)}_sub${subId}__${subBtn}_name`;
          // NOTE: upstream shadows `command` before building `_command`,
          // which is equivalent to parent + _sub{N} here.
          let subTitle = text(subTitleQuery);
          if (subTitle === subTitleQuery || subTitle === '') subTitle = subCommand;
          const mctx: AddModalContext = {
            argModalId: subArgId,
            category,
            command: subCommand,
            parentCommand: command,
            subId,
            commandValue: subObj,
            commandId: asString(subObj['command']),
            buttonTitle: subTitle,
          };
          return join([
            subDesc !== '' ? html`<div class="addbutton-description"><p>${subDesc}</p></div>` : raw(''),
            html`<button arg_modal_ID="${subArgId}" class="dropdown-btn no-dropdown ${raw(dark)}" id="open-button-${subArgId}" dropdown-commandTag="${subCommand}">
              ${subTitle}
            </button>`,
            addArgsModal(ctx, mctx),
          ]);
        });
        return join([
          descBlock,
          html`<button class="dropdown-btn ${raw(dark)}" dropdown-category="${command}">
            ${buttonTitle}
          </button>
          <div class="dropdown-container">
            <div class="dropdown-item-container">
              ${join(subs)}
            </div>
          </div>`,
        ]);
      });

    return join([
      html`<button class="dropdown-btn ${raw(dark)}" dropdown-category="${categoryName}">
        ${categoryName}
      </button>
      <div class="dropdown-container">
        ${join(items)}
      </div>`,
    ]);
  });

  return join(categories);
}

/** Add-button modal chrome (index.jinja addbutton section). */
export function addModalChrome(ctx: BootContext): Html {
  const dark = ctx.dark_theme;
  return html`
    <div class="addbutton-modal-container ${raw(dark)}">
      <div class="addbutton-modal-content ${raw(dark)}" id="addbutton-modal-content">
        <div class="addbutton-modal-header bold">
          <h1 class="addbutton-modal"> ${text('add_a_button')} </h1>
          <div class="addbutton-modal-close">
            <svg class="addbutton-config-modal ${raw(dark)}" xmlns="http://www.w3.org/2000/svg" width="19" height="19" fill="currentColor" class="bi bi-x-circle-fill" viewBox="0 0 16 16">
              <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z"/>
            </svg>
          </div>
        </div>
        <div class="addbutton-modal-main">
          <br class="addbutton-container ${raw(dark)}" />
          <div class="all-commands ${raw(dark)}">
            ${addBrowserView(ctx)}
          </div>
        </div>
      </div>
    </div>`;
}

/** One args modal (addbutton_modal.jinja). */
export function addArgsModal(ctx: BootContext, mctx: AddModalContext): Html {
  const dark = ctx.dark_theme;
  const { argModalId: id, commandValue } = mctx;
  const args = asArray(commandValue['args']);

  let argCounter = 0;
  const argBlocks = args.map((argValue, argIndex) => {
    const arg = asObject(argValue);
    if (asString(arg['TYPE']) !== 'text') argCounter++;
    const rctx: ArgsRenderContext = {
      ctx,
      category: mctx.category,
      command: mctx.command,
      subId: mctx.subId,
      parentCommand: mctx.parentCommand,
      argModalId: id,
      commandValue,
    };
    return argBranch(ctx, rctx, arg, argIndex, argCounter);
  });

  const style = asObject(commandValue['style']);
  const hasStyle = Object.keys(commandValue).includes('style') && Object.keys(style).length > 0;
  const btn: 'btn' | 'category' = asString(commandValue['TYPE']).includes('multiple') ? 'category' : 'btn';
  const buttonName = addButtonName(ctx, mctx, btn, hasStyle);

  const styleImage = asString(style['image']);
  const defaultSize =
    hasStyle && asString(style['image_size']).trim() !== ''
      ? asString(style['image_size']).trim().replace('%', '')
      : '75';

  return html`
<div class="addbutton-modal-container-args ${raw(dark)}" id="modal-container-${id}" arg_modal_ID="${id}">
  <div class="addbutton-modal-content-args ${raw(dark)}">
    <div class="addbutton-modal-header-args bold modal-container-${id}">
      <h1 class="addbutton-modal-args"> ${text('configure_your_button')}: ${mctx.buttonTitle}</h1>
      <div class="addbutton-modal-close-args">
        <svg class="addbutton-args-config-modal ${raw(dark)}" xmlns="http://www.w3.org/2000/svg" width="19" height="19" fill="currentColor" class="bi bi-x-circle-fill" viewBox="0 0 16 16">
          <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z"/>
        </svg>
      </div>
    </div>
    <div class="addbutton-modal-main-args">
      <div class="config-container ${raw(dark)}">
        <form class="args-form" arg_modal_ID="${id}" novalidate>
          <div class="args-container ${raw(dark)}" arg_modal_ID="${id}">
            ${join(argBlocks)}
          </div>
          ${args.length > 0 ? html`<div class="editorStyle-bar ${raw(dark)}"></div>` : raw('')}
          <div class="editorStyle">
            <div class="fakeform-container ${raw(dark)}">
              <div class="fakeform">
                ${addPreview(ctx, mctx, buttonName)}
              </div>
            </div>
            <div class="inputs_container">
              <label for="image-input_${id}"> ${text('image')}: </label>
              <input type="file" id="image-input_${id}" class="${raw(dark)}" />
              <div class="slider-container">
                <label for="image-size-slider_${id}"> ${text('image_size')}: </label>
                <input type="range" id="image-size-slider_${id}" class="${raw(dark)}" name="image-size" min="0" max="100" value="${defaultSize}" step="1" />
                <input type="number" id="image-size-value_${id}" class="image-size-value ${raw(dark)}" min="0" step="1" value="${defaultSize}" />
                %
              </div>
              <label for="background-color-input_${id}"> ${text('background_color')}: </label>
              <div class="background-color-input-container">
                <input type="color" class="background-color-input ${raw(dark)}" id="background-color-input_${id}" />
                <input type="text" id="background-color-hex_${id}" class="${raw(dark)}" placeholder="${text('background_color_hex')}" />
              </div>
              <label for="button-text-input_${id}"> ${text('button_title')}: </label>
              <input type="text" id="button-text-input_${id}" class="${raw(dark)}" placeholder="${buttonName}" />
            </div>
          </div>
          <div class="editorStyle-bar ${raw(dark)}" style="display: none;"></div>
          <div class="arg_container" arg_modal_ID="${id}" style="display: none;">
            <label for="command_${id}">Command (experimental):</label>
            <input class="${raw(dark)}" type="text" name="" id="command_${id}" value="${asString(commandValue['command'])}" readonly />
          </div>
          <input type="submit" value="${text('save')}" id="${id}_submit" class="createbutton_submit ${raw(dark)}" style="margin-top: 30;" />
        </form>
      </div>
    </div>
  </div>
</div>`;
}

function addButtonName(ctx: BootContext, mctx: AddModalContext, btn: 'btn' | 'category', hasStyle: boolean): string {
  void ctx;
  const base =
    mctx.subId !== 0
      ? `${catKey(mctx.category)}_${cmdKey(mctx.parentCommand)}_sub${mctx.subId}`
      : `${catKey(mctx.category)}_${cmdKey(mctx.command)}`;
  if (hasStyle) {
    const displayQuery = `${base}__${btn}_default_display_name`;
    const displayName = text(displayQuery);
    if (displayName !== displayQuery && displayName !== '') return displayName;
  }
  return text(`${base}__${btn}_name`);
}

function argBranch(
  ctx: BootContext,
  rctx: ArgsRenderContext,
  arg: JsonObject,
  argIndex: number,
  argCounter: number
): Html {
  const dark = ctx.dark_theme;
  const type = asString(arg['TYPE']);
  const base =
    rctx.subId !== 0
      ? `${catKey(rctx.category)}_${cmdKey(rctx.parentCommand)}_sub${rctx.subId}_`
      : `${catKey(rctx.category)}_${cmdKey(rctx.command)}_`;

  if (type.includes('input')) {
    const argName = text(`${base}arg_${argCounter}_name`);
    const folderForm = type.includes('webdeck_foldername')
      ? html`<div class="webdeck_foldername_div">
                      <form id="webdeck_foldername_form" novalidate>
                        <input class="${raw(dark)}" type="text" id="folderName" name="folderName" placeholder="New folder name" />
                        <button id="submitButton" type="submit">Create folder</button>
                      </form>
                    </div>`
      : raw('');
    return join([
      html`<div class="arg_container" arg_modal_ID="${rctx.argModalId}" arg_id="${String(argIndex)}">
                  <label for="${argName}_${rctx.argModalId}">${argName}:</label>
                  ${argField(rctx, arg, argIndex)}
                </div>`,
      folderForm,
    ]);
  }
  if (type.includes('choice')) {
    const options = asArray(arg['options']);
    const choices = options.map((choiceValue, choiceIndex) => {
      const choice = asObject(choiceValue);
      const choiceId = choiceIndex + 1;
      const choiceName = text(`${base}arg_${argCounter}_option_${choiceId}_name`);
      const isChecked = asString(choice['TYPE']).includes('checked');
      const items = asString(choice['TYPE']).includes('multiple')
        ? join(asArray(choice['items']).map((item) => argField(rctx, asObject(item), choiceIndex)))
        : argField(rctx, choice, choiceIndex);
      return join([
        html`<div class="choice">
                      <input ${isChecked ? raw('checked') : raw('')} class="choice ${raw(dark)}" type="radio" name="choice" value="${String(choiceIndex)}" onchange="showArg_${rctx.argModalId}('${String(choiceIndex)}')" />
                      <label for="${choiceName}">${choiceName}</label>
                    </div>`,
        html`<div ${isChecked ? raw('') : raw('style="display: none;"')} class="arg_container" arg_modal_ID="${rctx.argModalId}" arg_id="${String(choiceIndex)}">
                      ${items}
                    </div>`,
      ]);
    });
    return html`<div class="choices_ALL">
                  <label for="choice">${text('choose_option')} :</label><br />
                  ${join(choices)}
                </div>`;
  }
  if (type.includes('text') && asString(arg['value']) !== '') {
    return html`<input class="invisible" type="text" size="10" value="${asString(arg['value'])}" />`;
  }
  return raw('');
}

function addPreview(ctx: BootContext, mctx: AddModalContext, buttonName: string): Html {
  const dark = ctx.dark_theme;
  const { argModalId: id, commandValue } = mctx;
  const style = asObject(commandValue['style']);
  const hasStyle = Object.keys(commandValue).includes('style') && Object.keys(style).length > 0;
  const namesColor = asString(get(ctx.config, 'front', 'names_color'));
  const buttontextStyle =
    namesColor !== '' && namesColor.trim() !== '' ? `style="color:${namesColor};"` : '';

  if (!hasStyle) {
    return join([
      html`<button type="button" class="wd_button" role="button" style="overflow: hidden; overflow-y: hidden; max-height: 89.6px;">
                      <img id="button-image_${id}" draggable="false" alt="" style="
                        width: ${String(112 * (50 / 100) + 3)}px;
                      " />
                      <div class="usage">
                        <div id="usage-title_${id}" class="usage-title" style=""></div>
                        <div id="usage-value_${id}" class="usage-value" style=""></div>
                      </div>
                    </button>`,
      html`<p class="buttontext" id="button-text-preview_${id}">
                      ${buttonName}
                    </p>`,
    ]);
  }

  let fill = '';
  if ('color' in style) {
    const color = asString(style['color']);
    fill = color === 'invert' ? 'filter: invert(1)' : `fill:${color}; color:${color};`;
  }
  const styleImage = asString(style['image']);
  let media: Html;
  if (styleImage === '') {
    media = html`<img id="button-image_${id}" draggable="false" alt="" style="width: ${String(112 * (50 / 100) + 3)}px;" />`;
  } else {
    const imagelink = addImageLink(styleImage);
    const sizeNum = parseInt(rep(asString(style['image_size']), '%', ''), 10);
    const px = 112 * (sizeNum / 100) + 3;
    media = imagelink.endsWith('.svg')
      ? svgSlot(imagelink, ` id="button-image_${id}" style="width:${px}px; height:${px}; ${fill}"`, '<svg')
      : html`<img id="button-image_${id}" src="${imagelink}" draggable="false" style="
                            width: ${String(px)}px;
                            ${fill}"
                          />`;
  }
  return join([
    html`<button type="button" id="button-element_${id}" class="wd_button" role="button" style="overflow: hidden; overflow-y: hidden; max-height: 89.6px;">
                      ${media}
                      <div class="usage">
                        <div id="usage-title_${id}" class="usage-title" style="${fill}"></div>
                        <div id="usage-value_${id}" class="usage-value" style="${fill}"></div>
                      </div>
                    </button>`,
    html`<p class="buttontext" id="button-text-preview_${id}" ${raw(buttontextStyle)}>
                      ${buttonName}
                    </p>`,
  ]);
}

function addImageLink(styleImage: string): string {
  if (styleImage.startsWith('http')) return styleImage;
  if (styleImage.includes(':')) return 'static/img/' + (styleImage.split('\\').pop() ?? styleImage);
  if (styleImage.startsWith('**uploaded/')) {
    // Upstream reads an out-of-scope config path here (renders broken);
    // use the style image itself so uploaded art actually previews.
    return '.config/user_uploads/' + rep(styleImage, '**uploaded/', '');
  }
  return 'static/img/' + styleImage;
}

// --- Wire-up (per-modal <script> block) -------------------------------------

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

function getCommand(command: string, argModalId: string): string {
  const form = document.querySelector(`form[arg_modal_ID="${argModalId}"] .args-container`);
  const inputs = form?.querySelectorAll('input, select, textarea') ?? [];
  const values = Array.from(inputs)
    .filter((input) => {
      const parent = input.parentElement;
      if (parent && window.getComputedStyle(parent).display === 'none') {
        return false;
      }
      if (input.classList.contains('choice')) {
        return false;
      }
      const ancestorDivs = input.closest('.editorStyle, .webdeck_foldername_div');
      if (ancestorDivs) {
        return false;
      }
      return true;
    })
    .map((input) => {
      const el = input as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
      if (el.tagName === 'SELECT') {
        const select = el as HTMLSelectElement;
        return select.options[select.selectedIndex]?.value ?? '';
      } else if ((el as HTMLInputElement).type === 'radio' || (el as HTMLInputElement).type === 'checkbox') {
        return (el as HTMLInputElement).checked ? (el as HTMLInputElement).value : '';
      } else if ((el as HTMLInputElement).type === 'submit' || (el as HTMLInputElement).type === 'button') {
        return '';
      } else {
        const field = el as HTMLInputElement;
        if (field.value.startsWith('/folder')) {
          field.value = field.value.replace(/"/g, '');
        }
        return field.value;
      }
    })
    .filter((value) => value !== '');

  return command + ' ' + values.join('<|§|>');
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

  (window as unknown as Record<string, unknown>)[`showArg_${id}`] = (argId: string) => {
    const elements = document.querySelectorAll(`div.arg_container[arg_modal_ID="${id}"]:not([arg_id="${argId}"])`);
    elements.forEach(function (element) {
      if (element.getAttribute('arg_id') === argId) {
        (element as HTMLElement).style.display = 'block';
      } else {
        (element as HTMLElement).style.display = 'none';
      }
    });
  };

  wireFoldernameForm();
  wireUsagePreview(id);

  document.getElementById(`image-input_${id}`)?.addEventListener('change', function () {
    const input = this as unknown as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const formData = new FormData();
    formData.append('file', file);

    const xhr = new XMLHttpRequest();
    xhr.open('POST', '/upload_file', true);
    xhr.onload = function () {
      if (xhr.status === 200) {
        console.log('File downloaded successfully!');
        const swapped = swapPreviewImage(id, input);
        if (!swapped) return;
        buttonState['image_size'] = '70';
        updateImageSize(swapped.slider, swapped.value, swapped.image, buttonState);
        buttonState['image'] = '**uploaded/' + (input.files?.[0]?.name ?? '');
      } else {
        console.error('Failed to download file.');
      }
    };
    xhr.send(formData);
  });

  const image = document.getElementById(`button-image_${id}`) as HTMLElement | null;
  const imageSizeSlider = document.getElementById(`image-size-slider_${id}`) as HTMLInputElement | null;
  const imageSizeValue = document.getElementById(`image-size-value_${id}`) as HTMLInputElement | null;
  if (image && imageSizeSlider && imageSizeValue) {
    updateImageSize(imageSizeSlider, imageSizeValue, image, buttonState);
  }

  const buttonElement = document.getElementById(`button-element_${id}`) as HTMLElement | null;
  const bgInput = document.getElementById(`background-color-input_${id}`) as HTMLInputElement | null;
  const bgHex = document.getElementById(`background-color-hex_${id}`) as HTMLInputElement | null;
  if (buttonElement && bgInput && bgHex) {
    updateButtonBackgroundColor(buttonElement, bgInput, bgHex, buttonState);
  }

  if (devMode) {
    document.getElementById(`command_${id}`)?.addEventListener('input', function () {
      buttonState['message'] = (this as unknown as HTMLInputElement).value;
    });
  }

  const buttonText = document.getElementById(`button-text-input_${id}`) as HTMLInputElement | null;
  const buttonPreview = document.getElementById(`button-text-preview_${id}`);
  buttonText?.addEventListener('input', function () {
    const textValue = buttonText.value;
    if (buttonPreview) buttonPreview.textContent = textValue;
    buttonState['name'] = textValue;
  });

  document.getElementById(`${id}_submit`)?.addEventListener('click', function (event) {
    if (id !== 'NONE') {
      event.preventDefault();
    }
    setTimeout(function () {
      buttonCommandAdd(id, mctx.commandId);
    }, 1000);
  });
}

/** Folder-creation form (document-first matching reproduces upstream). */
function wireFoldernameForm(): void {
  document.getElementById('submitButton')?.addEventListener('click', function (event) {
    event.preventDefault();
    const folderName = (document.getElementById('folderName') as HTMLInputElement | null)?.value ?? '';
    if (folderName.trim() !== '') {
      const parentFolderEl = document.querySelector('.buttons-center:not(.invisible)');
      const parentFolder = (parentFolderEl?.id ?? '').replace(/^folder-/, '');
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
              const divs = document.querySelectorAll('.webdeck_foldername_ALL');
              divs.forEach(function (div) {
                const newDiv = document.createElement('div');
                newDiv.setAttribute('class', 'webdeck_foldername');
                const input = document.createElement('input');
                input.setAttribute('type', 'radio');
                input.setAttribute('name', 'file');
                input.setAttribute('value', folderName);
                input.setAttribute('required', '');
                const label = document.createElement('label');
                label.setAttribute('for', folderName);
                label.textContent = folderName;
                newDiv.appendChild(input);
                newDiv.appendChild(label);
                div.appendChild(newDiv);
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

function wireUsagePreview(id: string): void {
  const usageInput = document.getElementById(`usage-title-input_${id}`) as HTMLInputElement | null;
  if (!usageInput) return;
  const usageTitlePreview = document.getElementById(`usage-title_${id}`);
  const usageValuePreview = document.getElementById(`usage-value_${id}`);
  if (!usageTitlePreview || !usageValuePreview) return;

  const initial = usageInput.value;
  usageTitlePreview.textContent = initial;
  usageValuePreview.textContent = '-';

  usageInput.addEventListener('input', function () {
    const state = addModalStates.get(id);
    usageTitlePreview.textContent = usageInput.value;
    if (state) state.button['name'] = usageInput.value;
  });

  reloadUsagePreview(id);
  const diskLetter = document.getElementById(`disk-letter_${id}`);
  diskLetter?.addEventListener('change', function () {
    console.log('changed!');
    usageValuePreview.textContent = '-';
    const titleSecond = usageTitlePreview.classList.item(1);
    const valueSecond = usageValuePreview.classList.item(1);
    if (titleSecond) usageTitlePreview.classList.remove(titleSecond);
    if (valueSecond) usageValuePreview.classList.remove(valueSecond);
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
  document.getElementById(`usage-title_${id}`)?.classList.add(`${path}`);
  document.getElementById(`usage-value_${id}`)?.classList.add(`${path}`);
}

function buttonCommandAdd(argModalId: string, command: string): void {
  const state = addModalStates.get(argModalId);
  if (!state) return;
  const commandString = argModalId === 'NONE' ? command : getCommand(command, argModalId);
  console.log(commandString);

  state.button['message'] = commandString;
  console.log(state.button);
  console.log('buttonCommand received, from: add');
  const element = document.querySelector('#addbutton-modal-content');
  const locationFolder = element?.getAttribute('add_FOLDER') ?? '';
  const locationId = element?.getAttribute('add_ID') ?? '';

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

  element?.removeAttribute('add_ID');
  element?.removeAttribute('add_FOLDER');
}

/** Add-modal dropdown toggles (index.jinja inline script after commands). */
export function wireBrowserDropdowns(): void {
  const dropdown = document.getElementsByClassName('dropdown-btn');
  for (const btn of dropdown) {
    btn.addEventListener('click', function (this: Element) {
      if (!(btn.classList.contains('final-btn') || btn.classList.contains('no-dropdown'))) {
        this.classList.toggle('active');
      }
        const dropdownContent = this.nextElementSibling as HTMLElement | null;
        try {
          if (dropdownContent?.style.display === 'block') {
            if (!dropdownContent.classList.contains('addbutton-description')) {
              dropdownContent.style.display = 'none';
            }
          } else if (dropdownContent) {
            dropdownContent.style.display = 'block';
          }
        } catch {
          // Ne rien faire (pass)
        }
      });
  }
}
