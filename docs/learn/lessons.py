"""Lessons of the CL++ tutorial. Each block is one of:

    ("p", text) ("h2", text) ("note", text) ("tip", text) ("list", [items])
    ("table", [header, row, ...]) ("syntax", code)
    ("example", code[, {"other.clp": text}])   run; its output is shown
    ("error", code)                            must fail; its error is shown

Text may use `code` and **bold**.
"""

LESSONS = [
    # ------------------------------------------------------------------ basics
    {"title": "Home", "menu": "CL++ Home", "group": "CL++ Tutorial", "hero": True, "blocks": [
        ("p", "CL++ is a small, statically typed language for games, tools and scripts. Programs read like C++ "
              "without the noise: no semicolons, no headers and no `main()` function."),
        ("example", '''post("Hello, CL++!")'''),
        ("h2", "Learning by examples"),
        ("p", "Every lesson shows a short explanation, then examples. Each example was run when this site was built, "
              "and the box under it shows exactly what it printed."),
        ("example", '''string name = "Ada"
number level = 3
post(`${name} reached level ${level + 1}`)'''),
        ("tip", "Start with the **Introduction**, or jump to any lesson from the menu on the left."),
    ]},
    {"title": "Introduction", "menu": "CL++ Intro", "blocks": [
        ("h2", "What is CL++?"),
        ("p", "CL++ is a language with C-style syntax, static types and a compiler written in CL++ itself. "
              "Source files end in `.clp`. They run in an interpreter while you develop, or compile to bytecode for "
              "the `clpp-vm` virtual machine."),
        ("h2", "Why CL++?"),
        ("list", [
            "**Readable:** a line break ends a statement, so `;` is optional.",
            "**Typed:** `number`, `float`, `string`, sized numbers like `u8` and `f16`, generics and `optional<T>`.",
            "**Checked before it runs:** wrong types, unknown names and missing cases are reported with a suggestion.",
            "**Practical:** math, text, tables, time, JSON and files are built in.",
            "**Good tooling:** a VS Code extension with errors as you type, hover, completion and formatting.",
        ]),
        ("example", '''def Player {
    string Name
    number Health = 100
}

Player hero = Player { Name = "Rui" }
post(hero.Name .: " has " .: hero.Health .: " hp")'''),
    ]},
    {"title": "Get Started", "menu": "CL++ Get Started", "blocks": [
        ("p", "Install the VS Code extension (it contains the interpreter, the compiler and the VM), create a file "
              "named `hello.clp` and press **Ctrl+F5** to run it."),
        ("example", '''post("Hello World!")'''),
        ("h2", "From the terminal"),
        ("table", [
            ["Command", "What it does"],
            ["`clpp-boot hello.clp`", "Runs the file in the interpreter."],
            ["`clpp-boot check hello.clp`", "Reports errors without running."],
            ["`clpp-boot build hello.clp --run`", "Type checks, compiles to bytecode and runs it on `clpp-vm`."],
            ["`clpp-boot fmt hello.clp`", "Formats the file."],
        ]),
        ("note", "A program starts at the top of the file and runs statement by statement. There is no `main()`."),
    ]},
    {"title": "Syntax", "menu": "CL++ Syntax", "blocks": [
        ("p", "A statement ends at the end of its line. Blocks use braces. Names are case-sensitive."),
        ("example", '''number apples = 3
number pears = 4
if (apples < pears) {
    post("more pears")
}'''),
        ("h2", "Semicolons are optional"),
        ("p", "You can still put two statements on one line by separating them with `;`."),
        ("example", '''number a = 1; number b = 2
post(a + b)'''),
        ("h2", "Long expressions"),
        ("p", "An expression continues on the next line when the line ends with an operator or the next line starts with one."),
        ("example", '''string message = "CL++ "
    .: "is "
    .: "readable"
post(message)'''),
    ]},
    {"title": "Output", "menu": "CL++ Output", "blocks": [
        ("p", "`post(value)` prints a value and a line break. Join values into text with `.:`."),
        ("example", '''post("I am learning CL++")
post(42)
post(3.5)
post(true)
post("Score: " .: 10 .: " points")'''),
        ("h2", "Collections print too"),
        ("example", '''post({ 1, 2, 3 })
post({ Gold = 5 })'''),
        ("note", "`+` adds numbers only. Use `.:` to join text."),
        ("error", '''post("total: " + 3)'''),
    ]},
    {"title": "Comments", "menu": "CL++ Comments", "blocks": [
        ("p", "`<<` starts a comment that runs to the end of the line. `<<[ ... ]>>` spans several lines."),
        ("example", '''<< This line is ignored
post("Hello") << so is the rest of this line

<<[ A comment
    on several lines ]>>
post("World")'''),
        ("h2", "Documentation comments"),
        ("p", "`<<{ ... }>>` documents the declaration below it. Editors show its tags when you hover the name."),
        ("example", '''<<{
    !desc = "Distance between two numbers."
    !params = "a: number, b: number"
    !return = "number, never negative"
}>>
number Distance(number a, number b) {
    if (a > b) {
        return a - b
    }
    return b - a
}

post(Distance(3, 10))'''),
        ("tip", "Write `<< TODO: ...` or `<< FIXME: ...` to leave a note; the editor lists it in the Problems panel."),
    ]},

    # ------------------------------------------------------------------ values
    {"title": "Variables", "menu": "CL++ Variables", "group": "Variables and Types", "blocks": [
        ("p", "Declare a variable with its type and a name. Give it a value with `=`."),
        ("syntax", '''type name = value'''),
        ("example", '''string name = "Ada"
number age = 36
float height = 1.65
bool online = true
post(name .: " " .: age .: " " .: height .: " " .: online)'''),
        ("h2", "Change a value"),
        ("example", '''number score = 10
score = 15
score += 5
post(score)'''),
        ("h2", "auto"),
        ("p", "`auto` takes the type from the value."),
        ("example", '''auto lives = 3
auto title = "Knight"
post(title .: " with " .: lives .: " lives")'''),
        ("h2", "Default values"),
        ("p", "A variable without a value starts at zero, an empty text or an empty collection."),
        ("example", '''number count
string label
array<number> items
post(count .: " [" .: label .: "] " .: len(items))'''),
    ]},
    {"title": "Constants", "menu": "CL++ Constants", "blocks": [
        ("p", "`const` makes a value that never changes. Constants are often written in capitals."),
        ("example", '''const number MAX_PLAYERS = 4
const float GRAVITY = 9.8
post(MAX_PLAYERS .: " players, gravity " .: GRAVITY)'''),
        ("note", "A constant needs a value when it is declared."),
    ]},
    {"title": "Data Types", "menu": "CL++ Data Types", "blocks": [
        ("table", [
            ["Type", "Holds", "Example"],
            ["`number`", "whole numbers (64-bit)", "`42`, `-7`, `0xFF`, `1_000`"],
            ["`float`", "decimal numbers (64-bit)", "`3.14`, `2.5e3`"],
            ["`string`", "text", "`\"hello\"`"],
            ["`bool`", "`true` or `false`", "`true`"],
            ["`array<T>`", "a list of values", "`{ 1, 2, 3 }`"],
            ["`dictionary<K, V>`", "keys with values", "`{ Gold = 5 }`"],
            ["`optional<T>`", "a value or `null`", "`null`"],
            ["`void`", "no value (functions)", ""],
        ]),
        ("example", '''number whole = 7
float half = 0.5
string word = "seven"
bool yes = true
post(whole * half)
post(word .: " " .: yes)'''),
        ("h2", "Types are checked"),
        ("p", "Putting a value of the wrong type in a variable is an error, found before the program runs."),
        ("error", '''number count = "ten"'''),
    ]},
    {"title": "Numbers", "menu": "CL++ Numbers", "blocks": [
        ("p", "`number` holds whole numbers and `float` holds decimals. Dividing two `number` values gives a whole number."),
        ("example", '''post(7 / 2)
post(7.0 / 2)
post(7 % 2)
post(2 * 3 + 1)'''),
        ("h2", "Writing numbers"),
        ("example", '''post(0xFF)
post(0b1010)
post(1_000_000)
post(2.5e3)'''),
        ("h2", "Whole numbers become floats"),
        ("p", "A `number` can go where a `float` is expected. The other way needs `Math.floor`, `Math.round` or `Math.trunc`."),
        ("example", '''float price = 10
post(price)'''),
        ("error", '''number rounded = 2.5'''),
    ]},
    {"title": "Sized Numbers", "menu": "CL++ Sized Numbers", "blocks": [
        ("p", "Sized types use a fixed amount of memory. Pick them for buffers, colors, network data or physics."),
        ("table", [
            ["Type", "Range"],
            ["`i8`, `i16`, `i32`, `i64`", "signed whole numbers, e.g. `i8` is -128 to 127"],
            ["`u8`, `u16`, `u32`, `u64`", "non-negative whole numbers, e.g. `u8` is 0 to 255"],
            ["`f16`, `f32`, `f64`", "half, single and double precision floats"],
        ]),
        ("example", '''u8 red = 255
i16 offset = -300
f32 speed = 7.5
post(red .: " " .: offset .: " " .: speed)'''),
        ("h2", "Overflow wraps around"),
        ("p", "A computed value that does not fit wraps around, like on the CPU."),
        ("example", '''u8 level = 250
level += 10
post(level)
i8 small = 127
small += 1
post(small)'''),
        ("h2", "Precision"),
        ("example", '''f16 half = 0.1
f32 single = 0.1
post(half)
post(single)'''),
        ("h2", "Literals must fit"),
        ("error", '''u8 tooBig = 300'''),
    ]},
    {"title": "Booleans", "menu": "CL++ Booleans", "blocks": [
        ("p", "A `bool` is `true` or `false`. Comparisons produce booleans. Combine them with `and`, `or` and `not`."),
        ("example", '''number age = 20
bool adult = age >= 18
post(adult)
post(adult and age < 30)
post(not adult or false)'''),
        ("note", "Conditions must be `bool`: write `if (count != 0)`, not `if (count)`."),
        ("error", '''number count = 3
if (count) {
    post("yes")
}'''),
    ]},
    {"title": "Strings", "menu": "CL++ Strings", "blocks": [
        ("p", "A `string` holds text in double or single quotes. `len` counts characters, also accents and emoji."),
        ("example", '''string greeting = "Olá, mundo"
post(len(greeting))
post(greeting[0])
post(greeting[0 .. 3])'''),
        ("h2", "Joining text"),
        ("example", '''string first = "Ada"
string last = "Lovelace"
post(first .: " " .: last)'''),
        ("h2", "Special characters"),
        ("table", [["Escape", "Result"], ["`\\n`", "new line"], ["`\\t`", "tab"], ["`\\\"`", "double quote"], ["`\\\\`", "backslash"]]),
        ("example", '''post("one\\ntwo")
post("say \\"hi\\"")'''),
        ("tip", "The **Text** module has `split`, `trim`, `replace`, `upper` and more."),
    ]},
    {"title": "Template Strings", "menu": "CL++ Template Strings", "blocks": [
        ("p", "Text in backticks can contain `${expression}`. The value of the expression is placed in the text."),
        ("example", '''string name = "Ada"
number hp = 80
post(`${name} has ${hp} hp`)
post(`next level: ${hp / 10 + 1}`)'''),
        ("example", '''array<string> items = { "sword", "shield" }
post(`first: ${items[0]}, total: ${len(items)}`)
post(`${len(items) > 1 ? "many" : "one"} items`)'''),
        ("note", "Write `\\${` to show the characters `${` literally."),
    ]},
    {"title": "Typed Values", "menu": "CL++ Typed Values", "blocks": [
        ("p", "`(Type value)` makes a value exactly that type. Numbers are converted; other values must already have the type."),
        ("example", '''post((float 3))
post((u16 65535))
post((string "ok"))'''),
        ("error", '''post((string 5))'''),
        ("note", "Write a space after the type: `(len(x) > 1)` is a call, not a typed value."),
    ]},

    # ------------------------------------------------------------------ operators
    {"title": "Operators", "menu": "CL++ Operators", "group": "Operators and Math", "blocks": [
        ("table", [
            ["Operator", "Meaning", "Example"],
            ["`+ - * / %`", "arithmetic", "`7 % 3` is `1`"],
            ["`.:`", "join as text", "`\"a\" .: 1`"],
            ["`== != < <= > >=`", "comparison", "`a == b`"],
            ["`and or not`", "logic", "`a and not b`"],
            ["`= += -= *= /= %=`", "assignment", "`x += 2`"],
            ["`++ --`", "add or subtract one", "`i++`"],
            ["`& | ^ shl shr ~`", "bits", "`1 shl 4`"],
            ["`a ? b : c`", "choose a value", "`hp > 0 ? \"alive\" : \"dead\"`"],
            ["`a .. b`", "range from a up to b", "`0 .. 5`"],
        ]),
        ("example", '''number x = 10
x += 5
x++
post(x)
post(x > 10 ? "big" : "small")
post((6 & 3) .: " " .: (6 | 3) .: " " .: (1 shl 4))'''),
    ]},
    {"title": "Math", "menu": "CL++ Math", "blocks": [
        ("p", "Link the `@clpp.math` module to use constants like `Math.pi` and functions like `Math.sqrt`."),
        ("example", '''link @clpp.math as Math

post(Math.max(5, 10))
post(Math.min(5, 10))
post(Math.sqrt(64))
post(Math.abs(-4.7))
post(Math.round(2.6))'''),
        ("h2", "Constants"),
        ("example", '''link @clpp.math as Math

post(Math.pi)
post(Math.e)
post(Math.huge > Math.maxInt)
post(Math.maxInt)'''),
        ("h2", "Rounding"),
        ("example", '''link @clpp.math as Math

post(Math.floor(2.7) .: " " .: Math.ceil(2.1) .: " " .: Math.trunc(-2.7) .: " " .: Math.round(-2.5))'''),
        ("h2", "Trigonometry and more"),
        ("example", '''link @clpp.math as Math

float angle = Math.rad(90)
post(Math.sin(angle))
post(Math.hypot(3, 4))
post(Math.pow(2, 10))
post(Math.clamp(150, 0, 100))
post(Math.lerp(0, 10, 0.25))'''),
        ("h2", "Random numbers"),
        ("example", '''link @clpp.math as Math

Math.seed(7)
number roll = Math.randomInt(1, 6)
post(roll >= 1 and roll <= 6)'''),
        ("h2", "Complete Math reference"),
        ("table", [
            ["Member", "Description"],
            ["`pi`, `tau`, `e`", "π, 2π and Euler's number"],
            ["`huge`, `nan`, `epsilon`", "infinity, not-a-number, float precision"],
            ["`maxInt`, `minInt`", "largest and smallest `number`"],
            ["`abs(x)`, `sign(x)`", "absolute value, -1 / 0 / 1"],
            ["`floor`, `ceil`, `round`, `trunc`, `fract`", "rounding"],
            ["`sqrt`, `cbrt`, `pow`, `exp`, `log`, `log2`, `log10`", "powers and logarithms"],
            ["`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`", "trigonometry (radians)"],
            ["`sinh`, `cosh`, `tanh`, `hypot`", "hyperbolic functions, length of (x, y)"],
            ["`min`, `max`, `clamp`, `lerp`", "limits and interpolation"],
            ["`deg`, `rad`", "convert radians and degrees"],
            ["`isNan`, `isInf`", "test special floats"],
            ["`random()`, `randomInt(a, b)`, `seed(n)`", "random numbers"],
        ]),
    ]},

    # ------------------------------------------------------------------ control flow
    {"title": "If Else", "menu": "CL++ If...Else", "group": "Control Flow", "blocks": [
        ("p", "`if` runs a block when its condition is `true`. Add `else if` and `else` for other cases."),
        ("example", '''number hour = 15
if (hour < 12) {
    post("Good morning")
} else if (hour < 18) {
    post("Good afternoon")
} else {
    post("Good evening")
}'''),
        ("h2", "Short form"),
        ("p", "The `? :` operator chooses between two values."),
        ("example", '''number hp = 0
post(hp > 0 ? "alive" : "defeated")'''),
    ]},
    {"title": "Match", "menu": "CL++ Match", "blocks": [
        ("p", "`match` compares a value with several cases. The first case that is equal runs. `_` matches anything."),
        ("example", '''string command = "jump"
match (command) {
    "go", "run" => post("moving")
    "jump" => post("jumping")
    _ => post("unknown command")
}'''),
        ("h2", "Matching enums"),
        ("p", "With an enum, every case must be covered, or a `_` case must be present. A forgotten case is an error."),
        ("example", '''enum class Light {
    Red,
    Yellow,
    Green
}

Light light = Light::Yellow
match (light) {
    Light::Red => post("stop")
    Light::Yellow => {
        post("slow down")
    }
    Light::Green => post("go")
}'''),
        ("error", '''enum class Light {
    Red,
    Green
}
Light light = Light::Red
match (light) {
    Light::Red => post("stop")
}'''),
    ]},
    {"title": "While Loop", "menu": "CL++ While Loop", "blocks": [
        ("p", "`while` repeats a block as long as its condition is `true`."),
        ("example", '''number i = 0
while (i < 5) {
    post(i)
    i += 1
}'''),
        ("h2", "do while"),
        ("p", "`do ... while` runs the block once before checking the condition."),
        ("example", '''number tries = 0
do {
    tries += 1
} while (tries < 3)
post(`tried ${tries} times`)'''),
    ]},
    {"title": "For Loop", "menu": "CL++ For Loop", "blocks": [
        ("p", "`for` goes through every element of an array, every key of a dictionary, every character of a string, or a range."),
        ("example", '''array<string> fruits = { "apple", "pear", "plum" }
for (string fruit in fruits) {
    post(fruit)
}'''),
        ("h2", "Counting with a range"),
        ("p", "`a .. b` counts from `a` up to, but not including, `b`."),
        ("example", '''for (number i in 0 .. 4) {
    post(i * i)
}'''),
        ("h2", "Characters and keys"),
        ("example", '''for (string letter in "CL++") {
    post(letter)
}
dictionary<string, number> ages = { Ada = 36, Rui = 20 }
for (string name in ages) {
    post(`${name} is ${ages[name]}`)
}'''),
    ]},
    {"title": "Break and Continue", "menu": "CL++ Break/Continue", "blocks": [
        ("p", "`break` leaves a loop. `continue` skips to the next round."),
        ("example", '''for (number i in 0 .. 10) {
    if (i == 6) {
        break
    }
    if (i % 2 == 0) {
        continue
    }
    post(i)
}'''),
    ]},
    {"title": "Guard", "menu": "CL++ Guard", "blocks": [
        ("p", "`guard (condition) else { ... }` checks something that must be true. When it is false, the `else` block must leave: `return`, `break`, `continue` or `report`."),
        ("example", '''number SafeDivide(number a, number b) {
    guard (b != 0) else {
        return 0
    }
    return a / b
}

post(SafeDivide(10, 2))
post(SafeDivide(10, 0))'''),
    ]},

    # ------------------------------------------------------------------ collections
    {"title": "Arrays", "menu": "CL++ Arrays", "group": "Collections", "blocks": [
        ("p", "An `array<T>` is an ordered list. Indexes start at 0."),
        ("example", '''array<string> heroes = { "Ada", "Rui", "Zoe" }
post(heroes[0])
post(len(heroes))
heroes[1] = "Lia"
post(heroes)'''),
        ("h2", "Adding and removing"),
        ("example", '''array<number> scores = { 30, 10 }
push(scores, 20)
insert(scores, 0, 5)
post(scores)
post(pop(scores))
post(remove(scores, 0))
post(scores)'''),
        ("h2", "Searching and sorting"),
        ("example", '''array<number> values = array<number>(4, 1, 3)
sort(values)
post(values)
post(find(values, 3))
post(find(values, 9))
post(values[0 .. 2])'''),
        ("note", "Reading an index that does not exist is an error. Valid indexes go from 0 to `len(array) - 1`."),
    ]},
    {"title": "Dictionaries", "menu": "CL++ Dictionaries", "blocks": [
        ("p", "A `dictionary<K, V>` maps keys to values. Assigning to a new key adds it."),
        ("example", '''dictionary<string, number> stock = { Apples = 10, Pears = 4 }
stock["Plums"] = 7
stock["Apples"] += 1
post(stock["Apples"])
post(len(stock))
post(stock)'''),
        ("h2", "Keys"),
        ("p", "A bare name like `Apples` is the text \"Apples\". Use quotes for keys with spaces and brackets for computed keys."),
        ("example", '''dictionary<string, number> prices = { Bread = 2, "Green tea" = 5, ["Ca" .: "ke"] = 8 }
post(prices["Green tea"] .: " " .: prices["Cake"])
post(find(prices, "Bread"))'''),
    ]},
    {"title": "Table Functions", "menu": "CL++ Table Functions", "blocks": [
        ("p", "The `@clpp.table` module works with arrays and dictionaries. Its functions return new collections."),
        ("example", '''link @clpp.table as Table
link @clpp.text as Text

dictionary<string, number> stock = { Apples = 3, Pears = 5 }
post(Table.keys(stock))
post(Table.sum(Table.values(stock)))
post(Table.get(stock, "Kiwis", 0))

array<number> values = array<number>(4, 1, 3, 1)
post(Table.sorted(values))
post(Table.unique(values))
post(Table.max(values) .: " " .: Table.count(values, 1))
post(Table.range(0, 5))'''),
        ("table", [
            ["Function", "Description"],
            ["`keys(d)`, `values(d)`, `has(d, k)`, `get(d, k, fallback)`, `merge(a, b)`", "dictionaries"],
            ["`sorted`, `reverse`, `unique`, `slice`, `concat`, `copy`", "new arrays"],
            ["`contains`, `indexOf`, `count`", "searching"],
            ["`sum`, `min`, `max`, `isEmpty`", "summaries"],
            ["`range(a, b)`, `fill(n, value)`", "building arrays"],
        ]),
    ]},

    # ------------------------------------------------------------------ functions
    {"title": "Functions", "menu": "CL++ Functions", "group": "Functions", "blocks": [
        ("p", "A function has a return type, a name and parameters. `void` means it returns nothing."),
        ("syntax", '''returnType Name(type parameter) {
    ...
}'''),
        ("example", '''void Greet(string name) {
    post("Hello " .: name)
}

number Square(number x) {
    return x * x
}

Greet("Ada")
post(Square(9))'''),
        ("h2", "Recursion"),
        ("example", '''number Factorial(number n) {
    if (n <= 1) {
        return 1
    }
    return n * Factorial(n - 1)
}

post(Factorial(10))'''),
        ("h2", "Arguments are checked"),
        ("error", '''number Square(number x) {
    return x * x
}
post(Square("nine"))'''),
    ]},
    {"title": "Lambdas", "menu": "CL++ Lambdas", "blocks": [
        ("p", "`func (parameters) -> type { ... }` creates a function value. It can be stored, passed and called."),
        ("example", '''auto double = func (number x) -> number {
    return x * 2
}
post(double(21))'''),
        ("h2", "Callbacks"),
        ("example", '''array<number> Map(array<number> values, auto transform) {
    array<number> result = {}
    for (number value in values) {
        push(result, transform(value))
    }
    return result
}

post(Map(array<number>(1, 2, 3), func (number x) -> number { return x * 10 }))'''),
        ("h2", "Captured variables"),
        ("p", "A lambda keeps a copy of the variables around it. Arrays and objects are shared, because they are references."),
        ("example", '''void Run() {
    number bonus = 5
    array<string> log = {}
    auto addBonus = func (number x) -> number { return x + bonus }
    auto remember = func (string text) -> void { push(log, text) }
    bonus = 100
    post(addBonus(1))
    remember("saved")
    post(log)
}

Run()'''),
    ]},

    # ------------------------------------------------------------------ defs
    {"title": "Defs", "menu": "CL++ Defs", "group": "Defs", "blocks": [
        ("p", "A `def` describes a kind of object: its fields and their default values. Create an object with `Type { Field = value }`."),
        ("example", '''def Player {
    string Name
    number Health = 100
    number Level = 1
}

Player ada = Player { Name = "Ada", Level = 3 }
post(ada.Name .: " " .: ada.Health .: " " .: ada.Level)
ada.Health -= 25
post(ada.Health)'''),
        ("h2", "Objects are shared"),
        ("p", "Assigning an object to another variable does not copy it; both names refer to the same object."),
        ("example", '''def Counter {
    number Value
}

Counter a = Counter {}
Counter b = a
b.Value = 5
post(a.Value)
post(a == b)'''),
    ]},
    {"title": "Methods", "menu": "CL++ Methods", "blocks": [
        ("p", "Functions inside a `def` are methods. Inside a method, `@this` is the object, and `@Name` is short for `@this.Name`."),
        ("example", '''def Account {
    string Owner
    number Balance

    void Deposit(number amount) {
        @Balance += amount
    }

    bool Withdraw(number amount) {
        if (amount > @Balance) {
            return false
        }
        @Balance -= amount
        return true
    }

    string Describe() {
        return `${@Owner}: ${@Balance}`
    }
}

Account account = Account { Owner = "Rui" }
account.Deposit(50)
post(account.Withdraw(80))
post(account.Withdraw(20))
post(account.Describe())'''),
    ]},
    {"title": "Inheritance", "menu": "CL++ Inheritance", "blocks": [
        ("p", "`def Child:Base` gets every field and method of `Base`. The child can add members and replace methods."),
        ("example", '''def Animal {
    string Name
    string Sound() {
        return "..."
    }
    string Speak() {
        return `${@Name} says ${@Sound()}`
    }
}

def Dog:Animal {
    string Sound() {
        return "woof"
    }
}

def Cat:Animal {
    string Sound() {
        return "meow"
    }
}

array<Animal> pets = { Dog { Name = "Rex" }, Cat { Name = "Mia" }, Animal { Name = "Fish" } }
for (Animal pet in pets) {
    post(pet.Speak())
}'''),
        ("note", "A `Dog` can be used wherever an `Animal` is expected, but not the other way round."),
    ]},
    {"title": "Enums", "menu": "CL++ Enums", "blocks": [
        ("p", "`enum class` lists named values. Use them with `Name::Value`."),
        ("example", '''enum class Element {
    Fire,
    Water,
    Earth
}

Element choice = Element::Water
post(choice)
post(choice == Element::Water)'''),
    ]},
    {"title": "Generics", "menu": "CL++ Generics", "blocks": [
        ("p", "A generic def has type parameters in `< >`. Each use picks the types."),
        ("example", '''def Box<T> {
    T Value
}

def Pair<K, V> {
    K Key
    V Value
}

Box<number> count = Box<number> { Value = 5 }
Pair<string, number> entry = Pair<string, number> { Key = "hp", Value = 30 }
post(count.Value + 1)
post(entry.Key .: " = " .: entry.Value)'''),
        ("h2", "Generic functions"),
        ("example", '''template<typename T> T Last(array<T> values) {
    return values[len(values) - 1]
}

post(Last(array<string>("a", "b", "c")))
post(Last(array<number>(4, 5)))'''),
        ("error", '''def Box<T> {
    T Value
}
Box<number> count = Box<number> { Value = 1 }
string text = count.Value'''),
    ]},
    {"title": "Optional", "menu": "CL++ Optional", "blocks": [
        ("p", "`optional<T>` holds a value or `null`. Check it with `!= null` before using it; inside the `if`, it is a normal value."),
        ("example", '''def Player {
    string Name
}

optional<Player> Find(array<Player> players, string name) {
    for (Player player in players) {
        if (player.Name == name) {
            return player
        }
    }
    return null
}

array<Player> team = { Player { Name = "Ada" }, Player { Name = "Rui" } }
optional<Player> found = Find(team, "Rui")
if (found != null) {
    post("found " .: found.Name)
}
post(Find(team, "Zoe") == null)'''),
        ("error", '''def Player {
    string Name
}
optional<Player> maybe = null
post(maybe.Name)'''),
    ]},

    # ------------------------------------------------------------------ errors
    {"title": "Exceptions", "menu": "CL++ Exceptions", "group": "Errors", "blocks": [
        ("p", "`report(message)` raises an error. `try { ... } catch (error) { ... }` catches it; `error` is the message."),
        ("example", '''number CheckAge(number age) {
    if (age < 0) {
        report(`invalid age: ${age}`)
    }
    return age
}

try {
    CheckAge(-3)
} catch (error) {
    post("caught: " .: error)
}'''),
        ("h2", "Runtime errors can be caught too"),
        ("example", '''try {
    array<number> small = { 1 }
    post(small[5])
} catch (error) {
    post(error)
}'''),
        ("h2", "Uncaught errors"),
        ("p", "An error that nobody catches stops the program and shows the line, a marker and a suggestion."),
        ("error", '''number a = 10
number b = 0
post(a / b)'''),
    ]},

    # ------------------------------------------------------------------ modules
    {"title": "Modules", "menu": "CL++ Modules", "group": "Modules and Library", "blocks": [
        ("p", "`link \"./file.clp\" as Name` uses another file. Only declarations marked `export` are visible."),
        ("example", '''link "./shapes.clp" as Shapes

Shapes.Square box = Shapes.Square { Side = 4 }
post(Shapes.Area(box))
post(Shapes.SIDES)''', {"shapes.clp": '''export const number SIDES = 4

export def Square {
    number Side = 1
}

export number Area(Square square) {
    return square.Side * square.Side
}
'''}),
        ("note", "A linked file runs once, before the file that links it."),
    ]},
    {"title": "Text", "menu": "CL++ Text", "blocks": [
        ("p", "The `@clpp.text` module has the usual string functions."),
        ("example", '''link @clpp.text as Text

string line = "  Ada, Rui, Zoe  "
array<string> names = Text.split(Text.trim(line), ", ")
post(names)
post(Text.join(names, " & "))
post(Text.upper("cl++") .: " " .: Text.replace("one two two", "two", "2"))
post(Text.contains("hello", "ell") .: " " .: Text.startsWith("hello", "he"))
post(Text.padLeft("7", 3, "0") .: " " .: Text.repeat("ab", 3))
post(Text.toNumber("0xFF") + 1)
post(Text.format("{} has {} hp", "Ada", 100))'''),
        ("table", [
            ["Function", "Description"],
            ["`trim`, `trimStart`, `trimEnd`, `lower`, `upper`", "clean up and change case"],
            ["`contains`, `startsWith`, `endsWith`, `indexOf`, `lastIndexOf`, `count`", "search"],
            ["`replace`, `split`, `join`, `repeat`, `reverse`, `chars`", "transform"],
            ["`padLeft`, `padRight`, `format`", "layout"],
            ["`code`, `fromCode`, `toNumber`, `isNumber`", "convert"],
        ]),
    ]},
    {"title": "Time", "menu": "CL++ Time", "blocks": [
        ("p", "The `@clpp.time` module reads the clock, measures durations and formats dates (UTC)."),
        ("example", '''link @clpp.time as Time

post(Time.format(0))
post(Time.format(1700000000))
dictionary<string, number> day = Time.date(951782400)
post(`${day["year"]}-${day["month"]}-${day["day"]}`)

float start = Time.clock()
Time.sleep(10)
post(Time.clock() - start >= 0.009)'''),
    ]},
    {"title": "JSON", "menu": "CL++ JSON", "blocks": [
        ("p", "The `@clpp.json` module reads and writes JSON. Objects become dictionaries."),
        ("example", '''link @clpp.json as Json

dictionary<string, number> save = Json.parse("{\\"level\\": 3, \\"coins\\": 250}")
post(save["level"] + 1)
save["coins"] += 50
post(Json.stringify(save))
post(Json.pretty({ Name = "Ada", Items = { "sword", "map" } }))'''),
    ]},
    {"title": "Files", "menu": "CL++ Files", "blocks": [
        ("p", "The `@clpp.fs` module reads and writes text files. Paths are relative to the folder the program runs in."),
        ("example", '''link @clpp.fs as Fs

Fs.write("notes.txt", "first line\\nsecond line")
post(Fs.exists("notes.txt"))
post(Fs.read("notes.txt"))'''),
        ("note", "For safety, paths may not contain `..`."),
    ]},

    # ------------------------------------------------------------------ tools
    {"title": "Compile and Run", "menu": "CL++ Compile and Run", "group": "Tools", "blocks": [
        ("p", "During development, `clpp-boot` runs a file directly. To ship a program, compile it to bytecode and run it on the virtual machine."),
        ("table", [
            ["Command", "What it does"],
            ["`clpp-boot game.clp`", "run in the interpreter"],
            ["`clpp-boot check game.clp`", "find errors without running"],
            ["`clpp-boot build game.clp`", "type check and compile to `game.clpb`"],
            ["`clpp-vm game.clpb`", "run the bytecode"],
            ["`clpp-boot fmt game.clp`", "format the file"],
        ]),
        ("p", "The compiler is written in CL++. `clpp-boot bootstrap` compiles the compiler with itself and checks that the result is identical."),
    ]},
    {"title": "Editor", "menu": "CL++ Editor", "blocks": [
        ("p", "The VS Code extension gives you:"),
        ("list", [
            "colors for code, comments, doc tags and `TODO` notes;",
            "errors as you type, with a suggestion for each one;",
            "hover with signatures and documentation comments;",
            "go to definition (F12), outline and completion after `.`, `@` and `::`;",
            "Format Document, snippets (`def`, `fn`, `for`, `match`, `doc`, ...) and **Ctrl+F5** to run.",
        ]),
        ("tip", "Type `doc` and press Tab above a function to start a documentation comment."),
    ]},
    {"title": "Reference", "menu": "CL++ Reference", "group": "Reference", "blocks": [
        ("h2", "Keywords"),
        ("table", [
            ["Keyword", "Use"],
            ["`if`, `else`, `while`, `do`, `for`, `in`, `break`, `continue`, `return`", "control flow"],
            ["`match`, `guard`, `try`, `catch`", "matching and errors"],
            ["`def`, `enum class`, `func`, `template`, `typename`", "declarations"],
            ["`link`, `as`, `export`, `const`, `auto`", "modules and variables"],
            ["`and`, `or`, `not`, `shl`, `shr`", "operators"],
            ["`true`, `false`, `null`", "values"],
        ]),
        ("h2", "Built-in functions"),
        ("table", [
            ["Function", "Description"],
            ["`post(value)`", "print a value"],
            ["`warn(value)`", "print a warning"],
            ["`report(message)`", "raise an error"],
            ["`len(value)`", "length of a string, array or dictionary"],
            ["`push`, `pop`, `insert`, `remove`, `find`, `sort`", "array helpers"],
            ["`args()`", "command-line arguments"],
        ]),
        ("h2", "Standard modules"),
        ("table", [
            ["Link", "Lesson"],
            ["`link @clpp.math as Math`", "Math"],
            ["`link @clpp.text as Text`", "Text"],
            ["`link @clpp.table as Table`", "Table Functions"],
            ["`link @clpp.time as Time`", "Time"],
            ["`link @clpp.json as Json`", "JSON"],
            ["`link @clpp.fs as Fs`", "Files"],
        ]),
    ]},
    {"title": "Exercises", "menu": "CL++ Exercises", "blocks": [
        ("p", "Try to write each program yourself, then compare with the solution."),
        ("h2", "1. FizzBuzz"),
        ("p", "Print the numbers 1 to 15, but `Fizz` for multiples of 3, `Buzz` for multiples of 5 and `FizzBuzz` for both."),
        ("example", '''for (number i in 1 .. 16) {
    if (i % 15 == 0) {
        post("FizzBuzz")
    } else if (i % 3 == 0) {
        post("Fizz")
    } else if (i % 5 == 0) {
        post("Buzz")
    } else {
        post(i)
    }
}'''),
        ("h2", "2. Inventory"),
        ("p", "Count how many of each item a player picked up."),
        ("example", '''array<string> picked = { "coin", "gem", "coin", "coin", "gem", "key" }
dictionary<string, number> inventory = {}
for (string item in picked) {
    if (find(inventory, item)) {
        inventory[item] += 1
    } else {
        inventory[item] = 1
    }
}
post(inventory)'''),
        ("h2", "3. Monsters"),
        ("p", "Use inheritance and `match` to describe enemies."),
        ("example", '''enum class Size {
    Small,
    Large
}

def Monster {
    string Name
    Size Kind = Size::Small
    number Attack() {
        match (@Kind) {
            Size::Small => {
                return 3
            }
            Size::Large => {
                return 9
            }
        }
        return 0
    }
}

def Dragon:Monster {
    number Attack() {
        return 20
    }
}

array<Monster> enemies = { Monster { Name = "Rat" }, Monster { Name = "Troll", Kind = Size::Large }, Dragon { Name = "Smaug" } }
number total = 0
for (Monster enemy in enemies) {
    post(`${enemy.Name} attacks for ${enemy.Attack()}`)
    total += enemy.Attack()
}
post(`total damage: ${total}`)'''),
    ]},
]


# Detailed and extra lessons (lessons_extra.py) replace, insert and append.
from lessons_extra import APPEND, INSERT_AFTER, REPLACE  # noqa: E402

_by_title = {lesson["title"]: lesson for lesson in REPLACE}
LESSONS = [
    {**_by_title[lesson["title"]], **({"group": lesson["group"]} if "group" in lesson else {})} if lesson["title"] in _by_title else lesson
    for lesson in LESSONS
]
for _after, _new in INSERT_AFTER:
    _at = next(i for i, lesson in enumerate(LESSONS) if lesson["title"] == _after) + 1
    LESSONS[_at:_at] = _new
_reference = next(i for i, lesson in enumerate(LESSONS) if lesson["title"] == "Reference")
LESSONS[_reference:_reference] = APPEND
