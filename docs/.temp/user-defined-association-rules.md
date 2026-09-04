# User-defined association rules (future)

Not implemented. dtct identifiers stay open: any attribute, field, model, or trait name parses. MCP's default vocabulary (`OneToMany` vs `ManyToMany`, `Text` vs `Select`, widget vs type) is only a suggestion list for agents.

Later, a project should be able to declare **its own** deny/allow pairings instead of a global keyword table baked into the parser or MCP:

- attribute ↔ attribute (e.g. at most one of a user-listed cardinality set)
- attribute ↔ type (`unsigned` only with listed integer types)
- attribute ↔ trait
- field ↔ model
- trait ↔ trait

Rules would be **data** next to the contract (for example beside `keywords.md`), consumed by MCP and maybe `check`. Grammar stays unrestricted. Until then, `infer_patterns.association_suggestions` is prevention/mitigation for agents writing or reviewing `.dtct` / `.ngin`.
