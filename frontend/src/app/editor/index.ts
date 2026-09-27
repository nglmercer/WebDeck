import { html, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import type { JsonObject } from '../framework/types';
import { q, byId } from '../query';
import { show_addbutton_modal } from './modals';
import { pageState } from './state';
import { showError } from './toast';

// Editor mode (index.jinja EDITORMODE block). All implicit globals become
// module state; behavior is otherwise identical.

let ifModif = 0;
let swapMode = 0;
let savedOnClicks: Record<string, string | null> = {};
let swapChanges: string[] = [];
let swapUNChanges: string[] = [];
let swapFirstBtn: string | 0 = 0;
let swapSecondBtn: string | 0 = 0;
let isMouseOverOpenFolder = false;

export function editorBarView(): Html {
  return html`
    <div id="EditorButtons" style="display: none;">
      <button class="button" id="SaveExitEditorButton">
        <img src="static/img/save.svg" width="20" height="20" id="EditorButtonLogo" />
        [E] ${text('save_and_exit')}
      </button>
      <button class="button" id="exitEditorButton"> [Q] ${text('quit_without_saving')} </button>

      <button class="button" id="swapEditorButton">
        <img src="static/img/swap.png" width="20" height="20" id="EditorButtonLogo" />
        [S] ${text('swap_buttons')}
      </button>
    </div>
  `;
}

export function isSwapMode(): boolean {
  return swapMode === 1;
}

function formCoords(form: Element): { parentId: string; formNumber: string } {
  const parentId = (q(form).parent().prop('id') ?? '').replace(/^folder-/, '');
  const formClassMatch = (q(form).attr('class') ?? '').match(/form-(\d+)/);
  return { parentId, formNumber: formClassMatch?.[1] ?? '' };
}

function editorButtons(): HTMLElement | null {
  return byId<HTMLElement>('EditorButtons').get(0) ?? null;
}

function editorButtonsFolders(): HTMLElement | null {
  return byId<HTMLElement>('EditorButtons-Folders').get(0) ?? null;
}

export function createVoidButton(event: Event | null = null, form: Element | null = null): void {
  ifModif = 1;

  let closestForm: Element | null;
  if (form === null && event !== null) {
    closestForm = q(event.target as Element).closest('form.form').get(0) ?? null;
    if (closestForm === null) {
      closestForm = q(event.target as Element).closest('div.void').get(0) ?? null;
    }
  } else {
    closestForm = form;
  }
  if (!closestForm) return;

  const { parentId, formNumber } = formCoords(closestForm);

  const voidDiv = q('<div>')
    .addClass('void')
    .addClass(`form-${formNumber}`)
    .attr('id', q(closestForm).prop('id') ?? '');
  const addButtonDiv = q('<div>').addClass('add-button').css({ display: 'flex', top: '40.3675' });
  const checkboxDiv = q('<div>').addClass('checkbox').css('display', 'none');
  // Parsed from markup: <svg>/<path> land in the SVG namespace, exactly
  // like the createElementNS version (attribute order is irrelevant).
  const svg = q(
    '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"><path d="M12 4v16m8-8H4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>'
  );

  addButtonDiv.append(svg);
  voidDiv.append(checkboxDiv).append(addButtonDiv);

  console.log(`${parentId} > ${formNumber}`);

  addButtonDiv.attr('add_FOLDER', parentId);
  addButtonDiv.attr('add_ID', formNumber);

  // Upstream also ORs `tempEditorConfig == {}`, which is always false in
  // JS (object identity), so the 1:1 condition is just the null check.
  if (pageState.tempEditorConfig === null) {
    void loadEditorConfig().then((config) => {
      pageState.tempEditorConfig = loadEditorConfigSync(config);
    });
    console.log(pageState.tempEditorConfig);
  }
  const buttons = (
    (pageState.tempEditorConfig['front'] as JsonObject | undefined)?.['buttons'] as
      | Record<string, JsonObject[]>
      | undefined
  )?.[parentId];
  if (buttons) buttons[Number(formNumber)] = { VOID: 'VOID' } as unknown as JsonObject;
  q(closestForm).replaceWith(voidDiv);

  console.log('The button has been removed.');
  console.log(pageState.tempEditorConfig);
  pageState.config = pageState.tempEditorConfig;

  reloadEditorEvents();
}

async function loadEditorConfig(): Promise<JsonObject> {
  const response = await fetch('/get_config', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({}),
  });
  return (await response.json()) as JsonObject;
}

