# datuma_k good practices

- One contract for a record. Do not copy fields into Pydantic, Zod, and forms by hand.
- One trait and one attribute name across platforms. Tag `Data` / `Enum` / `unique` / `email` on the contract even if only `api_server` exists today. List likely future platforms on the `keywords.md` row; do not invent `ApiUser` vs `WebUser`.
- Shared attributes (`min`, `max`, `required`, `local`, `nullable`, `unique`) mean the same thing in every `.ngin` file. If Python uses `ge=` for `min`, TypeScript must use `.min()` for `min`, not a different rule.
- Generate types, validators, and form fields from `dk`. Handwrite routing, auth, HTTP clients, styling, and business rules.
- Put generated files in a `generated/` folder (or similar). Keep a `.dkcache` next to them; do not edit inside generated spans.
- Document every dtct keyword in `data/keywords.md` (description, purpose, platforms) in the same change as the contract edit.
- Prefer existing `definition/*.def.ngin` helpers (`pascal_case`, `py_type`, `ts_type`) over new one-off case functions.
- Run `validate` (or `datuma_k check`) after editing `.dtct` / `.ngin`. Use `preview` before `generate` / `run`.
- Use ngin when two or more platforms must stay in sync on contract-derived code. Skip ngin for a one-off file that will not be regenerated when fields change. Tagging a contract is not the same as generating: future platforms belong on the keyword row first.
- Prefer standard types from `datuma://language/standards` on new fields. Type is not the widget (`string` may be `Select`, not `Text`). If you adopt `i32` / `string`, extend `py_type` / `ts_type`; do not rewrite existing `text_type` contracts unless asked.
- Run `infer_patterns` when writing or reviewing `.dtct` / `.ngin`. `association_suggestions` are prevention/mitigation for agents, not `validate` failures. The compiler does not forbid stacked flags.
- Do not wrap datuma_k as a second source of truth. Files under `data/` and `engine/` are the source; the binary only parses, plans, and commits.
