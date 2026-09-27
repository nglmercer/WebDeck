// Port of static/js/loadingscreen.js. Runs after first render (the SPA
// boot replaces the window-load timing with identical sequencing).

export function initLoadingScreen(): void {
  const loadingScreen = document.getElementById('loading-screen');
  const serverDisconnected = document.getElementById('server-disconnected');
  if (!loadingScreen || !serverDisconnected) return;

  loadingScreen.classList.add('hidden');
  setTimeout(function () {
    serverDisconnected.classList.remove('invisible');
    loadingScreen.classList.add('transparent');
    loadingScreen.style.pointerEvents = 'none';
  }, 5000);
}
