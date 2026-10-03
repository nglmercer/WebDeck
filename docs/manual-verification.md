# Manual verification still required

This checklist completes the live checks in [the improvement plan](improvement-plan.md). Automated evidence is recorded in [the requirement register](improvement-requirements.md). A compile result, fake-effect server, screenshot, or tool-presence inventory is insufficient for the checks below.

Record date, build/archive digest, OS/session, device and application versions, exact fixture, expected/observed result, cleanup outcome and evidence location. Record unavailable separately from failed or untested. Use a temporary configuration and disposable target application; retain existing desktop/account settings.

## Assistive technology

Use a supported browser and an actual screen reader, recording both versions.

- Navigate the deck, controls/help dialogs, button editor, settings and pairing using the keyboard. Confirm names, focus containment, Escape and focus restoration are announced consistently.
- Change a numeric field to invalid input, then correct it. Confirm the error is associated with the field and clears without repetitive announcements.
- Save a draft, cause a deliberate revision conflict with a second temporary client, and verify clean/dirty/saving/saved/conflict feedback is understandable. Confirm the draft remains exportable.
- Inspect disabled Connect, Approve, Revoke, list Add/Remove and row navigation controls. Confirm each reason is discoverable; revoked and pending device states must be distinguishable.
- Leave metric polling active while navigating. Confirm regular metric updates do not repeatedly interrupt speech. Run one harmless fixture command and confirm running/failure feedback is announced once per meaningful transition.

## Windows, native Linux X11 and Linux Wayland

Run the following on each supported session, identifying native X11 separately from XWayland.

| Area | Fixture and expected result |
| --- | --- |
| Tray and QR | Start the packaged app with tray enabled. Tray actions reach the temporary deck. Scan its QR from a second device; URL and pairing instructions are usable. Quit removes the owned tray/server. |
| Permissions | Approve a temporary controller with Read only. Input/script/audio commands are denied. Revoke it and confirm new requests fail. On Wayland, record portal permission grant/deny/cancel outcomes. |
| Keyboard | Focus a disposable text editor. Execute a known chord and text command once. Verify exact text/chord, modifier release, unrelated command responsiveness and no replay after observer disconnect. |
| Clipboard | Save clipboard content locally, use a known fixture value, verify native read/write/paste, and restore the previous content. Do not publish clipboard contents in evidence. |
| Media/audio | Use a disposable media session and generated quiet test clip. Verify play/pause, volume/mute, endpoint choice and owner shutdown. Restore volume/routing; confirm no playback/resource remains. |
| Capture | Use a fixture window with recognizable content. Verify dimensions/colors and clipboard capture; record portal deny/cancel cleanup on Wayland. Delete only fixture captures and restore clipboard. |
| Child cleanup | Launch an owned disposable child/descendant. Quit WebDeck and confirm termination/reaping. Separately verify intentionally handed-off application launches survive as documented. |
| Shutdown | Start accepted fixture work, disconnect its observer, then quit. Verify accepted ownership/drain, process/audio cleanup, port release and no automatic replay. |
| Portable package | Extract outside the repository. Verify UI/assets and actual upgrade/rollback preserving temporary configuration/uploads. Windows requires its native archive; Linux development results do not establish Windows packaging. |

Avoid power, firewall and user-application termination commands as verification fixtures.

## OBS and Spotify

- OBS: use an isolated scene collection/profile and suitable local service. Verify successful bounded identification, a harmless fixture scene action, wrong-credential failure, local-admin access and secret redaction. Restore prior profile/scene; do not interrupt an active recording or stream.
- Spotify: use an explicitly available test account/application. Complete authorization, verify status and a harmless controlled playback action, then edit credentials and verify old continuation links cannot authorize the new draft. Record token cleanup without storing credentials in evidence. Restore the prior playback state.

No live platform/account success is currently claimed. Update R07/R53/R59/R69 only when the recorded observations cover their full scope.