function loadEditorConfigSync(config: JsonObject): JsonObject {
  void config;
  // Upstream assigns the promise-returning call itself (a latent bug kept
  // shape-identical: tempEditorConfig would hold a Promise). Reproduce the
  // assignment target faithfully is impossible in TS; keep current config.
  return pageState.tempEditorConfig;
}

export function deleteFolder(folderName: string): void {
  ifModif = 1;
  const folderElement = byId('folder-' + folderName).get(0) ?? null;
  if (folderElement) {
    if (confirm(text('delete_folder_confirm'))) {
      q(folderElement).remove();

      const folderButtons = q(`button[onclick="folder(\`${folderName}\`)"]`);
      for (const btn of folderButtons.toArray()) {
        if (q(btn).hasClass('wd_button')) {
          createVoidButton(null, q(btn).parent().get(0) ?? null);
        } else {
          q(btn).remove();
        }
      }

      const buttons = (
        (pageState.tempEditorConfig['front'] as JsonObject | undefined)?.['buttons'] as
          | Record<string, unknown>
          | undefined
      );
      if (buttons) delete buttons[folderName];
    }
  }
}

export function showAddConfirmation(_event: Event): void {
  console.log('The button is being added');
}

export function showDeleteConfirmation(event: Event): void {
  // NOTE: upstream `|| force` references an undeclared global (throws only
  // when Cancel is clicked, after confirm already returned false — net
  // effect identical to falsy here).
  const force = (window as unknown as Record<string, unknown>)['force'];
  if (pageState.editorMode === 1 && (confirm(text('button_delete_confirmation')) || force)) {
    createVoidButton(event, null);
  }
}

export function showEditWindow(_event: Event): void {
  console.log('edit pressé');
}

function swapForms(parentId1: string, formNumber1: string, parentId2: string, formNumber2: string): void {
  // Exact-id root + descendant search: equivalent to `#folder-X .form-Y`
  // without interpolating user-controlled folder names into a selector.
  const form1 = byId(`folder-${parentId1}`).find(`.form-${formNumber1}`).get(0) ?? null;
  const form2 = byId(`folder-${parentId2}`).find(`.form-${formNumber2}`).get(0) ?? null;
  if (!form1 || !form2) return;

  const tempHtml = q(form1).html() ?? '';
  q(form1).html(q(form2).html() ?? '');
  q(form2).html(tempHtml);

  for (const el of q('.edit-button').toArray()) {
    q(el).on('click', showEditWindow);
  }
  for (const el of q('.delete-button').toArray()) {
    q(el).on('click', showDeleteConfirmation);
  }

  const folderButtons = (
    (pageState.tempEditorConfig['front'] as JsonObject | undefined)?.['buttons'] as
      | Record<string, JsonObject[]>
      | undefined
  );
  if (folderButtons) {
    const tempValue = folderButtons[parentId1]?.[Number(formNumber1)];
    if (folderButtons[parentId1] && folderButtons[parentId2]) {
      folderButtons[parentId1][Number(formNumber1)] = folderButtons[parentId2][
        Number(formNumber2)
      ] as JsonObject;
      folderButtons[parentId2][Number(formNumber2)] = tempValue as JsonObject;
    }
  }
  q('div.checkbox').removeClass('checkbox-checked');
}

