# Settings design system

Settings uses flat neutral surfaces, thin borders and a single neutral selection color. Its tokens are scoped to `.settings-shell`; the deck keeps its own colors, images and full-screen layout. There are no decorative shadows or gradients.

Desktop uses a sidebar and a header aligned to the content. At 800px and below, navigation becomes a horizontally scrollable tab list and cards stack into one column. The save action remains in the settings header. Arrow keys, Home and End work in navigation; native radios provide keyboard selection in choice groups.

## Components

- `SettingsCard`: a titled surface with an optional icon and description, rendered through a typed Svelte snippet.
- `ChoiceField<T extends string | number>`: a named radio group with typed values, visible options and optional descriptions. Numeric durations stay numbers; transport values stay the `http | socket` union.
- `NumberField`: existing bounded numeric input, with optional `controls` and `unit` props. Settings enables its minus/plus buttons; other screens retain the existing input. Invalid values do not enter the draft, and settings blocks saving while an input is invalid.
- `SwitchField`: a boolean choice using a labelled native checkbox, with a full-size clickable label.
- `FileField`: a keyboard-accessible native file input inside a flat file card. The input's file-type filter and original upload/inspection callbacks remain intact.
- `StatusBadge`: a text label with `neutral | success | error` tones. State never depends on color alone.

Cards use 20px padding on desktop and 16px on mobile, 10px corners, and 20px gaps. Interactive controls have 44px minimum height. Radio, checkbox and file inputs use their full labels as targets. Focus indicators use a solid outline rather than glow.

## Sections and behavior

Appearance separates layout, button styling, a live sample preview, language cards, and uploaded files. The preview reflects the first eight configured buttons, not the complete spatial deck layout.

OBS and Spotify each have a separate card. Device status and revocation are visible in device cards. Managing a device or creating a manual pairing token opens a labelled dialog. Network controls use a compact side card. Pairing and address QR codes open on demand in nonmodal dialogs so host approvals remain reachable; troubleshooting opens a separate dialog. No disclosure sections are used. Trust duration and transport use choice groups.

Backups separate inspecting/restoring a file from exporting the draft. Inspection never applies a backup automatically. Plugins show status, version, runtime type, and enable/disable actions as rows.

The existing draft, revision, save-conflict, authorization and API contracts remain in use. Network controls still apply immediately; deck appearance and integration settings follow the existing explicit-save flow.
