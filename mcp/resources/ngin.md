# Writing `.ngin` templates

ngin walks the contract (`dk`) and emits files. Put `*.ngin` under `NGIN_DIRECTORY` (default `engine/`). Shared helpers live in `*.def.ngin` under `DEF_DIRECTORY` (default `definition/`). `datuma_k start` writes `definition/cases.def.ngin` (`pascal_case`, `snake_case`, `py_type`, `ts_type`, `zod_base`, …).

## File emit

```
|$ROOT_DIRECTORY/backend/generated/models.py>
```
...generated text...
```
```

`|path>` opens an output file. Fenced ` ``` ` blocks are literal text mixed with interpolations.

## Interpolation

`@{ ... }@` is ngin code. `=> ```...```` yields text into the current file. `+=` appends.

```
@{
resources = dk.trait("Resource");
for (model in resources.models) {
  => ```
export type @{ => pascal_case(model) }@ = {
@{
    for (field in model.fields) {
      => ```  @{ => field }@: @{ => ts_type(field.type) }@;
```
    }
}@
};
```
}
}@
```

## `dk` query

| Call / property | Meaning |
| --- | --- |
| `dk.trait("Resource")` / `dk.traits("Resource")` | Narrow to that trait |
| `.models` `.fields` `.traits` `.types` `.attributes` | Project rows |
| `dk.model("Event")` `dk.field("title")` `dk.attribute("min")` `dk.type("int_type")` | Include filter |
| `dk.not_model` `dk.not_trait` `dk.not_field` `dk.not_attribute` `dk.not_type` | Exclude filter |
| `field.type` | Type name on a field row |
| `field.attribute("min")` | Attribute view; missing is falsy |
| `min_attr.args[0]` | First argument |
| `.length` | Count of projected names |

One include/exclude per dimension. Duplicate `dk.trait` then `dk.trait` again is an error.

## Control flow

`if` / `else if` / `else`, `for (x in xs)`, `fn name(args) { ... }` in `.def.ngin` (later definition of the same name wins). `start` already provides case helpers — call them; do not copy the functions into every template.

## dkcache

Generated spans are overwritten on the next `run`. Text **between** spans is kept. Do not put handwritten business logic inside a generated snippet.