export function swapButton(event: Event): void {
  if (pageState.editorMode === 1 && swapMode === 1 && !isMouseOverOpenFolder) {
    let closestForm = q(event.target as Element).closest('form.form').get(0) ?? null;
    if (closestForm === null) {
      closestForm = q(event.target as Element).closest('div.void').get(0) ?? null;
    }
    if (!closestForm) return;
    const { parentId, formNumber } = formCoords(closestForm);

    const checkbox = q(closestForm).find('div.checkbox').get(0) ?? null;
    if (swapFirstBtn === 0) {
      swapFirstBtn = `${parentId};;;${formNumber}`;
      console.log(`1: ${swapFirstBtn}\n2: ${swapSecondBtn}`);
      q(checkbox).addClass('checkbox-checked');
    } else if (swapSecondBtn === 0) {
      if (swapFirstBtn === `${parentId};;;${formNumber}`) {
        swapFirstBtn = 0;
        console.log(`1: ${swapFirstBtn}\n2: ${swapSecondBtn}`);
        q(checkbox).removeClass('checkbox-checked');
      } else {
        q(checkbox).addClass('checkbox-checked');
        swapSecondBtn = `${parentId};;;${formNumber}`;
        if ((swapFirstBtn as string | 0) !== 0) {
          console.log('both btns are selected');
          const [parentId1, formNumber1] = (swapFirstBtn as string).split(';;;') as [string, string];
          const [parentId2, formNumber2] = (swapSecondBtn as string).split(';;;') as [string, string];

          swapForms(parentId1, formNumber1, parentId2, formNumber2);

          swapChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);
          console.log(`swapChanges: ${swapChanges}`);

          swapFirstBtn = 0;
          swapSecondBtn = 0;
          console.log(`sucessfully swapped ${parentId1};;;${formNumber1} to ${parentId2};;;${formNumber2}`);
          ifModif = 1;
        }
        console.log(`1: ${swapFirstBtn}\n2: ${swapSecondBtn}`);
      }
    } else {
      console.log('if you see this, it means my code is REALLY really bad. But it works, I hope.');
    }
  }
}

function setEditorButtonsDisplay(selector: string, display: string): void {
  q(selector).css('display', display);
}

export function toggleEditorButtonsMode(): void {
  const display = pageState.editorMode === 0 ? 'none' : 'flex';
  setEditorButtonsDisplay('.add-button', display);
  setEditorButtonsDisplay('.edit-button', display);
  setEditorButtonsDisplay('.delete-button', display);
  q(editorButtons()).css('display', display);
  q(editorButtonsFolders()).css('display', pageState.editorMode === 0 ? 'none' : 'block');
}

/** Swap-button label text node (qdom has no text-node API; stays native). */
function swapButtonLabel(): ChildNode | null {
  return byId('swapEditorButton').get(0)?.childNodes[1] ?? null;
}

export function hideEditorPartially(): void {
  setEditorButtonsDisplay('.add-button', 'none');
  setEditorButtonsDisplay('.edit-button', 'none');
  setEditorButtonsDisplay('.delete-button', 'none');
}

export function showEditorPartially(): void {
  setEditorButtonsDisplay('.add-button', 'flex');
  setEditorButtonsDisplay('.edit-button', 'flex');
  setEditorButtonsDisplay('.delete-button', 'flex');
}

