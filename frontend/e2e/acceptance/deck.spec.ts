import { test, expect } from '@playwright/test';
import { io } from 'socket.io-client';

test('real server: navigation, one click, edit persistence, conflicts, settings and cleanup', async ({ page, request }) => {
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByRole('button',{name:'Work folder',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Work folder',exact:true}).click();
  await expect(page.getByRole('button',{name:'Home folder',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Home folder',exact:true}).click();
  let commands = 0; page.on('request',r => { if (new URL(r.url()).pathname === '/api/v2/commands') commands++; });
  await page.getByRole('button',{name:'Debug action',exact:true}).click();
  await expect.poll(() => commands).toBe(1);
  await page.keyboard.press('q');
  const tile = page.locator('form.form').filter({has:page.getByRole('button',{name:'Debug action',exact:true})});
  await tile.locator('.edit-button').click();
  const modal = page.locator('.editbutton-modal-container:visible');
  await expect(modal).toBeVisible();
  await expect(modal.locator('[role=dialog]')).toBeFocused();
  await modal.getByRole('tab',{name:/Appearance|Style/i}).click();
  await modal.locator('input[id^="button-text-input_"]').fill('Edited action');
  await modal.locator('input[id$="_submit"]').click();
  await expect(page.getByRole('button',{name:'Edited action',exact:true})).toBeVisible();
  await page.getByTestId('alert-ok').click();
  await page.reload();
  await expect(page.getByRole('button',{name:'Edited action',exact:true})).toBeVisible();
  // Editor captures a revision; an independent writer must cause a visible
  // conflict while preserving the local draft and the winning disk state.
  await page.keyboard.press('q');
  await page.locator('form.form').filter({has:page.getByRole('button',{name:'Edited action',exact:true})}).locator('.edit-button').click();
  const conflicted = page.locator('.editbutton-modal-container:visible');
  await conflicted.getByRole('tab',{name:/Appearance|Style/i}).click();
  await conflicted.locator('input[id^="button-text-input_"]').fill('Preserved draft');
  const persisted = await (await request.get('/api/v2/config')).json();
  persisted.config.front.names_color = '#abcdef';
  expect((await request.post('/api/v2/config',{data:{revision:persisted.revision,config:persisted.config}})).ok()).toBeTruthy();
  await conflicted.locator('input[id$="_submit"]').click();
  await expect(page.getByRole('alertdialog')).toContainText('draft is preserved');
  await page.getByTestId('alert-ok').click();
  await expect(conflicted.locator('input[id^="button-text-input_"]')).toHaveValue('Preserved draft');
  await page.reload();
  await page.locator('.open-config-modal').click();
  await expect(page.locator('.modal-container [role=dialog]')).toBeFocused();
  await page.getByRole('button',{name:'Visuals',exact:true}).click();
  await page.locator('[name="front.width"]').fill('2');
  await page.locator('[name="front.height"]').fill('2');
  await page.locator('#config-form input[type=submit]').click();
  await expect.poll(async () => (await (await request.get('/api/v2/config')).json()).config.front.width).toBe(2);
  await page.getByTestId('alert-ok').click();
  await page.reload();
  expect((await (await request.get('/api/v2/config')).json()).config.front.buttons.index).toHaveLength(4);
  for (let i=0;i<3;i++) {
    await page.locator('.open-config-modal').click(); await expect(page.locator('.modal-container')).toBeVisible();
    await page.keyboard.press('Escape'); await expect(page.locator('.modal-container')).toBeHidden();
  }
  expect(errors).toEqual([]);
});

test('v2 socket lifecycle, retired namespace rejection, grants, revocation and expiry', async ({ request }) => {
  const base = 'http://127.0.0.1:59996';
  const grant = await (await request.post('/api/v2/devices',{data:{name:'Socket test',capabilities:['read'],ttl_seconds:60}})).json();
  const connect = (namespace:string,token?:string) => io(base+namespace,{auth:{...(token ? {token} : {})},transports:['websocket'],autoConnect:false,reconnection:false});
  const legacy = connect('/'); const socket = connect('/v2',grant.token);
  const connected = (s: ReturnType<typeof connect>) => new Promise<void>((resolve,reject) => { s.once('connect',()=>resolve()); s.once('connect_error',reject); s.connect(); });
  try {
    await expect(connected(legacy)).rejects.toBeDefined(); await connected(socket);
    const events: any[] = [];
    const completed = new Promise<void>(resolve => socket.on('command_result',event => {events.push(event);if(event.state !== 'accepted')resolve();}));
    socket.emit('command',{message:'/debug-send {}',request_id:'socket-1'}); await completed;
    expect(events.map(e=>e.state)).toEqual(['accepted','completed']); expect(events[1].request_id).toBe('socket-1');
    socket.disconnect();
    socket.volatile.emit('command',{message:'/debug-send {}',request_id:'offline-must-not-replay'});
    await connected(socket);
    await new Promise(resolve=>setTimeout(resolve,200));
    expect(events.some(event=>event.request_id==='offline-must-not-replay')).toBe(false);
    const forbidden = new Promise<any>(resolve=>socket.once('command_result',resolve)); socket.emit('command',{message:'/PCshutdown',request_id:'denied'}); expect((await forbidden).code).toBe('forbidden');
    expect((await request.delete('/api/v2/devices/'+grant.device.id)).ok()).toBeTruthy();
    const revoked = new Promise<any>(resolve=>socket.once('command_result',resolve)); socket.emit('command',{message:'/debug-send {}',request_id:'revoked'}); expect((await revoked).code).toBe('unauthorized');
    socket.disconnect(); const reconnect = connect('/v2',grant.token); await expect(connected(reconnect)).rejects.toBeDefined(); reconnect.disconnect();
    const short = await (await request.post('/api/v2/devices',{data:{name:'Expiring',capabilities:['read'],ttl_seconds:2}})).json();
    const expiring = connect('/v2',short.token); await connected(expiring);
    await new Promise(resolve=>setTimeout(resolve,2100));
    const expired = new Promise<any>(resolve=>expiring.once('command_result',resolve)); expiring.emit('command',{message:'/debug-send {}'}); expect((await expired).code).toBe('unauthorized'); expiring.disconnect();
  } finally {legacy.disconnect();socket.disconnect();}
});

test('real server: create and delete a button, themes and backgrounds persist', async ({page,request}) => {
  const errors:string[]=[];page.on('pageerror',error=>errors.push(error.message));
  const snapshot=await(await request.get('/api/v2/config')).json();
  snapshot.config.front.width=3;snapshot.config.front.height=2;snapshot.config.front.buttons={index:[{name:'Settings',message:'/open-config',image:'settings.png'},...Array.from({length:5},()=>({VOID:'VOID'}))]};
  expect((await request.post('/api/v2/config',{data:{revision:snapshot.revision,config:snapshot.config}})).ok()).toBeTruthy();
  await page.goto('/');await expect(page.getByRole('button',{name:'Settings',exact:true})).toBeVisible();
  await page.keyboard.press('q');
  await page.locator('.buttons-center:not(.invisible) [data-testid=add-slot]').first().click();
  await expect(page.locator('.addbutton-modal-container')).toBeVisible();
  await page.locator('#addbutton-search').fill('key');
  await page.locator('[data-testid=add-leaf]').filter({hasText:/key/i}).first().click();
  const args=page.locator('.addbutton-modal-container-args:visible');await expect(args).toBeVisible();
  await args.getByRole('tab',{name:/Appearance|Style/i}).click();
  await args.locator('input[id^="button-text-input_"]').fill('Created button');
  await args.getByTestId('add-args-save').click();
  await page.getByTestId('alert-ok').click();
  await expect(page.getByRole('button',{name:'Created button',exact:true})).toBeVisible();
  await page.reload();await expect(page.getByRole('button',{name:'Created button',exact:true})).toBeVisible();
  await page.keyboard.press('q');
  await page.locator('form.form').filter({has:page.getByRole('button',{name:'Created button',exact:true})}).locator('.delete-button').click();
  await page.getByTestId('confirm-ok').click();
  await expect(page.getByRole('button',{name:'Created button',exact:true})).toHaveCount(0);
  await page.locator('#SaveExitEditorButton').click();await page.getByTestId('alert-ok').click();
  await page.reload();await expect(page.getByRole('button',{name:'Created button',exact:true})).toHaveCount(0);
  await page.getByRole('button',{name:'Settings',exact:true}).click();
  await page.getByTestId('lib-tab-themes').click();
  await page.locator('[data-filename="//acceptance.css"] .enable-theme-hitbox').click();
  await page.getByTestId('lib-tab-backgrounds').click();
  await page.locator('#background-color-hex').fill('#123456');await page.locator('#create-color-bg').click();
  await expect(page.locator('[data-background="#123456"]')).toBeVisible();
  await page.locator('#config-form input[type=submit]').click();await page.getByTestId('alert-ok').click();
  const saved=await(await request.get('/api/v2/config')).json();expect(saved.config.front.themes).toContain('acceptance.css');expect(saved.config.front.background).toContain('#123456');
  await page.reload();await page.getByRole('button',{name:'Settings',exact:true}).click();await page.getByTestId('lib-tab-backgrounds').click();
  await expect(page.locator('[data-background="#123456"]')).toBeVisible();expect(errors).toEqual([]);
});


test('paired controller loads deck without settings access or integration secrets', async ({page,request}) => {
  const snapshot=await(await request.get('/api/v2/config')).json();
  snapshot.config.settings.obs={host:'localhost',port:4455,password:'private-test-password'};
  snapshot.config.settings.spotify_api={client_id:'private-test-id',client_secret:'private-test-secret',username:'private-test-user'};
  expect((await request.post('/api/v2/config',{data:{revision:snapshot.revision,config:snapshot.config}})).ok()).toBeTruthy();
  const grant=await(await request.post('/api/v2/devices',{data:{name:'Read controller',capabilities:['read','input'],ttl_seconds:60}})).json();
  await page.addInitScript(token=>sessionStorage.setItem('webdeck.device-token',token),grant.token);
  const response=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/v2/boot');
  await page.goto('/');
  const boot=await(await response).json();
  expect(boot.can_edit).toBe(false);
  expect(boot.config.settings.obs).toBeUndefined(); expect(boot.config.settings.spotify_api).toBeUndefined();
  await expect(page.locator('form.form').first()).toBeVisible();
  await page.keyboard.press('q'); await expect(page.locator('#config-form')).toHaveCount(0);
  const denied=await request.get('/api/v2/config',{headers:{Authorization:'Bearer '+grant.token}});
  expect(denied.status()).toBe(403);
});
