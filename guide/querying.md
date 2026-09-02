# Querying (`dk`)

Every template runs with `dk` in scope: a host over the merged contract database. Filters return a new view. Projections return an array of row hosts. Stringify of a row is its name (`Event`, `capacity`, `min`).

## Projections

Properties on a view. Each item in the array is itself a view scoped to that name.

| Property | Meaning |
| --- | --- |
| `.models` | Distinct model names in this view |
| `.fields` | Distinct field names |
| `.traits` | Distinct trait names |
| `.types` | Distinct type names |
| `.attributes` | Distinct attribute names |
| `.length` | Count of projected names on the view’s current dimension |
| `.type` | Type name string; only on a **field** row |
| `.args` | Argument array; typical on an **attribute** row |

```
for (model in dk.models) { ... }
for (field in model.fields) { ... }
ty = field.type;
min_attr = field.attribute("min");
bound = min_attr.args[0];
```

`field.type` is a string such as `int_type`. `min_attr.args[0]` is the first argument of `min(1)`.

## Include filters

Call with one string. Singular and plural names are aliases (`dk.trait` / `dk.traits`).

| Call | Dimension |
| --- | --- |
| `.model("Event")` / `.models("Event")` | Model |
| `.trait("Resource")` / `.traits("Resource")` | Trait |
| `.field("title")` / `.fields("title")` | Field |
| `.attribute("min")` / `.attributes("min")` | Attribute |
| `.type("int_type")` / `.types("int_type")` | Type |

Chain them. Each dimension may be included **once**. A second `.trait(...)` on the same view is a runtime error (`duplicate filter dimension`).

A name that does not exist yields an empty view, which is falsy — not an error.

```
resources = dk.trait("Resource");
event = dk.model("Event");
mins = field.attribute("min");
if (mins) {
  => mins.args[0]
}
```

## Exclude filters

| Call | Dimension |
| --- | --- |
| `.not_model("Post")` | Drop that model |
| `.not_trait("Internal")` | Drop that trait |
| `.not_field("id")` | Drop that field |
| `.not_attribute("deprecated")` | Drop that attribute |
| `.not_type("blob_type")` | Drop that type |

Same one-per-dimension rule as includes.

## Cookbook

All models tagged `Resource`, then each field:

```
resources = dk.trait("Resource");
for (model in resources.models) {
  for (field in model.fields) {
    => ```  @{ => field }@: @{ => field.type }@;
```
  }
}
```

Numeric bounds, same facts, different code:

```
min_attr = field.attribute("min");
max_attr = field.attribute("max");
if (min_attr) {
  => ```.min(@{ => min_attr.args[0] }@)```
}
if (max_attr) {
  => ```.max(@{ => max_attr.args[0] }@)```
}
```

Fields that carry an attribute (from [`tests/ngin/sample.ngin`](../tests/ngin/sample.ngin)):

```
filterable_fields = model.attribute("filterable").fields;
```

Everything except one model:

```
rest = dk.not_model("Post");
for (model in rest.models) { ... }
```

`datuma_k catalog` uses the same include dimensions (`--trait`, `--model`, `--field`, `--attribute`, `--type`). See [Generation](generation.md#catalog).
