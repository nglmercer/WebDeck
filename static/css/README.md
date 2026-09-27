# WebDeck styles — module system

`style.css` is the default theme **manifest**: theme metadata header plus
`@import`s. The rules live in `base/` and `components/`; cascade order is
the `@import` order in `style.css` (it matches the old monolithic file
exactly, so every override keeps working).

## Adding / changing styles

- Put new rules in the module that owns the feature, or add a module and an
  `@import` line in cascade position (later = wins ties).
- Keep class names stable: user themes (`.config/themes/*.css`) override
  these selectors, so a rename breaks third-party themes.
- Sibling `@import`s fetch in parallel; keep modules small and single-topic.

## Reusable components (theme-author contract)

These classes are shared building blocks, not page-specific styling:

| Class(es) | Module | Used by |
|---|---|---|
| `.button` | `components/buttons.css` | every plain button |
| `.wd_button` + `p.buttontext` | `components/buttons.css` | deck grid, editor preview |
| `.modal-container` / `-content` / `-header` / `-close`, `.addbutton-*`, `.editbutton-*` | `components/modal.css` | all four modals (grouped selectors — one rule skins every variant) |
| `.setting`, `.setting-category` | `components/settings.css` | settings form rows |
| `details.wd-collapse` | `components/collapse.css` | collapsible sections (pairs with `frontend/src/components/collapse.ts`) |
| `input[type=text/password/number]`, `select`, `input[type=submit]` | `components/forms.css` | every form |
| `.switch` + `.slider` | `components/switch.css` | every toggle |
| `.checkbox` | `components/checkbox.css` | round pickers |
| `input[type=range]` | `components/range.css` | every slider |
| `.password-container` + `.show-password` | `components/forms.css` | password fields with visibility toggle |
| `*-color-input-container` pairs | `components/color-inputs.css` | picker + HEX inputs |
| `.editorStyle`, `.fakeform`, `.inputs_container` | `components/editor.css` | edit + add-button modals (pairs with `frontend/src/components/editor.ts`) |
| `.dropdown-btn`, `.dropdown-container` | `components/dropdown.css` | command dropdowns |
| `.invisible`, `.bold` | `base/core.css` | global utilities |

## Server contracts (do not break)

- `style.css` must keep the `theme-*` header lines above the
  `/* ------… */` marker: `parse_css_file` reads the theme name, icon,
  description, and author from them.
- `get_svgs` follows the manifest's `@import`s (local `.css` targets only,
  in order) and collects `url(….svg)` references for icon inlining. New
  icon references work from any module; remote (`http…`) imports are
  skipped. Pinned by `app::server::tests::svgs_follow_stylesheet_imports_in_order`.
- The packager ships the whole `static/` tree, so new modules deploy with
  no config change.
