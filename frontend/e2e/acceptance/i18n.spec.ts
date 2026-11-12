import { test, expect } from './fixtures';

test('grouped JSON changes the live interface and local validation to Spanish', async ({
  page,
  request,
}) => {
  await page.goto('/#settings');
  await page.locator('input[type=radio][value=es_ES]').check();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
  await expect(page.getByRole('heading', { name: 'Configuración', exact: true })).toBeVisible();
  const response = await (await request.get('/api/v2/translations')).json();
  expect(response.translations['common.save']).toBe('Guardar cambios');
  expect(response.translations['meta.langCode']).toBe('es');
  expect(
    Object.keys(response.translations).every((key) => key.includes('.') && !key.startsWith('ui_')),
  ).toBe(true);
  await page.locator('.settings-back').click();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Editar Play / pause', exact: true }).click();
  const editor = page.locator('dialog.editor');
  await expect(editor.getByRole('button', { name: 'Cancelar', exact: true })).toBeVisible();
  await editor.getByRole('tab', { name: 'Acción', exact: true }).click();
  await editor.getByRole('button', { name: 'Cambiar acción', exact: true }).click();
  await page.locator('[data-action-id="command:key"]').click();
  await page
    .locator('.guide-action-picker footer')
    .getByRole('button', { name: 'Seleccionar acción', exact: true })
    .click();
  await editor.getByRole('button', { name: 'Elegir teclas', exact: true }).click();
  const picker = page.locator('.key-selector .picker');
  const primary = picker.getByRole('button', { name: 'F1', exact: true });
  await primary.click();
  await primary.click();
  await picker.getByRole('button', { name: 'Ctrl', exact: true }).click();
  await expect(picker.getByRole('button', { name: 'Usar atajo', exact: true })).toBeDisabled();
  await expect(picker.getByRole('status')).toContainText('tecla principal');
});

test('partial locales reuse shared labels and fall back per leaf without old keys', async ({
  page,
  request,
}) => {
  await page.goto('/#settings');
  await page.locator('input[type=radio][value=de_DE]').check();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'de');
  const response = await (await request.get('/api/v2/translations')).json();
  expect(response.translations['common.cancel']).toBe('Abbrechen');
  expect(response.translations['common.settings']).toBe('Settings');
  expect(response.translations['guides.audio.steps.wave.title']).toBe('Select a fragment');
  await page.locator('.settings-back').click();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Edit Play / pause', exact: true }).click();
  await expect(
    page.locator('dialog.editor').getByRole('button', { name: 'Abbrechen', exact: true }),
  ).toBeVisible();
});
