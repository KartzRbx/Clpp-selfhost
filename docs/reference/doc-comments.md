# Doc comments `<<{ }>>`

A doc comment describes the declaration written right below it. Editors and the
language server read its tags to show hover text.

```clpp
<<{ !tag = "value" }>>
```

> **Status:** the self-hosted lexer and parser support doc comments
> (`src/parser/parser.clp`). The tags are stored in `Node.Docs`. Hover in the
> language server is planned. See [ROADMAP](../../ROADMAP.md).

## Example

```clpp
!as Geometry

<<{
    !name = "Distance"
    !desc = "Distance between two points on a line."
    !params = "from: number, to: number"
    !return = "number, never negative"
    !example = "Distance(2, 7) is 5"
}>>
export number Distance(number from, number to) {
    if (to > from) {
        return to - from
    }
    return from - to
}

<<{ !desc = "Largest distance the game allows." }>>
const number MAX_DISTANCE = 1000

post(Distance(2, 7))
```

Output:

```text
5
```

## Tags

| Tag | Meaning |
| --- | --- |
| `!name` | Must match the declared name. |
| `!desc` | What the declaration does. |
| `!params` | Parameters. |
| `!return` | Return value. |
| `!example` | Usage example. |
| `!deprecated` | Why the declaration should not be used, and what to use instead. |
| `!see` | Related declarations. |

Each tag takes a quoted string. Separate tags with line breaks; `,` and `;` also work.

## Rules

| Code | Result |
| --- | --- |
| Doc comment above a function, variable or `def` | tags attached to that declaration |
| Doc comment above a statement such as `post(1)` | error |
| `!name` different from the declared name | error |
| Unknown tag such as `!author` | error |
| Same tag twice | error |
| Two doc comments in a row | error |

## Common mistakes

```clpp
<<{ !desc = Distance between points }>>      << error: the value must be quoted
<<{ !desc = "Distance between points" }>>    << ok

<<[ !desc = "..." ]>>                        << this is a plain block comment: tags are ignored
```

## See also

- [`post`](post.md)
