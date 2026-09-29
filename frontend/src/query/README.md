# qdom — a modern, typesafe jQuery replacement

Zero-dependency DOM library for this project. Chainable like jQuery, but
with generic element typing, typed events, and no silent footguns. Only
what the app uses is implemented: Svelte components use native events
and reactivity instead, and HTTP lives in `src/api`.

```ts
import { sendCommand } from '../api/buttons';
import { q } from '../query';

// Modules run deferred, so the DOM is ready here.
// q('input') is Q<HTMLInputElement>: .val() and .prop() are checked.
q('#save').on('click', () => {
  const name = q('#name').val() ?? '';
  sendCommand(name).catch(console.error);
});

// Delegation keeps `this` typed as the matched descendant.
q('#list').on('click', '.item', function () {
  q(this).toggleClass('active');
});
```

## Why not jQuery

| jQuery | qdom |
|---|---|
| Untyped (`any` everywhere without `@types/jquery`) | Generic `Q<T>`; tag literals infer element types |
| Stringly events (`on('click', (e) => …)` untyped) | `on()` narrows via `HTMLElementEventMap` |
| `$(sel)[0]` lies (may be `undefined`) | Index access is honestly `T \| undefined` |
| Silent empty sets hide typos in selectors | Same chaining, but getters return `undefined` explicitly |
| Inserted `<script>` executes | Inserted HTML never executes scripts |
| XHR callbacks | `src/api` (`fetch` + typed `HttpError`, timeouts, abort signals) |

## API

**Select:** `q('input')` (typed!), `q('.cls')`, `q(el)`, `q(list)`,
`q('<div>…')` (create), `q(sel, context)`, `byId(id)` (exact
`getElementById` semantics — safe for ids that are not valid CSS
selectors, e.g. leading digits).

**Sets:** `length`, `[i]`, iteration, `get(i?)`, `toArray`,
`is` (selector, element, set, or predicate).

**Traverse:** `find`, `parent`, `closest`, `next` (immediate sibling
kept only when it matches).

**Manipulate:** `html/text` (get/set, function values), `append`
(originals go to the last target, clones elsewhere), `remove` (purges
listeners), `replaceWith`.

**Attributes/forms:** `attr` (get/set/bulk, `null` removes),
`removeAttr`, typed `prop`, `val` (multi-select arrays, checkable
arrays).

**Classes/style:** `addClass/removeClass/toggleClass` (string,
list, or function; `removeClass()` clears all), `hasClass`, `css`
(camelCase accepted, numbers gain `px` unless unitless).

**Events:** `on` (typed, multi-type strings, namespaces,
delegation).

HTTP lives in `src/api` (typed `HttpError`, timeouts, abort signals),
not in this DOM library.

## Deliberate deviations from jQuery

- No fx queue, `show/hide/toggle`, `trigger`, `data`, `clone`,
  `each/map/filter` set operations, or static utilities — use native
  `classList`, `dataset`, `cloneNode()`, `dispatchEvent()`, and array
  methods instead.
- No `wrap/unwrap`, dimensions/offset/scroll, event-map objects,
  `noConflict`, or Sizzle-only selectors (`:visible`, `:has()`,
  positional `:eq()` …). `querySelectorAll` semantics apply; invalid
  selectors throw real errors.
