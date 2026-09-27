import { html, join, raw, type Html } from '../framework/html';

/** Reusable form-field components (settings page, editor modals, ...). */

/** Toggle-switch row (`.setting` + `label.switch` pattern). */
export interface SwitchFieldOptions {
  dark: string;
  containerClass: string;
  label: string;
  id: string;
  name: string;
  checked: boolean;
  extraClass?: string | undefined;
}

export function switchField(o: SwitchFieldOptions): Html {
  return html`<div class="setting ${raw(o.containerClass)}${o.extraClass ? raw(` ${o.extraClass}`) : raw('')}">
    <p> ${o.label} </p>
    <label for="${o.id}" class="switch">
      <input class="${raw(o.dark)}" type="checkbox" id="${o.id}" name="${o.name}"${o.checked ? raw(' checked') : raw('')} />
      <span class="slider round"></span>
    </label>
  </div>`;
}

/** Labeled text input, optionally a password field with visibility toggle. */
export interface TextFieldOptions {
  dark: string;
  cls: string;
  label?: string;
  labelFor?: string;
  id: string;
  name: string;
  value: string;
  password?: boolean;
  toggleId?: string;
  placeholder?: string;
}

export function textField(o: TextFieldOptions): Html {
  const label =
    o.label !== undefined
      ? html`<label for="${o.labelFor ?? o.id}"> ${o.label} </label>`
      : raw('');
  // NOTE: the old inline markup carried a duplicated `class` attribute on
  // spotify-username/obs-HOST; browsers ignore the duplicate, so the
  // component emits the single valid one (zero behavior change).
  const field = html`<input class="${o.cls} ${raw(o.dark)}" type="${o.password ? 'password' : 'text'}" id="${o.id}" name="${o.name}"${
    o.value.trim() !== '' ? html` value="${o.value}"` : raw('')
  }${o.placeholder !== undefined ? html` placeholder="${o.placeholder}"` : raw('')} />`;
  if (!o.password || !o.toggleId) return html`${label}${field}`;
  return html`${label}<div class="password-container">${field}<span id="${o.toggleId}" class="show-password" onclick="togglePasswordVisibility('${o.id}', '${o.toggleId}')"></span></div>`;
}

/** Labeled dropdown. */
export interface SelectOption {
  value: string;
  label: string;
  selected: boolean;
}

export interface SelectFieldOptions {
  containerClass: string;
  id: string;
  name: string;
  label?: string;
  options: SelectOption[];
}

export function selectField(o: SelectFieldOptions): Html {
  const options = o.options.map(
    (opt) =>
      html`<option value="${opt.value}"${opt.selected ? raw(' selected') : raw('')}>${opt.label}</option>`
  );
  return html`<div class="setting ${raw(o.containerClass)}">
    ${o.label !== undefined ? html`<label for="${o.id}"> ${o.label} </label>` : raw('')}
    <select id="${o.id}" name="${o.name}">${join(options)}</select>
  </div>`;
}

/** Digit-only number input with a fallback default. */
export interface NumberFieldOptions {
  dark: string;
  id: string;
  name: string;
  label?: string;
  value: string;
  defaultValue?: string;
  min?: string;
  required?: boolean;
  style?: string;
}

export function numberField(o: NumberFieldOptions): Html {
  const effective = o.value.trim() !== '' ? o.value.trim() : (o.defaultValue ?? '');
  return html`${o.label !== undefined ? html`<label for="${o.id}"> ${o.label} </label>` : raw('')}<input${
    o.required ? raw(' required') : raw('')
  } class="${raw(o.dark)}" type="number"${o.min !== undefined ? html` min="${o.min}"` : raw('')} pattern="[0-9]*"${
    o.style !== undefined ? html` style="${o.style}"` : raw('')
  } oninput="this.value = this.value.replaceAll(/[^0-9]/g, '');" id="${o.id}" name="${o.name}"${
    effective !== '' ? html` value="${effective}"` : raw('')
  } />`;
}

/** Normalize a configured color to `#rrggbb` (or '' when unset). */
export function normalizeHexColor(value: string): string {
  const trimmed = value.trim();
  if (trimmed === '') return '';
  return trimmed.startsWith('#') ? trimmed : `#${trimmed}`;
}

/** Synced color-picker + HEX text pair. */
export interface ColorFieldOptions {
  dark: string;
  containerClass: string;
  colorClass: string;
  colorId: string;
  hexClass: string;
  hexId: string;
  hexName?: string;
  placeholder?: string;
  /** Already-normalized `#rrggbb` or '' (see `normalizeHexColor`). */
  value: string;
}

export function colorField(o: ColorFieldOptions): Html {
  const val = o.value !== '' ? html` value="${o.value}"` : raw('');
  // Empty class segments (editor hex inputs carry only the theme class)
  // collapse away so no stray whitespace lands in `class`.
  const colorCls = [o.colorClass, o.dark].filter((c) => c !== '').join(' ');
  const hexCls = [o.hexClass, o.dark].filter((c) => c !== '').join(' ');
  return html`<div class="${raw(o.containerClass)}">
    <input type="color" class="${colorCls}" id="${o.colorId}"${val} />
    <input type="text" id="${o.hexId}"${
      o.hexName !== undefined ? html` name="${o.hexName}"` : raw('')
    } class="${hexCls}"${o.placeholder !== undefined ? html` placeholder="${o.placeholder}"` : raw('')}${val} />
  </div>`;
}

/** Section heading with a tutorial/info link (settings groups). */
export interface InfoTitleOptions {
  label: string;
  labelFor?: string;
  href: string;
  title: string;
  icon: Html;
}

export function infoTitle(o: InfoTitleOptions): Html {
  return html`<div class="settings-title-info">
    <label${o.labelFor !== undefined ? html` for="${o.labelFor}"` : raw('')} style="margin-top: 3px;"> ${o.label} </label>
    <a href="${o.href}" target="_blank" title="${o.title}">${o.icon}</a>
  </div>`;
}
