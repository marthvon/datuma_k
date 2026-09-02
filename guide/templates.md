# Templates (`*.ngin`)

A `.ngin` file opens output paths and emits text while walking the contract. Put templates under `NGIN_DIRECTORY` (default `engine/`). Shared functions live in [`*.def.ngin`](definitions.md), not in the template files.

This is not Jinja or Mustache. Literal text sits in fenced blocks. Programming happens in `@{ … }@` using the [scripting language](language.md). Contract access is the [`dk` host](querying.md).

## Glyphs

| Glyph | Role |
| --- | --- |
| `\|path>` then a fence | Open an output file |
| `@{ … }@` | Interp: statements, loops, emits |
| `=> expr` | Write the stringified expression into the current file |
| `=> ```…```` | Emit a nested template |
| `+= ```…```` | Keep the region only if something emitted inside |
| `?(cond)?"sep"=> payload` | Guarded emit (separator only when a prior emit already fired) |
| `$NAME` / `${NAME}` | Env var in a path |

## Open a file

````ngin
|$ROOT_DIRECTORY/backend/generated/models.py>
```
from pydantic import BaseModel, Field

@{ => "generated body" }@
```
````

`|path>` must come before any `=>` at the root of the file. Nested `|path>` inside an already-open fence is an error.

Paths resolve against `.env`. `$ROOT_DIRECTORY` and `${ROOT_DIRECTORY}` are the same. Quoted segments allow spaces:

```
|$ROOT_DIRECTORY/"My Models"/out.ts>
```

Interp works in a path, which is how one template emits one file per model:

````ngin
@{
for (model in dk.models) {
  |$ROOT_DIRECTORY/generated/@{ => snake_case(model) }@.ts>
  ```
  export type @{ => pascal_case(model) }@ = {};
  ```
}
}@
````

## Emit

`=> expr` stringifies the expression (a model row becomes `"Event"`). `=> ```…```` walks another template and writes that text.

````ngin
@{
for (field in model.fields) {
  => ```  @{ => field }@: @{ => ts_type(field.type) }@;
```
}
}@
````

## Plus and guards

`+= ```…```` is dropped if nothing inside actually emitted. Guards live **inside** a `+=` template:

````ngin
@{ += ```
    import {
        @{
        ?(filterable.length > 1)?","=> ```"myfilterfunction"```
        ?(emailable.length > 1)?","=> ```"myemailfunction"```
        }@
    } from "@/vendor/custompackage";
``` }@
````

If both conditions fire, the separator is inserted between them: `first,second`. If only the second fires, you get `second` with no leading comma.

## Worked pattern: one contract, two languages

The example contract tags models `[Resource]` and puts `min` / `max` on numeric fields. Both templates filter the same way, then map attributes differently.

Python ([`tests/example/engine/python.ngin`](../tests/example/engine/python.ngin)):

````ngin
|$ROOT_DIRECTORY/backend/generated/models.py>
```
from datetime import datetime

from pydantic import BaseModel, Field


@{
resources = dk.trait("Resource");
for (model in resources.models) {
  => ```
class @{ => pascal_case(model) }@(BaseModel):
@{
    for (field in model.fields) {
      min_attr = field.attribute("min");
      max_attr = field.attribute("max");
      suffix = "";
      if (min_attr) {
        if (max_attr) {
          suffix = " = Field(ge=" + min_attr.args[0] + ", le=" + max_attr.args[0] + ")";
        } else {
          suffix = " = Field(ge=" + min_attr.args[0] + ")";
        }
      } else if (max_attr) {
        suffix = " = Field(le=" + max_attr.args[0] + ")";
      }
      => ```    @{ => field }@: @{ => py_type(field.type) }@@{ => suffix }@
```
    }
}@
```
}
}@
```
````

TypeScript types ([`tests/example/engine/typescript.ngin`](../tests/example/engine/typescript.ngin)):

````ngin
|$ROOT_DIRECTORY/frontend/src/generated/types.ts>
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
````

The Zod schema in the same file reads `min_attr.args[0]` and emits `.min(1).max(500)` instead of `Field(ge=1, le=500)`. `py_type` / `ts_type` / `zod_base` are helpers in the example’s [`definition/cases.def.ngin`](../tests/example/definition/cases.def.ngin), not built into DTCT.

## What is not implemented

- **No `#include` / `#define`.** The editor grammar may highlight them; the engine does not. “Includes” means: every `*.def.ngin` under `DEF_DIRECTORY` is loaded into scope before templates run.
- **`*.def.ngin` under `engine/` are skipped** as templates. Plain `*.ngin` under `definition/` are ignored as helpers.
- **`return` inside a nested `@{ }@`** is swallowed so the outer emit can continue. Use `return` at the template/function level you actually want to leave.

## After emit

`datuma_k run` commits planned files through [dkcache](generation.md#dkcache). Put generated output in a `generated/` folder (or similar). Handwrite routing, HTTP clients, and business rules **between** spans or in files the templates never open.
