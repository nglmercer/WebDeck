# qdom — a modern, typesafe jQuery replacement

Zero-dependency DOM library for this project. Chainable like jQuery, but
with generic element typing, typed events, Promise-based effects, and no
silent footguns.

```ts
import { q, ready } from '../query';

ready(() => {
  // q('input') is Q<HTMLInputElement>: .val() and .prop() are checked.
  q('#save').on('click', () => {
    const name = q('#name').val() ?? '';
    q.post('/send-data', { message: name }).catch(console.error);
  });

  // Delegation keeps `this` typed as the matched descendant.
  q('#list').on('click', '.item', function () {
    q(this).toggleClass('active');
  });

  // Effects are awaitable (Web Animations API under the hood).
  await q('#panel').slideToggle(200);
});
```

## Why not jQuery

| jQuery | qdom |
|---|---|
| Untyped (`any` everywhere without `@types/jquery`) | Generic `Q<T>`; tag literals infer element types |
| Stringly events (`on('click', (e) => …)` untyped) | `on()` narrows via `HTMLElementEventMap` |
| `$(sel)[0]` lies (may be `undefined`) | Index access is honestly `T \| undefined` |
| Silent empty sets hide typos in selectors | Same chaining, but getters return `undefined` explicitly |
| Timer-based fx queue, string easings | Native WAAPI, `Promise<void>`, honors `prefers-reduced-motion` |
| Inserted `<script>` executes | Inserted HTML never executes scripts |
| `$.extend` pollutes prototypes | `__proto__`/`constructor`/`prototype` keys are skipped |
| XHR callbacks | `fetch` + typed `HttpError`, timeouts, abort signals |

## API

**Select:** `q('input')` (typed!), `q('.cls')`, `q(el)`, `q(list)`,
`q('<div>…')` (create), `q(sel, context)`, `q.ready(fn)`, `$q` alias.

**Sets:** `length`, `[i]`, iteration, `get(i?)`, `toArray`, `each`
(`false` breaks), `map` (no flattening), `eq/first/last/slice`,
`is/filter/not/has` (selector, element, set, or predicate).

**Traverse:** `find`, `children`, `parent`, `parents`, `closest`,
`siblings`, `next`, `prev` (immediate sibling kept only when it matches).

**Manipulate:** `html/text` (get/set, function values), `append`,
`prepend`, `before`, `after`, `appendTo`, `prependTo`, `insertBefore`,
`insertAfter` (originals go to the last target, clones elsewhere),
`remove` (purges data + listeners), `detach` (keeps them), `empty`,
`clone(withListeners?)`, `replaceWith`.

**Attributes/forms/data:** `attr` (get/set/bulk, `null` removes),
`removeAttr`, typed `prop`, `val` (multi-select arrays, checkable
arrays), `data` (stored values win over coerced `data-*` attributes),
`removeData`.

**Classes/style/visibility:** `addClass/removeClass/toggleClass` (string,
list, or function; `removeClass()` clears all), `hasClass`, `css`
(camelCase accepted, numbers gain `px` unless unitless), `show/hide/
toggle` (old-display restore, tag-default fallback).

**Events:** `on/one` (typed, multi-type strings, namespaces,
delegation), `off` (omitted criteria are wildcards), `trigger` (bubbling
`CustomEvent` with `detail`), `hover`.

**Effects** (all `Promise<void>`, plus optional `complete`):
`show/hide/toggle(duration)`, `fadeIn/fadeOut/fadeTo/fadeToggle`,
`slideDown/slideUp/slideToggle`, `animate(keyframes, duration|options)`,
`stop(gotoEnd?)`.

**Ajax:** `ajax<T>(url, options?)`, `get/getJSON/post`, typed `HttpError`
(`status`, `statusText`, `url`); plain bodies JSON-encode; `timeout`
rejects with `HttpError(0, 'timeout')`; caller `signal` composes.

**Utils:** `each`, `map`, `extend` (shallow/deep), `contains`.

## Deliberate deviations from jQuery

- Effects return promises instead of the chainable set; there is no fx
  queue — `await` sequences instead of `.delay()`/queue callbacks.
- `show/hide(duration)` tween opacity only (jQuery tweens dimensions
  too); slides tween height + vertical padding (not margins).
- `trigger()` always dispatches a `CustomEvent` and ignores namespaces
  for filtering; `on()` namespaces filter normally.
- `map()` (method and static) does not flatten nested arrays.
- `:hidden`-style checks are display-based only, so they work without a
  layout engine (SSR, happy-dom).
- No `wrap/unwrap`, dimensions/offset/scroll, `triggerHandler`, event-map
  objects, `noConflict`, or Sizzle-only selectors (`:visible`, `:has()`,
  positional `:eq()` …). `querySelectorAll` semantics apply; invalid
  selectors throw real errors.

## Roadmap (not in v1)

`wrap/unwrap`, `width/height/inner/outer`, `offset/position/scrollTop/
scrollLeft`, `triggerHandler`, native per-type constructors in
`trigger()`, `on()` object-map form, `serialize()/serializeArray()`.
