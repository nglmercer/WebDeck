# Keyboard and OBS hardware acceptance

Automated fake-backend tests do not establish native desktop, hardware-key,
compositor or real OBS compatibility. All rows below remain **not run** in this
implementation session; no isolated physical desktop/OBS test instance was
provided. The simulated WebSocket server uses an isolated loopback port.

| Environment | Required manual checks | Status |
|---|---|---|
| Windows | Ctrl+C; F1–F24; lock/system keys; distinct numpad; failed-press cleanup; concurrent chords | Not run |
| Linux X11 | Same; active keyboard layout; Unicode vs write action; release failure | Not run |
| Linux Wayland, approved portal | Initial approval, session reuse, F1–F24, modifiers, expiry, cleanup | Not run |
| Linux Wayland, denied portal | Explicit denial, no keys sent, renewed approval flow, bounded timeout | Not run |
| Laptop firmware Fn | Picker disabled; map to underlying media action; no universal Fn injection claim | Not run |
| Real OBS WebSocket 5/RPC 1 | Auth, restart, version negotiation, advertised source kinds, names/UUIDs | Not run |
| Isolated OBS test collection | First apply, second noop, nested scene contents, template overrides, pruning safeguards | Not run |
| Real OBS + concurrent deck edit | Partial report, revision conflict, safe retry, no secret output | Not run |

Before real mutations, export a backup through OBS and explicitly select a
throwaway collection. Never point the project acceptance test at an unknown
active production collection. Test multiple layouts, lock states and laptop
key remapping: browser `code`, browser `key`, and injected OS keys differ.
Native injection success does not prove that the foreground application handled
the key. Physical Fn is normally consumed by keyboard firmware.
