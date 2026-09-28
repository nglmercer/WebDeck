import { beforeEach, describe, expect, it } from 'vitest';
import { initI18n } from '../../framework/i18n';
import { pageState } from '../state';
import { editorUiState } from './state';
import { swapButton, swapEditorButtonFunction } from './swap';

function mount(): void {
  document.body.innerHTML = `
    <button id="swapEditorButton"><img />[S] x</button>
    <span id="swapHint" style="display: none;"></span>
    <div id="folder-x">
      <form class="form-0 form" id="e0X0"><div class="checkbox" style="display: none;"></div><span>A</span></form>
      <form class="form-1 form" id="e0X1"><div class="checkbox" style="display: none;"></div><span>B</span></form>
    </div>`;
}

function reset(): void {
  mount();
  initI18n({});
  pageState.editorMode = 0;
  pageState.tempEditorConfig = { front: { buttons: { x: [{ v: 'a' }, { v: 'b' }] } } };
  editorUiState.ifModif = 0;
  editorUiState.swapMode = 0;
  editorUiState.savedOnClicks = {};
  editorUiState.swapChanges = [];
  editorUiState.swapUNChanges = [];
  editorUiState.swapFirstBtn = 0;
  editorUiState.swapSecondBtn = 0;
  editorUiState.isMouseOverOpenFolder = false;
  document.body.classList.remove('swap-active');
}

function click(el: Element): void {
  swapButton({ target: el } as unknown as Event);
}

function form(n: number): HTMLFormElement {
  return document.querySelector(`#folder-x .form-${String(n)}`) as HTMLFormElement;
}

describe('swapEditorButtonFunction', () => {
  beforeEach(reset);

  it('toggles the swap-active body class, hint, and checkboxes', () => {
    pageState.editorMode = 1;
    const hint = document.querySelector('#swapHint') as HTMLElement;

    swapEditorButtonFunction();
    expect(editorUiState.swapMode).toBe(1);
    expect(document.body.classList.contains('swap-active')).toBe(true);
    expect(hint.style.display).toBe('inline');
    for (const box of document.querySelectorAll('div.checkbox')) {
      expect((box as HTMLElement).style.display).toBe('block');
    }

    swapEditorButtonFunction();
    expect(editorUiState.swapMode).toBe(0);
    expect(document.body.classList.contains('swap-active')).toBe(false);
    expect(hint.style.display).toBe('none');
    for (const box of document.querySelectorAll('div.checkbox')) {
      expect((box as HTMLElement).style.display).toBe('none');
    }
  });

  it('syncs the swap label and hover title with the mode', () => {
    pageState.editorMode = 1;
    const button = document.querySelector('#swapEditorButton') as HTMLElement;

    swapEditorButtonFunction();
    expect(button.textContent).toBe('stop_swap_mode');
    expect(button.getAttribute('title')).toBe('stop_swap_mode (S)');

    swapEditorButtonFunction();
    expect(button.textContent).toBe('swap_buttons');
    expect(button.getAttribute('title')).toBe('swap_buttons (S)');
  });

  it('clears stale pick markers when leaving swap mode', () => {
    pageState.editorMode = 1;
    editorUiState.swapMode = 1;
    click(form(0).querySelector('span') as Element);
    expect(form(0).classList.contains('swap-picked')).toBe(true);

    swapEditorButtonFunction();
    expect(form(0).classList.contains('swap-picked')).toBe(false);
    expect(form(0).querySelector('div.checkbox')!.classList.contains('checkbox-checked')).toBe(
      false
    );
  });
});

describe('swapButton picks', () => {
  beforeEach(() => {
    reset();
    pageState.editorMode = 1;
    editorUiState.swapMode = 1;
  });

  it('rings the first pick and clears it on re-click', () => {
    const target = form(0).querySelector('span') as Element;
    click(target);
    expect(editorUiState.swapFirstBtn).toBe('x;;;0');
    expect(form(0).classList.contains('swap-picked')).toBe(true);
    expect(
      form(0).querySelector('div.checkbox')!.classList.contains('checkbox-checked')
    ).toBe(true);

    click(target);
    expect(editorUiState.swapFirstBtn).toBe(0);
    expect(form(0).classList.contains('swap-picked')).toBe(false);
    expect(
      form(0).querySelector('div.checkbox')!.classList.contains('checkbox-checked')
    ).toBe(false);
  });

  it('swaps both tiles and clears the rings', () => {
    click(form(0).querySelector('span') as Element);
    click(form(1).querySelector('span') as Element);

    expect(form(0).innerHTML).toContain('>B<');
    expect(form(1).innerHTML).toContain('>A<');
    expect(document.querySelectorAll('.swap-picked')).toHaveLength(0);
    expect(document.querySelectorAll('.checkbox-checked')).toHaveLength(0);
    expect(editorUiState.swapChanges).toEqual(['x;;;0 > x;;;1']);
    expect(editorUiState.ifModif).toBe(1);
  });

  it('ignores clicks outside swap mode', () => {
    editorUiState.swapMode = 0;
    click(form(0).querySelector('span') as Element);
    expect(editorUiState.swapFirstBtn).toBe(0);
    expect(form(0).classList.contains('swap-picked')).toBe(false);
  });

  it('lets folder-chip taps navigate instead of selecting', () => {
    form(0).insertAdjacentHTML('afterbegin', '<div class="swapMode-open-folder"><span>Open</span></div>');
    click(form(0).querySelector('.swapMode-open-folder span') as Element);
    expect(editorUiState.swapFirstBtn).toBe(0);
    expect(form(0).classList.contains('swap-picked')).toBe(false);
  });
});

