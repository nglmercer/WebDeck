# WebDeck v2 development

The goal is one v2 application without v1 runtime compatibility. The migration
is **incomplete**. See [STATUS.md](STATUS.md) for the implemented boundaries,
remaining production surfaces, migration map and release gates, and
[MIGRATION_REPORT.md](MIGRATION_REPORT.md) for this change's actual validation.

Remote protected operations require pairing for both HTTP and Socket.IO. Use
loopback on the host to approve devices. Read/input controller boot omits private
administrative and integration settings; editor/settings access requires its own
grant. Plugin changes require an application restart; deck refresh reads the
initialized catalog without evaluating plugin code. Tokens are sent in headers/socket authentication, never URLs. Invalid
supplied credentials do not fall back to local privileges.

Use an isolated `WEBDECK_CONFIG_DIR` for evaluation. Existing configuration must
have supported schema metadata; missing/old versions are rejected without
conversion or data deletion. Complete canonical typed configuration/action
contracts remain unfinished. Do not relabel a v1 document to bypass validation.
V1 belongs on its deprecated branch. Existing backups and user uploads/themes/
plugins are not deleted. The runtime no longer restores/converts v1 backups.

Current command interfaces are `/api/v2/commands` and the `/v2` namespace's
`command` event. Accepted means policy/admission succeeded, completed means the
adapter returned, and failed is a sanitized application/adapter error. A
launched external process may outlive adapter completion. Request IDs correlate
observations; they do not authorize native-effect replay. Disconnect/timeouts
can leave the outcome unknown. Never retry automatically.

The prerelease identity is `2.0.0-alpha.1`. Automatic updates are disabled until
maintainer-controlled v2 distribution is verified. Verified manual installation
and rollback mechanisms remain. Development portable artifacts are versioned
and explicitly labelled `dev-portable`; no artifacts are published by this task.
Linux development packaging requires `strip` to remove debug symbols from staged
copies and keep the archive within updater limits. Original build binaries keep
their symbols.

Use the Rust, Node, component-check and real-browser commands in the uploaded
implementation task. CI additionally runs `node tools/validation/v2-only.mjs`;
it intentionally fails until every identified production compatibility surface
is replaced. Do not weaken or bypass that completion gate to merge this draft.
The [historical proposal](HISTORICAL_PROPOSAL.md) and
[historical plan](IMPLEMENTATION_PLAN.md) explain the prior design but do not
establish current requirements or validation.
