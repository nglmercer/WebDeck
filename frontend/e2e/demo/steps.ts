import { expect, type Page } from '@playwright/test';
import { vclick, vfill } from './actions';
import { appEventCount, waitForAppEvent } from './events';

// Selector contract (see e2e/README "Identifier conventions"):
// - deck tiles: [data-testid="deck-tile"] + [data-message="<command>"]
// - add slots:  [data-testid="add-slot"]
// - add leaves: [data-testid="add-leaf"] + [dropdown-commandtag="<catalog key>"]
// - lib tabs:   [data-testid="lib-tab-themes|lib-tab-backgrounds"]

/** 1. Boot: loading screen -> grid, usage tiles fill in. */
export async function bootStep(page: Page): Promise<void> {
  await page.goto('/');
  await waitForAppEvent(page, 'boot:ready');
  await expect(page.locator('#button_e0X0')).toBeVisible();
  await expect(page.locator('#folder-index .usage-value').first()).not.toHaveText('-', {
    timeout: 15_000,
  });
  await page.waitForTimeout(1500);
}

/** 2. Folder navigation: index -> spotify -> index. */
export async function folderNavStep(page: Page): Promise<void> {
  await vclick(page, '#folder-index [data-message="/folder spotify"]');
  await expect(page.locator('.buttons-center#folder-spotify')).toBeVisible();
  await page.waitForTimeout(1200);
  await vclick(page, '#folder-spotify [data-message="/folder index"]');
  await expect(page.locator('.buttons-center#folder-index')).toBeVisible();
  await page.waitForTimeout(1000);
}

/** 3. Settings tile opens the config modal; tour the Themes & backgrounds library. */
export async function configLibraryStep(page: Page): Promise<void> {
  await vclick(page, '#folder-index [data-message="/open-config "]');
  await expect(page.locator('.modal-container')).toHaveCSS('display', 'block');
  await page.waitForTimeout(1200);
  await vclick(page, '[data-testid="lib-tab-backgrounds"]');
  await expect(page.locator('#config-lib-pane-backgrounds')).toBeVisible();
  await page.waitForTimeout(1500);
  await vclick(page, '[data-testid="lib-tab-themes"]');
  await expect(page.locator('#config-lib-pane-themes')).toBeVisible();
  await page.waitForTimeout(1500);
  await vclick(page, '.modal-close');
  await expect(page.locator('.modal-container')).toBeHidden();
  await page.waitForTimeout(800);
}

/** 4. Editor mode, rename a button, save in place (no reload). */
export async function renameButtonStep(page: Page): Promise<void> {
  const entered = await appEventCount(page);
  await page.keyboard.press('q');
  await waitForAppEvent(page, 'editor:changed', entered);
  await expect(page.locator('#EditorButtons')).toBeVisible();
  await page.waitForTimeout(1000);
  await vclick(page, '.edit-button[edit_modal_ID="e0X0"]');
  await expect(page.locator('#edit-modal-container-e0X0')).toHaveCSS('display', 'block');
  await page.waitForTimeout(1000);
  // The modal opens on the Parameters tab; the name field lives on Appearance.
  await vclick(page, '#edit-e0X0-tab-look');
  await expect(page.locator('#button-text-input_e0X0')).toBeVisible();
  await page.waitForTimeout(800);
  await vfill(page, '#button-text-input_e0X0', 'My Media Folder');
  await page.waitForTimeout(1000);
  const saved = await appEventCount(page);
  await vclick(page, '#e0X0_submit');
  // Save closes the modal and re-renders the grid in place: editor stays on,
  // no ?edit=true reload, and the renamed tile shows the new name.
  await waitForAppEvent(page, 'save:completed', saved);
  await waitForAppEvent(page, 'app:refreshed', saved);
  await expect(page.locator('#edit-modal-container-e0X0')).not.toHaveCSS('display', 'block', {
    timeout: 15_000,
  });
  await expect(page.locator('#EditorButtons')).toBeVisible({ timeout: 15_000 });
  await expect(page.locator('#folder-index form#e0X0').getByText('My Media Folder')).toBeVisible({
    timeout: 15_000,
  });
  await expect(page).not.toHaveURL(/edit=true/);
  await page.waitForTimeout(1000);
}

