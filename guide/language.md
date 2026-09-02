# Language

Templates and definition files share one embedded language. It runs inside `@{ … }@` and as the whole body of `*.def.ngin`. There is no `datuma_k eval` and no standalone script CLI.

It is a small statement language: assignment, `fn`, `if` / `for`, operators, and a few members on strings, arrays, and dicts. There is **no comment syntax**.

## Types and literals

| Type | Syntax |
| --- | --- |
| null | `null` |
| bool | `true`, `false` |
| integer | `1`, `-8` |
| float (f32) | `1.5`, `.25` |
| double (f64) | `.5d` |
| string | `"a\"b\\c"` |
| array | `[1, 2]`, `[[1], [2]]` |
| dict | `{a: 1, "tag": "ok"}` |
| host | opaque (`dk` and row views) |

Truthy: non-null, non-zero number, `true`, non-empty string / array / dict, and a `dk` view that still has rows. Empty filter results are falsy (`if (min_attr)`).

Numbers of different kinds compare equal after promotion (`1 == 1.0`). Host rows compare equal to their name string (`model == "Event"`).

## Variables and functions

```
x = 1;
fn add(a, b) {
  return a + b;
}
```

Assignment declares the name in the current scope. Inside a function, assignment **shadows** an outer name; it does not write through.

Functions are hoisted globally. Later `fn` of the same name wins. Nested `fn` is also global. Fixed arity; no default arguments; functions are not values you can store.

Limits: 64 call frames, 1_000_000 loop iterations.

## Control flow

```
if (cond) {
  ...
} else if (other) {
  ...
} else {
  ...
};

for (item in xs) {
  ...
}

for (i = 0; i < n; i = i + 1) {
  ...
}

return expr;
break;
```

`for (item in xs)` walks array elements, dict **keys**, or string characters.

There is no `elseif` keyword — write `else if`. An `if` used as a statement often ends with `;`.

`if` as an expression uses `yield` in **both** branches:

```
fn fact(n) {
  return if (n) { yield n * fact(n - 1); } else { yield 1; };
}
```

Both branches must be a sole `yield`. Otherwise it is a parse error.

In ngin, `return` / `break` stop the materialize walker. A `return` inside a nested `@{ }@` is swallowed so the outer emit continues.

## Operators

Lowest to highest binding:

| Level | Operators |
| --- | --- |
| lowest | `\|\|` |
| | `&&` |
| | `\|` (bitwise) |
| | `^` xor / collection symmetric-diff; `&^` left-diff; `^&` right-diff |
| | `&` bitwise / intersect |
| | `==` `!=` |
| | `<` `>` `<=` `>=` |
| | `+` `-` |
| | `*` `/` `%` |
| highest | `**` (right-associative) |

Unary: `!` (bool / null), prefix and postfix `++` `--` on numbers.

Assign: `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `|=` `^=` `&&=` `||=`, plus the collection-diff assigns.

After an array or dict, `+` `-` `^` `&` are collection ops, not arithmetic. Booleans and null need `&&` / `||` (not bare `&` / `|`). Strings allow `+`, `*`, `==`, `!=`.

| Expression | Result |
| --- | --- |
| `"a" + "b"` | `"ab"` |
| `"xy" * 3` | `"xyxyxy"` |
| `"abc"[1]` | `"b"` |
| `[1, 2] + [2, 3]` | concat |
| `[1, 2] - [2, 3]` | left only |
| `[1, 2] ^ [2, 3]` | symmetric diff |
| `[1, 2] & [2, 3]` | intersection |
| `a &^ b` | left diff |
| `a ^& b` | right diff |
| `{a: 1} + {b: 2}` | merge, rhs wins |
| `base - ["a"]` | dict minus key list |

`+` on a string stringifies the right side, which is how templates build `Field(ge=` + `min_attr.args[0]` + `)`.

## Members

**string:** `.length` (Unicode scalar count), `.upper()`, `.lower()`, index `[i]`

**array:** `.length`, `.insert(v)` append, `.insert(i, v)` at index, `.remove()` pop last, `.remove(i)`

**dict:** `.length`, key as property (`d.a`) or `d["a"]`, `.insert(k, v)`, `.remove(k)`, `.asArray()` → `[[k, v], …]`

There are no free-standing builtins (`print`, `len`, …). Case conversion beyond `.upper()` / `.lower()` lives in [`.def.ngin`](definitions.md). Contract queries live on [`dk`](querying.md).
