import { html, join, raw, type Html } from '../framework/html';
import { text } from '../framework/i18n';
import { asArray, asBool, asObject, asString, get, rep, type BootContext } from '../framework/types';
import { q } from '../query';
import { folderDeleteIcon } from '../components/icons';

/** <head> extras that depend on boot data: title + theme stylesheets. */
export function applyHead(ctx: BootContext): void {
  document.title = ctx.is_exe ? 'WebDeck' : 'WebDeck DEV';
  const themes = asArray(get(ctx.config, 'front', 'themes'));
  const head = q('head');
  for (const file of [...themes].reverse()) {
    const name = asString(file);
    if (name.startsWith('//')) continue;
    // The server appends the built-in theme to this same list; it is served
    // from /static, not .config/themes (blindly prefixing 404s).
    // Exact match on purpose: anything else keeps the prefix, so a hostile
    // config value can never turn into an arbitrary remote stylesheet URL.
    const href = name === 'static/css/style.css' ? '/static/css/style.css' : `.config/themes/${name}`;
    head.append(q('<link>').attr({ rel: 'stylesheet', href }));
  }
  // NOTE: socket.io is a bundled `socket.io-client` import in wireup.ts;
  // upstream injected static/js/socketio.js here, but that file never
  // existed (404), so there is nothing to inject 1:1.
}

function backgroundVideo(randomBg: string): Html {
  if (!randomBg.endsWith('.mp4')) return raw('');
  const src = '.config/user_uploads/' + rep(rep(randomBg, '//', ''), '**uploaded/', '');
  return html`
    <div class="background-video">
      <video autoplay muted loop class="background-video">
        <source src="${src}" type="video/mp4" />
        Your browser does not support the video tag...
      </video>
    </div>
  `;
}

function consoleForm(ctx: BootContext): Html {
  if (!asBool(get(ctx.config, 'settings', 'show_console'))) return raw('');
  return html`
    <form class="form">
      <label style="color: white;">Console:</label><br />
      <input type="text" class="message ${raw(ctx.dark_theme)}" /><br />
      <button type="submit">Submit</button>
    </form>
  `;
}

function dynamicStyle(ctx: BootContext): Html {
  const { config, random_bg: randomBg, dark_theme: _dark } = ctx;
  void _dark;
  let css = '';
  if (randomBg && randomBg.trim() !== '') {
    if (randomBg.endsWith('.mp4')) {
      css += `
          body, div.fakeform-container {
            background-color: #00000000;
            background-image: none;
          }`;
    } else if (randomBg.toLowerCase().includes('**uploaded/')) {
      const stripped = rep(randomBg, '**uploaded/', '');
      const url = '.config/user_uploads/' + stripped;
      const extension = stripped.split('.').pop() ?? '';
      const bgImage = '.config/user_uploads/' + rep(stripped, '.' + extension, '-90.' + extension);
      css += `
          body, div.fakeform-container {
            background-color: transparent;
          }
          html::before {
            content: "";
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background-image: url("${url}")!important;
            background-size: cover;
            background-position: center;
            background-color: transparent;
            background-position: center;
            z-index: -1;
          }
          div.fakeform-container {
            background-image: url("${url}")!important;
            background-color: transparent;
          }
          @media (orientation: portrait) {
            html::before {
              background-image: url("${bgImage}")!important;
            }
          }`;
    } else {
      css += `
          body, div.fakeform-container {
            background-color: ${randomBg};
            background-image: none;
          }
          :root {
            --bg-color: ${randomBg};
          }`;
    }
  }

  const portraitRotate = asString(get(config, 'front', 'portrait_rotate'));
  css += `
      @media (orientation: portrait) {
        div.buttons-center {
          transform: rotate(${portraitRotate}deg);
        }
        div.all-buttons {
          width: max-content;
        }
      }`;

  const buttonsColor = asString(get(config, 'front', 'buttons_color'));
  if (buttonsColor !== '' && asBool(get(config, 'front', 'edit_buttons_color'))) {
    const buttonsColorDash = asString(get(config, 'front', 'buttons-color'));
    const flatColor = rep(buttonsColor, '#', '');
    css += `
        .wd_button, .wd_button:focus {
          background-color: ${buttonsColor};
          box-shadow: ${buttonsColorDash} 0 1px 3px 0;
        }
        .wd_button:hover {
          transform: translateY(-1px);
          animation-name: animation-${flatColor};
          animation-duration: 1s;
        }
        .wd_button:active {
          background-color: #${flatColor};
          border-color: rgba(0, 0, 0, 0.15);
          box-shadow: rgba(0, 0, 0, 0.06) 0 2px 4px;
          color: rgba(0, 0, 0, 0.65);
          transform: translateY(0);
        }`;
  }

  const buttons = asObject(get(config, 'front', 'buttons'));
  for (const folderButtons of Object.values(buttons)) {
    for (const buttonConfig of asArray(folderButtons)) {
      const bg = asObject(buttonConfig)['background_color'];
      if (typeof bg === 'string') {
        const flat = rep(bg, '#', '');
        css += `
            @keyframes animation-${flat} {
              100% {background-color: #${flat};}
              100% {background-color: none;}
            }
            .button-${flat}, .button-${flat}:focus {
              background-color: #${flat};
              box-shadow: #${flat} 0 1px 3px 0;
              color: rgba(0, 0, 0, 0.85) !important;
            }
            .button-${flat}:hover {
              transform: translateY(-1px);
              animation-name: animation-${flat};
              animation-duration: 1s;
            }
            .button-${flat}:active {
              background-color: #${flat};
              background: #${flat};
              border-color: rgba(0, 0, 0, 0.15);
              box-shadow: rgba(0, 0, 0, 0.06) 0 2px 4px;
              color: rgba(0, 0, 0, 0.65);
              transform: translateY(0);
            }`;
      }
    }
  }
  return raw(`<style>${css}\n    </style>`);
}

function foldersBar(ctx: BootContext): Html {
  const buttons = asObject(get(ctx.config, 'front', 'buttons'));
  const folders = Object.keys(buttons).map((folderId) => {
    // Upstream replaces '"' then autoescapes (double-escaping to &amp;quot;);
    // interpolating the replaced value reproduces that exactly.
    const safe = rep(folderId, '"', '&quot;');
    return html`
        <div style="display: inline-block; margin-right: 10px;">
          <button class="button EditorButtons-Folder" onclick="folder(\`${safe}\`)" style="display: flex; justify-content: center;align-items: center;">
            ${folderId}
            ${folderDeleteIcon(safe)}
          </button>
        </div>`;
  });
  return html`
    <div id="EditorButtons-Folders" style="color: white; display: none; position: fixed; top: 0; right: 0; text-align: right;">
      ${text('open_folder')}:
      ${join(folders)}
    </div>
  `;
}

/** Top-of-body shell: background, console, dynamic style, folder bar. */
export function shellView(ctx: BootContext): Html {
  return join([backgroundVideo(ctx.random_bg), consoleForm(ctx), dynamicStyle(ctx), foldersBar(ctx)]);
}
