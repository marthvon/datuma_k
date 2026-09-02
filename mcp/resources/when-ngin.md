# When to use ngin

Use ngin when **two or more platforms** must stay in sync on the same contract-derived artifacts: types, validation, serializers, forms, list/detail views keyed off `dk.models` / `dk.fields` / attributes.

Do **not** use ngin for:

- Routing, auth, middleware, HTTP servers/clients
- Styling, layout chrome, one-off pages
- Business rules that are not field constraints already in `.dtct`
- Files people will heavily rewrite inside generated spans (those edits are overwritten)

If the task is mixed, split it: ngin emits the contract-shaped bits; glue stays handwritten **between** generated spans.

The MCP `advise_ngin` tool applies these rules. After adding a field, update every `.ngin` that should see it, update `keywords.md`, then `validate` and `preview`.
