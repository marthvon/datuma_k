# datuma_k good practices

- One contract for a record. Do not copy fields into Pydantic, Zod, and forms by hand.
- Shared attributes (`min`, `max`, `required`, `local`) mean the same thing in every `.ngin` file. If Python uses `ge=` for `min`, TypeScript must use `.min()` for `min`, not a different rule.
- Generate types, validators, and form fields from `dk`. Handwrite routing, auth, HTTP clients, styling, and business rules.
- Put generated files in a `generated/` folder (or similar). Keep a `.dkcache` next to them; do not edit inside generated spans.
- Document every dtct keyword in `data/keywords.md` (description, purpose, platforms) in the same change as the contract edit.
- Prefer existing `definition/*.def.ngin` helpers (`pascal_case`, `py_type`, `ts_type`) over new one-off case functions.
- Run `validate` (or `datuma_k check`) after editing `.dtct` / `.ngin`. Use `preview` before `generate` / `run`.
- Use ngin when two or more platforms must stay in sync on contract-derived code. Skip ngin for a one-off file that will not be regenerated when fields change.
- Do not wrap datuma_k as a second source of truth. Files under `data/` and `engine/` are the source; the binary only parses, plans, and commits.
