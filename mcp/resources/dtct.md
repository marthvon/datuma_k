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
- Traits are optional. `Event [Resource]` marks Event as a Resource; ngin typically does `dk.trait("Resource").models`.
- Every type is followed by `<...>`. No attributes still needs empty brackets: `text_type<>`.
- Attributes may take arguments: `min(1)`, `max(500)`, `flag("on")`.
- Shared attributes (`min`, `max`, `required`, `local`) must mean the same thing in every template.

## Example

```
Event [Resource] {
  title: text_type<required>,
  capacity: int_type<min(1), max(500)>,
  starts_at: datetime_type<local>
}
```

## Files

Put `*.dtct` under `DTCT_DIRECTORY` (default `data/`). Document every model, trait, type, attribute, and field in `data/keywords.md` before `datuma_k check` / the `validate` tool will pass.

Do not generate `.dtct` from ngin. Contracts are authored by hand (or by the agent editing files). Types without `<` are a parse error.
