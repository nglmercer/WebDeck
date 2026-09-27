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
import { q, byId, post } from '../query';
import { editorSaveButton, editorStyleBlock } from '../components/editor';
import { wireKeyField } from '../components/keyfield';
import { renderField, type ArgsRenderContext } from './args';
import { consumesArgNumber, parseArg } from './argschema';
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
        // Plugin entries carry their doc text inline (no `.lang` entry).
        if (buttonDescription === descQuery) buttonDescription = asString(cmdObj['description']);
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
    if (consumesArgNumber(arg)) argCounter++;
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
          ${editorStyleBlock({
            dark,
            id,
            preview: addPreview(ctx, mctx, buttonName),
            defaultSize,
            backgroundColor: '',
            buttonName,
            nameValue: '',
          })}
          <div class="editorStyle-bar ${raw(dark)}" style="display: none;"></div>
          <div class="arg_container" arg_modal_ID="${id}" style="display: none;">
            <label for="command_${id}">Command (experimental):</label>
            <input class="${raw(dark)}" type="text" name="" id="command_${id}" value="${asString(commandValue['command'])}" readonly />
          </div>
          ${editorSaveButton(dark, id)}
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
  const parsed = parseArg(arg);
  const base =
    rctx.subId !== 0
      ? `${catKey(rctx.category)}_${cmdKey(rctx.parentCommand)}_sub${rctx.subId}`
      : `${catKey(rctx.category)}_${cmdKey(rctx.command)}`;

  if (parsed.kind === 'input') {
    const argName = parsed.label ?? text(`${base}__arg_${argCounter}_name`);
    const folderForm = parsed.folderForm
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
                  ${renderField(rctx, parsed.field, argCounter)}
                </div>`,
      folderForm,
    ]);
  }
  if (parsed.kind === 'choice') {
    const choices = parsed.options.map((option, choiceIndex) => {
      const choiceId = choiceIndex + 1;
      const choiceName = option.label ?? text(`${base}__arg_${argCounter}_option_${choiceId}_name`);
      // NOTE: nested fields keep the legacy 0-based choice index as their
      // label id (only nested dropdowns consume it; none exist in
      // commands.json, so this preserves behavior exactly).
      const items = join(option.fields.map((field) => renderField(rctx, field, choiceIndex)));
      return join([
        html`<div class="choice">
                      <input ${option.checked ? raw('checked') : raw('')} class="choice ${raw(dark)}" type="radio" name="choice" value="${String(choiceIndex)}" onchange="showArg_${rctx.argModalId}('${String(choiceIndex)}')" />
                      <label for="${choiceName}">${choiceName}</label>
                    </div>`,
        html`<div ${option.checked ? raw('') : raw('style="display: none;"')} class="arg_container" arg_modal_ID="${rctx.argModalId}" arg_id="${String(choiceIndex)}">
                      ${items}
                    </div>`,
      ]);
    });
    return html`<div class="choices_ALL">
                  <label for="choice">${text('choose_option')} :</label><br />
                  ${join(choices)}
                </div>`;
  }
  if (parsed.kind === 'hidden') {
    return html`<input class="invisible" type="text" size="10" value="${parsed.value}" />`;
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

/** Collect arg values in DOM order (exported for tests; powers submit + usage preview). */
export function getCommand(command: string, argModalId: string): string {
  const inputs = q(`form[arg_modal_ID="${argModalId}"] .args-container`)
    .find('input, select, textarea')
    .toArray();
  const values = inputs
    .filter((input) => {
      if (q(input).parent().css('display') === 'none') {
        return false;
      }
      if (q(input).hasClass('choice')) {
        return false;
      }
      if (q(input).hasClass('key-aux')) {
        return false;
      }
      if (q(input).closest('.editorStyle, .webdeck_foldername_div').length > 0) {
        return false;
      }
      return true;
    })
    .map((input) => {
      if (q(input).is('select')) {
        const select = input as HTMLSelectElement;
        return select.options[select.selectedIndex]?.value ?? '';
      }
      const field = input as HTMLInputElement;
      const kind = q(field).prop('type');
      if (kind === 'radio' || kind === 'checkbox') {
        return q(field).prop('checked') === true ? String(q(field).val() ?? '') : '';
      } else if (kind === 'submit' || kind === 'button') {
        return '';
      } else {
        const current = String(q(field).val() ?? '');
        if (current.startsWith('/folder')) {
          const stripped = current.replace(/"/g, '');
          q(field).val(stripped);
          return stripped;
        }
        return current;
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
    q(`div.arg_container[arg_modal_ID="${id}"]:not([arg_id="${argId}"])`)
      .toArray()
      .forEach(function (element) {
        if (q(element).attr('arg_id') === argId) {
          q(element).css('display', 'block');
        } else {
          q(element).css('display', 'none');
        }
      });
  };

  wireFoldernameForm();
  wireUsagePreview(id);
  wireKeyField(id);

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

/** Folder-creation form (document-first matching reproduces upstream). */
function wireFoldernameForm(): void {
  byId('submitButton').on('click', function (event) {
    event.preventDefault();
    const folderName = String(byId<HTMLInputElement>('folderName').val() ?? '');
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
