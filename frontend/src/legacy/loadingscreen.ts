// Port of static/js/loadingscreen.js. Runs after first render (the SPA
// boot replaces the window-load timing with identical sequencing).
import { q, byId } from '../query';

export function initLoadingScreen(): void {
  const loadingScreen = byId('loading-screen').get(0) ?? null;
  const serverDisconnected = byId('server-disconnected').get(0) ?? null;
  if (!loadingScreen || !serverDisconnected) return;

  q(loadingScreen).addClass('hidden');
  setTimeout(function () {
    q(serverDisconnected).removeClass('invisible');
    q(loadingScreen).addClass('transparent');
    q(loadingScreen).css('pointerEvents', 'none');
  }, 5000);
}
