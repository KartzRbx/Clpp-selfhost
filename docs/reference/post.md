# `post`

Prints a value to the standard output of `clpp-vm`, followed by a line break.

```clpp
post(value)
```

> **Status:** specified. `clpp-vm` does not exist yet, so the output below is what
> the VM will print, not something you can run today. See [ROADMAP](../../ROADMAP.md).

## Parameters

| Name | Type | Description |
| --- | --- | --- |
| `value` | any | The value to print. Numbers, strings, `bool` and collections are converted to text. |

## Return value

`void`

## Example

```clpp
!as PostExample

<<[ post runs at the top level of a file.
    There is no main() function. ]>>

string message = "Hello from CL++!"
number version = 1

post(message)
post(version)
post(version + 1 > 1)
post("version " .: version)
```

Run it:

```bash
clpp-vm post_example.clp
```

Output:

```text
Hello from CL++!
1
true
version 1
```

## Notes

1. Calls run in the order they appear. Top-level statements run from the top of the
   file to the bottom.
2. `post` is a built-in. You do not need a `link` to use it.
3. Use `.:` to join text. `+` only adds numbers.

## Common mistakes

```clpp
post("total: " + 3)      << error: '+' does not join text, use .:
post("total: " .: 3)     << ok, prints "total: 3"
```

## See also

- `warn` and `report` print to standard error. `report` also raises an error, which
  `try` / `catch` can catch.
- [Arrays and `{}` inference](collections.md)
