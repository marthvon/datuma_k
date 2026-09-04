# Standard dtct vocabulary

MCP **defaults** for new contracts — not language rules. The compiler accepts any identifier; projects may use other names. `infer_patterns` only comments when these default names appear.

Do not dump these names into `keywords.md` until a `.dtct` file actually uses them (`datuma_k check` rejects unused rows).

## Types

| Type | Meaning | ngin mapping (extend `py_type` / `ts_type` / `zod_base`) |
| --- | --- | --- |
| `i8` `i16` `i32` `i64` | Signed integer widths | Python `int`, TypeScript `number`, Zod `z.number().int()` |
| `float` `double` | IEEE floats | Python `float`, TypeScript `number`, Zod `z.number()` |
| `string` | UTF-8 text | `str` / `string` / `z.string()` |
| `boolean` | True/false | `bool` / `boolean` / `z.boolean()` |
| `datetime` | Instant | `datetime` / `Date` |
| `relationship` | Pointer at another model | `relationship<model(Venue), BelongsTo, Full, Select>` |

`unsigned` is an **attribute** on integer types (`i32<unsigned>`), not a type. Widget `Datetime` is not the `datetime` type.

Type is not the widget. A `string` may be `Text`, `Select`, `Radio`, or `Upload`. Do not assume a textbox.

Existing projects may still use `text_type`, `int_type`, `bool_type`, `datetime_type`. Prefer the table above on new fields. If you adopt `i32` / `string`, update `definition/*.def.ngin` helpers.

## Traits

| Trait | Use on |
| --- | --- |
| `Data` | Encapsulated structs (records with fields) |
| `Enum` | Discriminated names / empty-body models |

## Field attributes (flat flags)

**Relationship:** `model(...)` for the target. Cardinality: `OneToOne`, `OneToMany`, `ManyToMany`, `BelongsTo`. Dependency: `Partial`, `Full`, `Transitive` (not `Transitive-Dependency` — hyphens are not identifiers).

**Validation:** `required`, `min`, `max`, `min_length`, `max_length`, `regex(...)`, `email`, `phone_no`. Numeric bounds use `min`/`max`; string length uses `min_length`/`max_length`.

**UI widgets:** `Text`, `Select`, `Checkbox`, `Range`, `DateRange`, `Date`, `Datetime`, `DatetimeRange`, `Radio`, `AsyncSelect`, `Upload`.

**Also:** `nullable`, `unique`, `default(...)`, `unsigned`, `local`.

The compiler will parse stacked flags (`OneToOne` next to `ManyToMany`). `infer_patterns` may suggest reviewing that pairing. Treat those hits as review notes, not a failed `validate`.

```
Event [Data] {
  title: string<required, min_length(1), max_length(120), Text>,
  status: string<required, Select>,
  capacity: i32<min(1), max(500), Range>,
  starts_at: datetime<local, Datetime>,
  venue: relationship<model(Venue), BelongsTo, Full, Select>
}
```

UI attrs may list only frontend platforms on the keyword row. Cardinality / `model` / dependency are data-shaped: tag them even if only `api_server` exists; do not add ngin until a second consumer exists.

## Think ahead

Tag `Data` / `Enum` / `unique` / `relationship` / `email` / `phone_no` / cardinality on the **contract** even if only `api_server` exists today. List likely future platforms on the `keywords.md` row. Do **not** add an ngin target until a second consumer actually needs generated code (`advise_ngin`).
