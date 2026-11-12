import { readFileSync } from 'node:fs';
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

test('German uses its complete catalogue without old keys', async ({ page, request }) => {
  await page.goto('/#settings');
  await page.locator('input[type=radio][value=de_DE]').check();
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'de');
  const response = await (await request.get('/api/v2/translations')).json();
  expect(response.translations['common.cancel']).toBe('Abbrechen');
  expect(response.translations['common.settings']).toBe('Einstellungen');
  expect(response.translations['guides.audio.steps.wave.title']).not.toBe('Select a fragment');
  await page.locator('.settings-back').click();
  await page.keyboard.press('q');
  await page.getByRole('button', { name: 'Play / pause bearbeiten', exact: true }).click();
  await expect(
    page.locator('dialog.editor').getByRole('button', { name: 'Abbrechen', exact: true }),
  ).toBeVisible();
});

async function spanishSettings(
  page: import('@playwright/test').Page,
  request: import('@playwright/test').APIRequestContext,
) {
  const snapshot = await (await request.get('/api/v2/config')).json();
  snapshot.config.settings.language = 'es_ES';
  expect(
    (
      await request.put('/api/v2/config', {
        data: { revision: snapshot.revision, config: snapshot.config },
      })
    ).ok(),
  ).toBe(true);
  await page.goto('/#settings');
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
}

test('Spanish runtime, library and practice have complete translated copy and retain interpolation', async ({
  page,
  request,
}, info) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await spanishSettings(page, request);
  const original = await (await request.get('/api/v2/config')).json();
  await page.getByRole('tab', { name: 'Motor de ejecución y complementos', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Estado del motor de ejecución', exact: true }),
  ).toBeVisible();
  const runtime = page.locator('#settings-runtime');
  await expect(runtime.getByText('Listo', { exact: true })).toBeVisible();
  await expect(runtime).toContainText(/Comandos: \d+/);
  await expect(runtime).toContainText('JavaScript en entorno aislado');
  await expect(
    runtime.getByRole('button', { name: 'Recargar complementos', exact: true }),
  ).toBeVisible();
  await page.screenshot({ path: info.outputPath('runtime-spanish.png'), fullPage: true });
  await page.getByRole('button', { name: 'Ayuda y tutoriales', exact: true }).click();
  const library = page.getByRole('dialog', { name: 'Aprende a usar WebDeck', exact: true });
  await expect(library).toBeVisible();
  await expect(library).not.toContainText('Try the interactive demo');
  await library.screenshot({ path: info.outputPath('library-spanish.png') });
  await library.getByRole('button', { name: /Prueba la demo interactiva/ }).click();
  const practice = page.getByRole('dialog', { name: 'Panel de práctica', exact: true });
  await practice.getByRole('button', { name: /Escena de descanso/ }).click();
  await expect(practice.getByRole('status')).toHaveText(
    'Se cambió a la escena Escena de descanso.',
  );
  await practice.getByRole('button', { name: /Micrófono/ }).click();
  await expect(practice.getByRole('status')).toHaveText('Micrófono: Desactivado.');
  await practice.getByRole('button', { name: /Grabación/ }).click();
  await expect(practice.getByRole('status')).toHaveText('Grabación: Activado.');
  await practice.getByRole('button', { name: 'Restablecer demo', exact: true }).click();
  await expect(practice).toContainText('Solo demostración · No se envían comandos a tu equipo');
  await expect(practice).not.toContainText(
    /Focus scene|Break scene|Recording|Reset demo|Demo only/,
  );
  await practice.screenshot({ path: info.outputPath('practice-spanish.png') });
  await page.keyboard.press('Escape');
  expect(await (await request.get('/api/v2/config')).json()).toEqual(original);
});

test('Spanish runtime localizes client validation failures without showing internal message keys', async ({
  page,
  request,
}) => {
  await page.route('**/api/v2/runtime', (route) => route.fulfill({ json: { api_version: 2 } }));
  await spanishSettings(page, request);
  await page.getByRole('tab', { name: 'Motor de ejecución y complementos', exact: true }).click();
  const runtime = page.locator('#settings-runtime');
  await expect(runtime.getByRole('alert')).toHaveText(
    'El servidor devolvió una respuesta no válida. Recarga e inténtalo de nuevo.',
  );
  await expect(runtime).not.toContainText('errors.invalidResponse');
});

