# Typed commands

Commands are JSON values described by `contracts/v2.schema.json` and its generated `contracts/catalog.json`. [The action inventory](v2/ACTION_INVENTORY.md) groups them by capability.

```json
{"request_id":"example-1","command":{"type":"key","keys":["ctrl","c"]}}
```

Send this request to `POST /api/v2/commands`, or use the `/v2` Socket.IO namespace. The console accepts the command object alone, one per line.

Configured controller buttons invoke stable button references; the server resolves their private actions and checks the actual capabilities. Admission allows at most 16 accepted roots, while native effects execute serially. Accepted work survives observer disconnect. Script/plugin nested calls inherit the root budget and check capability, depth and deadline. Do not automatically retry when the outcome is unknown.

Folder navigation, reload, fullscreen, settings and usage are typed frontend actions. There is no text-prefix dispatcher.
