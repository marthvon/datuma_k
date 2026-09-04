# Writing `.dtct` contracts

A `.dtct` file declares models once. ngin reads this shape; each platform generates different code from the same fields and attributes.

## Shape

```
ModelName [Trait, OtherTrait] {
  field_name: type_name<attr, attr2(arg)>,
  other: type_name<>
}
```

- Model names and traits are identifiers.
- Traits are optional. Prefer **`Data`** for records with fields and **`Enum`** for empty / discriminant models (`datuma://language/standards`). `Event [Resource]` is still valid; ngin typically does `dk.trait("Resource").models`.
- Every type is followed by `<...>`. No attributes still needs empty brackets: `string<>`.
- Prefer standard types: `i8` `i16` `i32` `i64` `float` `double` `string` `boolean` `datetime` `relationship`. `unsigned` is an attribute (`i32<unsigned>`), not a type. Older contracts may still use `text_type` / `int_type`.
- Type is not the widget. A `string` may be `Text`, `Select`, `Radio`, or `Upload`. Do not assume a textbox. Widgets are flat flags: `Select`, `Checkbox`, `Range`, `Datetime`.
- `relationship` uses flat flags: `venue: relationship<model(Venue), BelongsTo, Full, Select>`. Cardinality (`OneToOne`, `OneToMany`, `ManyToMany`, `BelongsTo`) and dependency (`Partial`, `Full`, `Transitive`) are conventions, not parser rules.
- Attributes may take arguments: `min(1)`, `max(500)`, `default("x")`, `min_length(1)`, `regex("...")`. Shared attributes must mean the same thing in every template.

## Example

```
Event [Data] {
  title: string<required, min_length(1), Text>,
  status: string<required, Select>,
  capacity: i32<min(1), max(500), Range>,
  starts_at: datetime<local, Datetime>,
  venue: relationship<model(Venue), BelongsTo, Full, Select>,
  contact: string<email>
}
```

The compiler still parses stacked flags (`OneToOne` next to `ManyToMany`). `infer_patterns` may suggest reviewing that pairing; it is not a `validate` failure.

## Files

Put `*.dtct` under `DTCT_DIRECTORY` (default `data/`). Document every model, trait, type, attribute, and field in `data/keywords.md` before `datuma_k check` / the `validate` tool will pass.

Do not generate `.dtct` from ngin. Contracts are authored by hand (or by the agent editing files). Types without `<` are a parse error.
