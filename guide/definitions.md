# Definitions (`*.def.ngin`)

A `.def.ngin` file is a plain program: functions only, no `|path>` and no fences. All of them under `DEF_DIRECTORY` (default `definition/`) run before any template, so their `fn`s are in scope everywhere.

`datuma_k start` copies a starter set into `definition/cases.def.ngin`. Add more `*.def.ngin` files as needed; nested directories are loaded too.

## What loads

| Location | Loaded as |
| --- | --- |
| `definition/*.def.ngin` | Helpers (`ProgramParseMode`) |
| `engine/*.ngin` | Templates |
| `definition/*.ngin` (not `.def.ngin`) | Ignored |
| `engine/*.def.ngin` | Skipped as templates |

Later `fn` of the same name wins. Nested `fn` definitions are also global. There is no module system and no `export`.

## Starter helpers

`start` seeds case conversion, not platform type maps. From [`tests/ngin/defs/helpers.def.ngin`](../tests/ngin/defs/helpers.def.ngin):

| Function | Role |
| --- | --- |
| `ident(x)` | Return `x` unchanged |
| `snake_case(s)` | `EventDetail` → `event_detail` |
| `pascal_case(s)` | `event_detail` → `EventDetail` |
| `camel_case(s)` | `event_detail` → `eventDetail` |
| `title_case(s, separator)` | Split on `separator`, title-case words |
| `upper_case(s)` / `lower_case(s)` | Whole-string case |
| `replace_all(s, from, to)` | Replace every `from` with `to` |

The rest (`is_letter`, `is_upper`, `is_digit`, `is_word_char`, `is_break`, `slice_eq`, `collapse_initials`) are building blocks those functions use. Call the case helpers from templates; do not copy them into every `.ngin` file.

## Where DTCT names get meaning

Types such as `int_type` are just strings until a helper maps them. The example project adds that mapping in [`tests/example/definition/cases.def.ngin`](../tests/example/definition/cases.def.ngin):

```
fn py_type(name) {
  t = "" + name;
  if (t == "int_type") {
    return "int";
  } else if (t == "datetime_type") {
    return "datetime";
  } else if (t == "bool_type") {
    return "bool";
  } else {
    return "str";
  }
}

fn is_local(field) {
  return field.attribute("local");
}

fn route_of(model) {
  return "/" + snake_case(model) + "s";
}
```

`ts_type`, `zod_base`, and `input_type` in that file are the same idea for TypeScript / Zod / HTML. If Python uses `ge=` for `min`, TypeScript must use `.min()` for `min` — keep that convention in the helpers and templates, not as a DTCT built-in.

## Writing a helper

The [language](language.md) is the same as inside `@{ }@`. A definition file is only `fn`s and top-level statements that should run once at load (usually you only define functions).

```
fn route_of(model) {
  return "/" + snake_case(model) + "s";
}
```

Fixed arity, no default arguments, no closures as values. Recursion is capped at 64 frames.
