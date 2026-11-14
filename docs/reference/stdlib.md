# Standard modules and number types

```clpp
link @clpp.math as Math
link @clpp.text as Text
link @clpp.table as Table
link @clpp.fs as Fs
```

## Example

```clpp
link @clpp.math as Math
link @clpp.text as Text
link @clpp.table as Table

post(Math.floor(Math.pi * 100))
post(Math.huge > Math.maxInt)
post(Text.format("{} has {} hp", "Ada", 100))
post(Text.join(Text.split("a,b,c", ","), " | "))

dictionary<string, number> stock = { Apples = 3, Pears = 5 }
post(Table.keys(stock))
post(Table.sum(Table.values(stock)))

u8 red = 250
red += 10
post(red)
post((f16 0.1))
```

Output:

```text
314
true
Ada has 100 hp
a | b | c
{"Apples", "Pears"}
8
4
0.0999755859375
```

## Modules

| Module | Members |
| --- | --- |
| `Math` | constants `pi`, `tau`, `e`, `huge`, `nan`, `epsilon`, `maxInt`, `minInt`; `abs`, `sign`, `floor`, `ceil`, `round`, `trunc`, `fract`, `sqrt`, `cbrt`, `pow`, `exp`, `log`, `log2`, `log10`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `hypot`, `min`, `max`, `clamp`, `lerp`, `deg`, `rad`, `isNan`, `isInf`, `random`, `randomInt`, `seed` |
| `Text` | `trim`, `trimStart`, `trimEnd`, `lower`, `upper`, `contains`, `startsWith`, `endsWith`, `indexOf`, `lastIndexOf`, `count`, `replace`, `split`, `join`, `repeat`, `reverse`, `chars`, `code`, `fromCode`, `padLeft`, `padRight`, `toNumber`, `isNumber`, `format` |
| `Table` | `keys`, `values`, `has`, `get`, `merge`, `copy`, `reverse`, `sorted`, `contains`, `indexOf`, `count`, `slice`, `concat`, `unique`, `range`, `fill`, `sum`, `min`, `max`, `isEmpty` |
| `Time` | `now`, `millis`, `clock`, `sleep`, `date` (UTC parts), `format` (`YYYY-MM-DD hh:mm:ss`) |
| `Json` | `parse` (objects become dictionaries), `stringify`, `pretty` |
| `Fs` | `read`, `write`, `exists` (paths relative to the working directory; `..` is rejected) |

Hover over any member in VS Code for its signature and description. `Table` functions
return new collections; they never change their argument.

## Number types

| Type | Values |
| --- | --- |
| `number` = `i64`, `i8`, `i16`, `i32` | signed integers |
| `u8`, `u16`, `u32`, `u64` | non-negative integers (`u64` up to `Math.maxInt` for now) |
| `float` = `f64`, `f32`, `f16` | IEEE 754 floats; `f32` and `f16` round to their precision |

| Code | Result |
| --- | --- |
| `u8 x = 300` | error: a literal must fit (also reported by `clpp-boot check`) |
| `u8 x = 250` then `x += 10` | `4`: computed values wrap around like the CPU |
| `number n = 2.5` | error: use `Math.floor`, `Math.round` or `Math.trunc` |
| `float f = 3` | `3.0`: whole numbers widen to floats |
| `(f16 70000.0)` | error: largest `f16` is 65504 |

## `(Type value)`

`(Type value)` makes a value exactly that type. Numbers are converted, other values
must already have the type, or it is an error.

```clpp
post((u16 65535))       << 65535
post((float 3))         << 3.0
post((string "ok"))     << ok
post((string 5))        << error: expected string, got number (5)
```

Write a space after the type: `(len(x) > 1)` is a call, not a typed value.