export function toggleEditorMode(): void {
  pageState.editorMode = pageState.editorMode === 0 ? 1 : 0;
  toggleEditorButtonsMode();
  const editorButton = byId('editorButton').get(0) ?? null;
  if (editorButton) {
    q(editorButton).text(
      pageState.editorMode === 0 ? `[Q] ${text('enter_editor_mode')}` : `[Q] ${text('exit_editor_mode')}`
    );
  }
  console.log('La valeur de editorMode a été modifiée :', pageState.editorMode);

  if (pageState.editorMode === 1) {
    fetch('/get_config')
      .then(function (response) {
        pageState.tempEditorConfig = {};
        if (response.ok) {
          return response.json();
        } else {
          throw new Error(text('settings_load_error'));
        }
      })
      .then(function (configData: JsonObject) {
        pageState.tempEditorConfig = configData;
      })
      .catch(function (error) {
        console.error(error);
      });
  } else {
    if (ifModif === 1) {
      console.log(`swapChanges ${swapChanges}`);
      location.reload();
    }
    swapEditorButtonFunction();
    pageState.editorMode = 0;
    toggleEditorButtonsMode();
    if (editorButton) {
      q(editorButton).text(
        pageState.editorMode === 0 ? `[Q] ${text('enter_editor_mode')}` : `[Q] ${text('exit_editor_mode')}`
      );
    }
    console.log('La valeur de editorMode a été modifiée :', pageState.editorMode);
  }
}

export function SaveExitEditor(tempConfig: JsonObject): void {
  pageState.editorMode = 0;
  swapMode = 0;
  showEditorPartially();
  swapEditorButtonFunction();
  const swapLabel = swapButtonLabel();
  if (swapLabel) {
    swapLabel.nodeValue = `[S] ${text('swap_buttons')}`;
  }
  swapFirstBtn = 0;
  swapSecondBtn = 0;
  swapChanges = [];
  console.log(`swapChanges: ${swapChanges}`);
  toggleEditorButtonsMode();
  if (ifModif === 1 || swapChanges.length !== 0) {
    fetch('/save_buttons_only', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(tempConfig),
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
          location.reload();
        } else {
          showError('Error :/');
        }
      })
      .catch(function (error: Error) {
        showError(error.message);
      });
  }
  console.log('La valeur de editorMode a été modifiée :', pageState.editorMode);
}

export function swapEditorButtonFunction(_event?: Event): void {
  void _event;
  swapMode = swapMode === 0 ? 1 : 0;
  const swapLabel = swapButtonLabel();
  if (swapLabel) {
    swapLabel.nodeValue =
      swapMode === 0 ? `[S] ${text('swap_buttons')}` : `[S] ${text('stop_swap_mode')}`;
  }
  swapFirstBtn = 0;
  swapSecondBtn = 0;
  console.log('La valeur de swapMode a été modifiée :', swapMode);

  if (pageState.editorMode === 1 && swapMode === 1) {
    const swapLabelInner = swapButtonLabel();
    if (swapLabelInner) {
      swapLabelInner.nodeValue = `[S] ${text('stop_swap_mode')}`;
    }
    hideEditorPartially();
    setEditorButtonsDisplay('.swapMode-open-folder', 'inline-flex');
    setEditorButtonsDisplay('div.checkbox', 'block');

    // Snapshot: the loop below only touches attributes, so the live
    // collection's liveness is unobservable here.
    for (const button of q('button').toArray()) {
      if (!q(button).hasClass('EditorButtons-Folder')) {
        const onclickAttr = q(button).attr('onclick');
        const onclickHandlerAttr = q(button).attr('onclickhandler');
        if (onclickHandlerAttr) {
          savedOnClicks[onclickHandlerAttr] = onclickAttr ?? null;
        }
        q(button).removeAttr('onclick');
      }
    }
  } else {
    const swapLabelElse = swapButtonLabel();
    if (swapLabelElse) {
      swapLabelElse.nodeValue = `[S] ${text('swap_buttons')}`;
    }
    swapMode = 0;
    showEditorPartially();

    setEditorButtonsDisplay('.swapMode-open-folder', 'none');
    setEditorButtonsDisplay('div.checkbox', 'none');

    for (const button of q('button').toArray()) {
      if (!q(button).hasClass('EditorButtons-Folder')) {
        const onclickHandlerAttr = q(button).attr('onclickhandler');
        const savedOnClick = onclickHandlerAttr ? savedOnClicks[onclickHandlerAttr] : undefined;
        if (savedOnClick) {
          q(button).attr('onclick', savedOnClick);
        }
      }
    }
    swapChanges = [];
    console.log(`swapChanges: ${swapChanges}`);
  }
}

