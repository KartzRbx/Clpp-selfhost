# Arrays and `{}` inference

`array<T>` is an ordered list of values of type `T`, starting at index `0`.
`dictionary<K, V>` maps keys of type `K` to values of type `V`. Both use `{ ... }`
literals. An empty `{}` takes its type from the declaration it is assigned to.

```clpp
array<T> name = { a, b, c }
dictionary<K, V> name = { key = value }
```

> **Status:** parsing is implemented (`src/parser/parser.clp`). Type checking and
> `clpp-vm` are not written yet, so the output below is what the VM will print. See
> [ROADMAP](../../ROADMAP.md).

## Example: arrays

```clpp
!as ArrayExample

array<number> scores = { 10, 20, 30 }

post(len(scores))
post(scores[0])

push(scores, 40)
post(scores[3])

for (number score in scores) {
    post(score)
}
```

Output:

```text
3
10
40
10
20
30
40
```

## Example: empty collections

An empty `{}` has no element type of its own. The declared type on the left of `=`
decides what it becomes. This is called **target typing**.

```clpp
!as EmptyCollections

array<string> names = {}                 << empty array<string>
dictionary<string, number> ages = {}     << empty dictionary<string, number>
array<number> ids                        << no initializer: starts as an empty array

push(names, "Ada")
ages["Ada"] = 36

post(len(names))
post(ages["Ada"])
post(len(ids))
```

Output:

```text
1
36
0
```

When there is no declared type to read from, write the type in the constructor:

```clpp
auto table = dictionary<string, number>({})
auto list = array<number>({ 1, 2 })
```

## Example: dictionary keys

```clpp
!as DictionaryKeys

dictionary<string, number> stock = {
    Apples = 10,             << bare name: the key is the text "Apples"
    "Green pears" = 4,       << quoted: use this when the key has spaces
    ["Plums"] = 7            << bracketed: any expression can be the key
}

post(stock["Apples"])
post(stock["Green pears"])
post(stock["Plums"])
```

Output:

```text
10
4
7
```

## Rules

| Code | Result |
| --- | --- |
| `array<number> a = {}` | empty `array<number>` |
| `dictionary<string, number> d = {}` | empty `dictionary<string, number>` |
| `array<number> a` | empty `array<number>` (collections always start initialized) |
| `auto a = {}` | error: nothing tells the compiler the element type |
| `auto a` | declared but not initialized; the compiler does not guess a collection type |
| `array<number> a = { 1, "two" }` | error: `"two"` is not a `number` |

## Built-ins

| Call | Effect |
| --- | --- |
| `len(c)` | number of elements or entries |
| `push(a, v)` | append `v` to the end |
| `pop(a)` | remove and return the last element |
| `insert(a, i, v)` | insert `v` at index `i` |
| `remove(a, i)` | remove the element at index `i` |
| `find(a, v)` | index of `v`, or `-1` |
| `sort(a)` | sort in place |

## Common mistakes

```clpp
auto items = {}                      << error: element type unknown
array<number> items = {}             << ok

array<number> values = { 1, 2, 3 }
post(values[3])                      << runtime error: index 3 is out of range (last index is 2)
```

## See also

- [`post`](post.md)
