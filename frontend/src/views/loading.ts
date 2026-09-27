import { html, join, type Html } from '../framework/html';
import { text } from '../framework/i18n';

/** Loading overlay (index.jinja: loading-screen block). */
export function loadingScreen(svgs: string[]): Html {
  return html`
    <div id="loading-screen">
      <div>
        <p id="server-disconnected" class="invisible">${text('server_disconnected')}...</p>
        <div class="loadingspinner">
          <div id="square1"></div>
          <div id="square2"></div>
          <div id="square3"></div>
          <div id="square4"></div>
          <div id="square5"></div>
        </div>
      </div>
      <div class="invisible">
        ${join(svgs.map((svg) => html`<img src="${svg}" /> `))}
      </div>
    </div>
  `;
}
