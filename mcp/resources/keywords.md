# Keyword documentation (`data/keywords.md`)

Every distinct name in the contracts — models, traits, types, attributes, and fields — must have a row. `datuma_k check` and the MCP `validate` tool fail if the file is missing, a row is incomplete, `kind` does not match usage, or the table documents a name the contracts do not use.

`datuma_k start` writes the header row. Fill data rows when you add `.dtct` names.

```md
| keyword | kind | description | purpose | platforms |
| --- | --- | --- | --- | --- |
| Event | model | A scheduled gathering | Shared booking record for API validation and web forms | api_server, web_frontend |
```

| Column | Rule |
| --- | --- |
| `keyword` | The identifier as it appears in `.dtct` |
| `kind` | `model` \| `trait` \| `type` \| `attribute` \| `field` |
| `description` | Non-empty: what it is |
| `purpose` | Non-empty: why it exists in generated code |
| `platforms` | Comma-separated, non-empty subset of `api_server`, `web_frontend`, `mobile_frontend`. For `Data` / `Enum` / `unique` / `relationship` / `email` / `phone_no`, list likely future consumers even if only one platform exists today. That does not mean write ngin for those platforms yet. |

Field names shared across models (for example `capacity`) get one row. `platforms` lists where generated code for that keyword actually ships — not every platform in the company. Do not put `|` inside a cell.