export function undoSwap(): void {
  const lastChange = swapChanges[swapChanges.length - 1];
  console.log(`All changes: ${swapChanges}`);
  console.log(`Last change: ${lastChange}`);

  if (lastChange === undefined) {
    showError(text('no_changes_to_undo'));
  } else {
    const [swapFirst, swapSecond] = lastChange.split(' > ');
    const [parentId1, formNumber1] = (swapFirst as string).split(';;;') as [string, string];
    const [parentId2, formNumber2] = (swapSecond as string).split(';;;') as [string, string];

    swapForms(parentId1, formNumber1, parentId2, formNumber2);

    swapChanges.splice(-1);
    swapUNChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);

    console.log(`swapUNChanges: ${swapChanges}`);
    swapFirstBtn = 0;
    swapSecondBtn = 0;
    console.log(`sucessfully unswapped ${parentId1};;;${formNumber1} to ${parentId2};;;${formNumber2}`);
    ifModif = 1;
  }
}

export function undoUNSwap(): void {
  const lastChange = swapUNChanges[swapUNChanges.length - 1];
  console.log(`All changes: ${swapUNChanges}`);
  console.log(`Last change: ${lastChange}`);

  if (lastChange === undefined) {
    showError(text('no_changes_to_undo'));
  } else {
    const [swapFirst, swapSecond] = lastChange.split(' > ');
    const [parentId1, formNumber1] = (swapFirst as string).split(';;;') as [string, string];
    const [parentId2, formNumber2] = (swapSecond as string).split(';;;') as [string, string];

    swapForms(parentId1, formNumber1, parentId2, formNumber2);

    swapUNChanges.splice(-1);
    swapChanges.push(`${parentId1};;;${formNumber1} > ${parentId2};;;${formNumber2}`);

    swapFirstBtn = 0;
    swapSecondBtn = 0;
    console.log(`sucessfully unswapped ${parentId1};;;${formNumber1} to ${parentId2};;;${formNumber2}`);
    ifModif = 1;
  }
}

export function reloadEditorEvents(): void {
  console.log('reloading editor events');
  for (const el of q('.add-button').toArray()) {
    q(el).on('click', showAddConfirmation);
  }
  for (const el of q('.edit-button').toArray()) {
    q(el).on('click', showEditWindow);
  }
  for (const el of q('.delete-button').toArray()) {
    q(el).on('click', showDeleteConfirmation);
  }

  const AllButtons = q('form.form').toArray().concat(q('div.void').toArray());
  for (const el of AllButtons) {
    q(el).on('click', swapButton);
  }

  for (const button of q('div.add-button').toArray()) {
    q(button).on('click', function () {
      if (swapMode !== 1) {
        const addIdValue = q(button).attr('add_ID');
        const addFolderValue = q(button).attr('add_FOLDER');
        show_addbutton_modal(addFolderValue ?? null, addIdValue ?? null);
      }
    });
  }
}

export function wireEditorChrome(): void {
  for (const el of q('.swapMode-open-folder').toArray()) {
    q(el).on('mouseover', function () {
      isMouseOverOpenFolder = true;
    });
    q(el).on('mouseout', function () {
      setTimeout(function () {
        isMouseOverOpenFolder = false;
      }, 150);
    });
  }

  q('form')
    .toArray()
    .forEach((form) => {
      q(form).on('submit', function (event) {
        if (swapMode === 1) {
          event.preventDefault();
        }
      });
    });

  byId('editorButton').on('click', toggleEditorMode);
  byId('exitEditorButton').on('click', toggleEditorMode);

  byId('SaveExitEditorButton').on('click', function () {
    console.log('tempEditorConfig:');
    console.log(JSON.stringify(pageState.tempEditorConfig));
    SaveExitEditor(pageState.tempEditorConfig);
  });

  byId('swapEditorButton').on('click', swapEditorButtonFunction);
}