const shippedLocales = ['en_US', 'es_ES', 'de_DE', 'fr_FR', 'ko_KR', 'pl_PL', 'ru_RU'];
for (const locale of shippedLocales) {
  test(`${locale} has full server coverage and translated settings, practice and HTTP controls on mobile`, async ({
    page,
    request,
  }, info) => {
    const expected: Record<string, string> = JSON.parse(
      readFileSync(`../webdeck/translations/${locale}.json`, 'utf8'),
    );
    const english: Record<string, string> = JSON.parse(
      readFileSync('../webdeck/translations/en_US.json', 'utf8'),
    );
    const text = (key: string, values: Record<string, string> = {}) =>
      expected[key]!.replace(
        /\{([a-zA-Z0-9_]+)\}/g,
        (match, name: string) => values[name] ?? match,
      );
    const snapshot = await (await request.get('/api/v2/config')).json();
    snapshot.config.settings.language = locale;
    expect(
      (
        await request.put('/api/v2/config', {
          data: { revision: snapshot.revision, config: snapshot.config },
        })
      ).ok(),
    ).toBe(true);
    await page.setViewportSize({ width: 1280, height: 900 });
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto('/#settings');
    await expect(page.locator('html')).toHaveAttribute('lang', expected['meta.langCode']!);
    await expect(
      page.getByRole('heading', { name: text('common.settings'), exact: true }),
    ).toBeVisible();
    const server = await (await request.get('/api/v2/translations')).json();
    expect(Object.keys(expected).sort()).toEqual(Object.keys(english).sort());
    expect(server.translations).toEqual(expected);
    await page.getByRole('tab', { name: text('runtime.runtime'), exact: true }).click();
    await expect(
      page.getByRole('heading', { name: text('runtime.runtimeStatus'), exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole('button', { name: text('runtime.reloadPlugins'), exact: true }),
    ).toBeVisible();
    await page.getByRole('button', { name: text('settings.settingsHelp'), exact: true }).click();
    const library = page.getByRole('dialog', { name: text('learn.learn'), exact: true });
    await expect(
      library.getByRole('searchbox', { name: text('guides.library.search'), exact: true }),
    ).toBeVisible();
    await library.locator('.demo-entry').click();
    const practice = page.getByRole('dialog', { name: text('learn.practice'), exact: true });
    await practice.locator('[data-guide=practice-microphone]').click();
    await expect(practice.getByRole('status')).toHaveText(
      text('learn.demoToggle', { control: text('audio.microphone'), state: text('common.off') }),
    );
    await page.setViewportSize({ width: 375, height: 812 });
    await practice
      .getByRole('button', { name: text('learn.resetDemo'), exact: true })
      .scrollIntoViewIfNeeded();
    expect(await practice.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    await practice.screenshot({ path: info.outputPath(`practice-${locale}-mobile.png`) });
    await page.keyboard.press('Escape');
    await page.locator('.settings-back').click();
    await page.keyboard.press('q');
    await page
      .getByRole('button', {
        name: text('deck.editNamedButton', { label: 'Play / pause' }),
        exact: true,
      })
      .click();
    const editor = page.locator('dialog.editor');
    await editor.getByRole('tab', { name: text('editor.action'), exact: true }).click();
    await editor.getByRole('button', { name: text('actions.changeAction'), exact: true }).click();
    await page.locator('[data-action-id="command:fetch"]').click();
    await page
      .locator('.guide-action-picker footer')
      .getByRole('button', { name: text('actions.selectAction'), exact: true })
      .click();
    await expect(
      editor.getByRole('combobox', { name: text('forms.http.method'), exact: true }),
    ).toBeVisible();
    await expect(
      editor.getByRole('button', { name: text('forms.http.addHeader'), exact: true }),
    ).toBeVisible();
    await editor
      .getByRole('button', { name: text('editor.applyToDraft'), exact: true })
      .scrollIntoViewIfNeeded();
    expect(await editor.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await editor.screenshot({ path: info.outputPath(`editor-${locale}-mobile.png`) });
    expect(errors).toEqual([]);
  });
}
