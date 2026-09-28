import type { Page } from '@playwright/test';

// Playwright's video encoder does not capture the OS mouse pointer, so the
// tour renders its own: a DOM cursor that follows real mousemove events
// plus a click ripple. Injected via addInitScript so it survives the
// tour's page reloads (editor saves navigate with ?edit=true).
const CURSOR_SCRIPT = `(() => {
  const css = [
    '#demo-cursor{position:fixed;left:0;top:0;width:30px;height:30px;z-index:2147483647;pointer-events:none;transition:transform 70ms linear;}',
    '#demo-cursor svg{width:100%;height:100%;filter:drop-shadow(0 1px 2px rgba(0,0,0,.7));}',
    '#demo-cursor.demo-click svg{transform:scale(.82);transform-origin:top left;}',
    '.demo-ring{position:fixed;width:38px;height:38px;margin:-19px 0 0 -19px;border:3px solid #4da3ff;border-radius:50%;z-index:2147483647;pointer-events:none;animation:demo-ping .45s ease-out forwards;}',
    '@keyframes demo-ping{from{transform:scale(.4);opacity:1;}to{transform:scale(1.3);opacity:0;}}',
  ].join('\\n');
  function inject() {
    if (document.getElementById('demo-cursor')) return;
    const style = document.createElement('style');
    style.textContent = css;
    document.head.appendChild(style);
    const cursor = document.createElement('div');
    cursor.id = 'demo-cursor';
    cursor.innerHTML =
      '<svg viewBox="0 0 24 24"><path d="M5 3l14 7.5-6.2 1.8L9.5 19z" fill="#fff" stroke="#111" stroke-width="1.6" stroke-linejoin="round"/></svg>';
    document.body.appendChild(cursor);
    let x = Math.round(window.innerWidth / 2);
    let y = Math.round(window.innerHeight / 2);
    const place = () => {
      cursor.style.transform = 'translate(' + x + 'px,' + y + 'px)';
    };
    place();
    window.addEventListener('mousemove', (e) => { x = e.clientX; y = e.clientY; place(); }, { passive: true });
    window.addEventListener('mousedown', (e) => {
      cursor.classList.add('demo-click');
      const ring = document.createElement('div');
      ring.className = 'demo-ring';
      ring.style.left = e.clientX + 'px';
      ring.style.top = e.clientY + 'px';
      document.body.appendChild(ring);
      window.setTimeout(() => { ring.remove(); cursor.classList.remove('demo-click'); }, 460);
    }, { passive: true });
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', inject);
  else inject();
})();`;

export async function injectCursor(page: Page): Promise<void> {
  await page.addInitScript({ content: CURSOR_SCRIPT });
}