/**
 * 5. Add a new button: void slot -> search -> pick command -> save.
 * Slot 8 is empty in the fixtures, so it renders a plus tile in editor mode.
 * Submit re-renders in place; the stateful mocks replay the saved grid.
 */
export async function addButtonStep(page: Page): Promise<void> {
  await vclick(page, '#folder-index [data-testid="add-slot"] >> nth=0');
  await expect(page.locator('.addbutton-modal-container')).toHaveCSS('display', 'block');
  await page.waitForTimeout(1000);
  // Live search filters + auto-expands matching categories.
  await vfill(page, '#addbutton-search', 'lock');
  await page.waitForTimeout(1200);
  const leaf = '[data-testid="add-leaf"][dropdown-commandtag="Lock session"]';
  const leafLoc = page.locator(leaf);
  await expect(leafLoc).toBeVisible();
  // The leaf carries its args-modal id: scope to it instead of hardcoding
  // catalog indices (and `:visible` is vacuous here — hidden modals keep
  // layout with opacity 0, so assert the display flip instead).
  const argId = await leafLoc.getAttribute('arg_modal_ID');
  const argsModal = `#modal-container-${argId}`;
  await vclick(page, leaf);
  await expect(page.locator(argsModal)).toHaveCSS('display', 'block');
  await page.waitForTimeout(1200);
  const saved = await appEventCount(page);
  await vclick(page, `${argsModal} [data-testid="add-args-save"]`);
  await waitForAppEvent(page, 'save:completed', saved);
  await waitForAppEvent(page, 'app:refreshed', saved);
  await expect(page.locator('#EditorButtons')).toBeVisible({ timeout: 15_000 });
  // Slot 8 was the first void slot; the new tile lands there with the picked command.
  const tile = page.locator('#folder-index #button_e0X8');
  await expect(tile).toBeVisible({ timeout: 15_000 });
  expect(await tile.getAttribute('data-message')).toMatch(/^\/locksession/);
  await page.waitForTimeout(1000);
}

/** 6. Swap mode: pick two tiles, watch them trade places, exit swap mode. */
export async function swapStep(page: Page): Promise<void> {
  await vclick(page, '#swapEditorButton');
  await expect(page.locator('body.swap-active')).toHaveCount(1, { timeout: 10_000 });
  await page.waitForTimeout(800);
  await vclick(page, '#button_e0X1');
  await expect(page.locator('.swap-picked')).toHaveCount(1);
  await page.waitForTimeout(800);
  await vclick(page, '#button_e0X2');
  await page.waitForTimeout(1200);
  await vclick(page, '#swapEditorButton');
  await expect(page.locator('body.swap-active')).toHaveCount(0, { timeout: 10_000 });
  await page.waitForTimeout(800);
}

/** 7. Save & exit the editor, then press a command button. */
export async function saveExitStep(page: Page): Promise<void> {
  const saved = await appEventCount(page);
  await vclick(page, '#SaveExitEditorButton');
  await waitForAppEvent(page, 'save:completed', saved);
  await waitForAppEvent(page, 'app:refreshed', saved);
  await expect(page.locator('#EditorButtons')).toBeHidden();
  await page.waitForTimeout(1000);
  await vclick(page, '#folder-index [data-message="/colorpicker lang:en"]');
  // Let the usage poll repopulate the tiles (refresh restarted it) before closing.
  await expect(page.locator('#folder-index .usage-value').first()).not.toHaveText('-', {
    timeout: 15_000,
  });
  await page.waitForTimeout(1500);
}
