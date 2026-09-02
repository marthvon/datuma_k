# Contracts (`*.dtct`)

A `.dtct` file is a tagged record shape. datuma_k does **not** typecheck it. Names like `int_type` and `min` are free identifiers; templates and [`.def.ngin` helpers](definitions.md) decide what they mean.

Put `*.dtct` under `DTCT_DIRECTORY` (default `data/`). Several files are merged into one database. Model names must be unique across that tree.

## Grammar

```
ModelName [Trait1, Trait2] {
  field_name: type_name<attr1, attr2(1, "str", true)>,
  other: other_type<>
}
```

| Piece | Syntax | Notes |
| --- | --- | --- |
| Model | `Ident` then optional traits then `{…}` | Several models per file |
| Traits | `[A, B]` after the model name | Optional; empty `[]` is allowed |
| Field | `name: type_expr` | Comma-separated; **no trailing comma** before `}` |
| Type | `type_name<…>` | **`<`…`>` is required**, even when empty (`uuid_type<>`) |
| Attribute | `name` or `name(args)` | Comma-separated inside `<>` |
| Args | ident, `"string"`, number, `true` / `false` | |

Identifiers are `[A-Za-z_][A-Za-z0-9_]*`. There is no comment syntax and no import.

## Example

From [`tests/example/data/app.dtct`](../tests/example/data/app.dtct):

```
Event [Resource] {
  title: text_type<required>,
  capacity: int_type<min(1), max(500)>,
  starts_at: datetime_type<local>
}

Venue [Resource] {
  name: text_type<required>,
  capacity: int_type<min(1), max(10000)>
}
```

`[Resource]` is a tag for filtering (`dk.trait("Resource")`), not inheritance and not shared fields. `min` / `max` / `required` / `local` become queryable facts. Python maps them to `Field(ge=, le=)`; TypeScript maps them to Zod `.min().max()`. That mapping is in the templates, not in DTCT.

## Traits

Traits are optional lists after the model name. Multi-line commas are fine:

```
UserAccount [
  OauthSession,
  CookieSession,
] {
  email: email_type<max_length(255)>
}
```

Empty models are valid:

```
Marker {}

Tagged [TagA, TagB] {}
```

## Fields, types, attributes

Each field has one type. The type always takes an attribute list:

```
id: uuid_type<>
capacity: int_type<min(1), max(500)>
email: email_type<format("user@example.com"), max_length(255)>
enabled: bool_type<default(true)>
```

A missing `<>` is a parse error. A trailing comma before `}` is a parse error.

DTCT does not know that `int_type` is an integer. `py_type("int_type")` returning `"int"` is a helper you write. A typo such as `int_typo` still parses.

## What DTCT is not

- **Not a typechecker.** There is no built-in vocabulary of types or attributes.
- **No relations.** A field named `ref_field` is just a field. Nothing links models.
- **No comments, imports, or modules.** Split files if you want; merge still requires unique model names.
- **No constraints engine.** `min(1)` is data for templates. Enforcement lives in generated Pydantic / Zod / forms.

## How templates see this

The parser expands each model into queryable fact rows (trait × field × attribute). You do not write facts by hand. You query them through [`dk`](querying.md):

```
resources = dk.trait("Resource");
for (model in resources.models) {
  for (field in model.fields) {
    min_attr = field.attribute("min");
  }
}
```

Stringify of a row is its name (`Event`, `capacity`, `min`). An empty filter is falsy, so `if (min_attr)` is the usual “does this field have min?” test.

## Keyword table

`datuma_k start` writes `data/keywords.md` with an empty header. [`datuma_k check`](generation.md#check) requires a row for every distinct model, trait, type, attribute, and field name used in the contracts. `run` does not.

```md
| keyword | kind | description | purpose | platforms |
| --- | --- | --- | --- | --- |
| Event | model | A scheduled gathering | Shared booking record | api_server, web_frontend |
```

`kind` is `model`, `trait`, `type`, `attribute`, or `field`. `platforms` is a comma-separated subset of `api_server`, `web_frontend`, `mobile_frontend`. Field names shared across models (`capacity`) get one row. Do not put `|` inside a cell. Full rules: [`mcp/resources/keywords.md`](../mcp/resources/keywords.md).
