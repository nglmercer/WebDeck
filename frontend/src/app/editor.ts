import { html, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import type { JsonObject } from '../framework/types';
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
  const parentId = (form.parentNode as Element).id.replace(/^folder-/, '');
  const formClassMatch = form.className.match(/form-(\d+)/);
  return { parentId, formNumber: formClassMatch?.[1] ?? '' };
}

function editorButtons(): HTMLElement | null {
  return document.getElementById('EditorButtons');
}

function editorButtonsFolders(): HTMLElement | null {
  return document.getElementById('EditorButtons-Folders');
}

export function createVoidButton(event: Event | null = null, form: Element | null = null): void {
  ifModif = 1;

  let closestForm: Element | null;
  if (form === null && event !== null) {
    closestForm = (event.target as Element).closest('form.form');
    if (closestForm === null) {
      closestForm = (event.target as Element).closest('div.void');
    }
  } else {
    closestForm = form;
  }
  if (!closestForm) return;

  const { parentId, formNumber } = formCoords(closestForm);

  const voidDiv = document.createElement('div');
  voidDiv.classList.add('void');
  voidDiv.classList.add(`form-${formNumber}`);
  voidDiv.id = closestForm.id;
  const addButtonDiv = document.createElement('div');
  addButtonDiv.classList.add('add-button');
  addButtonDiv.style.display = 'flex';
  addButtonDiv.style.top = '40.3675';
  const checkboxDiv = document.createElement('div');
  checkboxDiv.classList.add('checkbox');
  checkboxDiv.style.display = 'none';
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
  svg.setAttribute('width', '20');
  svg.setAttribute('height', '20');
  svg.setAttribute('viewBox', '0 0 24 24');
  const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  path.setAttribute('d', 'M12 4v16m8-8H4');
  path.setAttribute('stroke', 'currentColor');
  path.setAttribute('stroke-width', '2');
  path.setAttribute('stroke-linecap', 'round');
  path.setAttribute('stroke-linejoin', 'round');

  svg.appendChild(path);
  addButtonDiv.appendChild(svg);
  voidDiv.appendChild(checkboxDiv);
  voidDiv.appendChild(addButtonDiv);

  console.log(`${parentId} > ${formNumber}`);

  addButtonDiv.setAttribute('add_FOLDER', parentId);
  addButtonDiv.setAttribute('add_ID', formNumber);

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
  closestForm.parentNode?.replaceChild(voidDiv, closestForm);

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
  const folderElement = document.getElementById('folder-' + folderName);
  if (folderElement) {
    if (confirm(text('delete_folder_confirm'))) {
      folderElement.remove();

      const folderButtons = document.querySelectorAll(`button[onclick="folder(\`${folderName}\`)"]`);
      for (let i = 0; i < folderButtons.length; i++) {
        const btn = folderButtons[i];
        if (btn.classList.contains('wd_button')) {
          createVoidButton(null, btn.parentNode as Element | null);
        } else {
          btn.remove();
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
  const form1 = document.querySelector(`#folder-${parentId1} .form-${formNumber1}`);
  const form2 = document.querySelector(`#folder-${parentId2} .form-${formNumber2}`);
  if (!form1 || !form2) return;

  const temp = document.createElement('div');
  temp.innerHTML = form1.innerHTML;
  form1.innerHTML = form2.innerHTML;
  form2.innerHTML = temp.innerHTML;

  const editButtonElements = document.querySelectorAll('.edit-button');
  const deleteButtonElements = document.querySelectorAll('.delete-button');
  for (let i = 0; i < editButtonElements.length; i++) {
    editButtonElements[i].addEventListener('click', showEditWindow);
  }
  for (let i = 0; i < deleteButtonElements.length; i++) {
    deleteButtonElements[i].addEventListener('click', showDeleteConfirmation);
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
  const checkboxDivs = document.querySelectorAll('div.checkbox');
  checkboxDivs.forEach((div) => div.classList.remove('checkbox-checked'));
}

export function swapButton(event: Event): void {
  if (pageState.editorMode === 1 && swapMode === 1 && !isMouseOverOpenFolder) {
    let closestForm = (event.target as Element).closest('form.form');
    if (closestForm === null) {
      closestForm = (event.target as Element).closest('div.void');
    }
    if (!closestForm) return;
    const { parentId, formNumber } = formCoords(closestForm);

    const checkbox = closestForm.querySelector('div.checkbox');
    if (swapFirstBtn === 0) {
      swapFirstBtn = `${parentId};;;${formNumber}`;
      console.log(`1: ${swapFirstBtn}\n2: ${swapSecondBtn}`);
      if (checkbox !== null) {
        checkbox.classList.add('checkbox-checked');
      }
    } else if (swapSecondBtn === 0) {
      if (swapFirstBtn === `${parentId};;;${formNumber}`) {
        swapFirstBtn = 0;
        console.log(`1: ${swapFirstBtn}\n2: ${swapSecondBtn}`);
        if (checkbox !== null) {
          checkbox.classList.remove('checkbox-checked');
        }
      } else {
        if (checkbox !== null) {
          checkbox.classList.add('checkbox-checked');
        }
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

export function toggleEditorButtonsMode(): void {
  const addButtonElements = document.querySelectorAll('.add-button');
  const editButtonElements = document.querySelectorAll('.edit-button');
  const deleteButtonElements = document.querySelectorAll('.delete-button');
  for (let i = 0; i < addButtonElements.length; i++) {
    (addButtonElements[i] as HTMLElement).style.display = pageState.editorMode === 0 ? 'none' : 'flex';
  }
  for (let i = 0; i < editButtonElements.length; i++) {
    (editButtonElements[i] as HTMLElement).style.display = pageState.editorMode === 0 ? 'none' : 'flex';
  }
  for (let i = 0; i < deleteButtonElements.length; i++) {
    (deleteButtonElements[i] as HTMLElement).style.display = pageState.editorMode === 0 ? 'none' : 'flex';
  }
  const bar = editorButtons();
  if (bar) bar.style.display = pageState.editorMode === 0 ? 'none' : 'flex';
  const folders = editorButtonsFolders();
  if (folders) folders.style.display = pageState.editorMode === 0 ? 'none' : 'block';
}

export function hideEditorPartially(): void {
  const editButtonElements = document.querySelectorAll('.edit-button');
  const deleteButtonElements = document.querySelectorAll('.delete-button');
  const addButtonElements = document.querySelectorAll('.add-button');
  for (let i = 0; i < addButtonElements.length; i++) {
    (addButtonElements[i] as HTMLElement).style.display = 'none';
  }
  for (let i = 0; i < editButtonElements.length; i++) {
    (editButtonElements[i] as HTMLElement).style.display = 'none';
  }
  for (let i = 0; i < deleteButtonElements.length; i++) {
    (deleteButtonElements[i] as HTMLElement).style.display = 'none';
  }
}

export function showEditorPartially(): void {
  const editButtonElements = document.querySelectorAll('.edit-button');
  const deleteButtonElements = document.querySelectorAll('.delete-button');
  const addButtonElements = document.querySelectorAll('.add-button');
  for (let i = 0; i < addButtonElements.length; i++) {
    (addButtonElements[i] as HTMLElement).style.display = 'flex';
  }
  for (let i = 0; i < editButtonElements.length; i++) {
    (editButtonElements[i] as HTMLElement).style.display = 'flex';
  }
  for (let i = 0; i < deleteButtonElements.length; i++) {
    (deleteButtonElements[i] as HTMLElement).style.display = 'flex';
  }
}

export function toggleEditorMode(): void {
  pageState.editorMode = pageState.editorMode === 0 ? 1 : 0;
  toggleEditorButtonsMode();
  const editorButton = document.getElementById('editorButton');
  if (editorButton) {
    editorButton.textContent =
      pageState.editorMode === 0 ? `[Q] ${text('enter_editor_mode')}` : `[Q] ${text('exit_editor_mode')}`;
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
      editorButton.textContent =
        pageState.editorMode === 0 ? `[Q] ${text('enter_editor_mode')}` : `[Q] ${text('exit_editor_mode')}`;
    }
    console.log('La valeur de editorMode a été modifiée :', pageState.editorMode);
  }
}

export function SaveExitEditor(tempConfig: JsonObject): void {
  pageState.editorMode = 0;
  swapMode = 0;
  showEditorPartially();
  swapEditorButtonFunction();
  const swapBtn = document.getElementById('swapEditorButton');
  if (swapBtn && swapBtn.childNodes[1]) {
    swapBtn.childNodes[1].nodeValue = `[S] ${text('swap_buttons')}`;
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
  const swapEditorButton = document.getElementById('swapEditorButton');
  if (swapEditorButton && swapEditorButton.childNodes[1]) {
    swapEditorButton.childNodes[1].nodeValue =
      swapMode === 0 ? `[S] ${text('swap_buttons')}` : `[S] ${text('stop_swap_mode')}`;
  }
  swapFirstBtn = 0;
  swapSecondBtn = 0;
  console.log('La valeur de swapMode a été modifiée :', swapMode);

  const checkboxDivs = document.querySelectorAll('div.checkbox');
  const openFolderDivs = document.querySelectorAll('.swapMode-open-folder');
  if (pageState.editorMode === 1 && swapMode === 1) {
    if (swapEditorButton && swapEditorButton.childNodes[1]) {
      swapEditorButton.childNodes[1].nodeValue = `[S] ${text('stop_swap_mode')}`;
    }
    hideEditorPartially();
    for (let i = 0; i < openFolderDivs.length; i++) {
      (openFolderDivs[i] as HTMLElement).style.display = 'inline-flex';
    }
    for (let i = 0; i < checkboxDivs.length; i++) {
      (checkboxDivs[i] as HTMLElement).style.display = 'block';
    }

    const buttons = document.getElementsByTagName('button');
    for (let i = 0; i < buttons.length; i++) {
      if (!buttons[i].classList.contains('EditorButtons-Folder')) {
        const button = buttons[i];
        const onclickAttr = button.getAttribute('onclick');
        const onclickHandlerAttr = button.getAttribute('onclickhandler');
        if (onclickHandlerAttr) {
          savedOnClicks[onclickHandlerAttr] = onclickAttr;
        }
        button.removeAttribute('onclick');
      }
    }
  } else {
    if (swapEditorButton && swapEditorButton.childNodes[1]) {
      swapEditorButton.childNodes[1].nodeValue = `[S] ${text('swap_buttons')}`;
    }
    swapMode = 0;
    showEditorPartially();

    const checkboxDivs2 = document.querySelectorAll('div.checkbox');
    const openFolderDivs2 = document.querySelectorAll('.swapMode-open-folder');
    for (let i = 0; i < openFolderDivs2.length; i++) {
      (openFolderDivs2[i] as HTMLElement).style.display = 'none';
    }
    for (let i = 0; i < checkboxDivs2.length; i++) {
      (checkboxDivs2[i] as HTMLElement).style.display = 'none';
    }

    const buttons = document.getElementsByTagName('button');
    for (let i = 0; i < buttons.length; i++) {
      if (!buttons[i].classList.contains('EditorButtons-Folder')) {
        const button = buttons[i];
        const onclickHandlerAttr = button.getAttribute('onclickhandler');
        const savedOnClick = onclickHandlerAttr ? savedOnClicks[onclickHandlerAttr] : undefined;
        if (savedOnClick) {
          button.setAttribute('onclick', savedOnClick);
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
  const addButtonElements = document.querySelectorAll('.add-button');
  const editButtonElements = document.querySelectorAll('.edit-button');
  const deleteButtonElements = document.querySelectorAll('.delete-button');

  const AllButtons1 = document.querySelectorAll('form.form');
  const AllButtons2 = document.querySelectorAll('div.void');
  const AllButtons = Array.from(AllButtons1).concat(Array.from(AllButtons2));

  for (let i = 0; i < addButtonElements.length; i++) {
    addButtonElements[i].addEventListener('click', showAddConfirmation);
  }
  for (let i = 0; i < editButtonElements.length; i++) {
    editButtonElements[i].addEventListener('click', showEditWindow);
  }
  for (let i = 0; i < deleteButtonElements.length; i++) {
    deleteButtonElements[i].addEventListener('click', showDeleteConfirmation);
  }
  for (let i = 0; i < AllButtons.length; i++) {
    AllButtons[i].addEventListener('click', swapButton);
  }

  const open_modal_addbutton = document.querySelectorAll('div.add-button');
  for (const button of open_modal_addbutton) {
    button.addEventListener('click', function () {
      if (swapMode !== 1) {
        const addIdValue = button.getAttribute('add_ID');
        const addFolderValue = button.getAttribute('add_FOLDER');
        show_addbutton_modal(addFolderValue, addIdValue);
      }
    });
  }
}

export function wireEditorChrome(): void {
  const openFolderDivs = document.querySelectorAll('.swapMode-open-folder');
  for (let i = 0; i < openFolderDivs.length; i++) {
    openFolderDivs[i].addEventListener('mouseover', function () {
      isMouseOverOpenFolder = true;
    });
    openFolderDivs[i].addEventListener('mouseout', function () {
      setTimeout(function () {
        isMouseOverOpenFolder = false;
      }, 150);
    });
  }

  const forms = document.querySelectorAll('form');
  forms.forEach((form) => {
    form.addEventListener('submit', function (event) {
      if (swapMode === 1) {
        event.preventDefault();
      }
    });
  });

  document.getElementById('editorButton')?.addEventListener('click', toggleEditorMode);
  document.getElementById('exitEditorButton')?.addEventListener('click', toggleEditorMode);

  document.getElementById('SaveExitEditorButton')?.addEventListener('click', function () {
    console.log('tempEditorConfig:');
    console.log(JSON.stringify(pageState.tempEditorConfig));
    SaveExitEditor(pageState.tempEditorConfig);
  });

  document.getElementById('swapEditorButton')?.addEventListener('click', swapEditorButtonFunction);
}
