"""Detailed and extra lessons, merged into LESSONS by lessons.py.

REPLACE:      lessons with the same title are replaced.
INSERT_AFTER: (title, [lessons]) placed right after the lesson with that title.
APPEND:       lessons added at the end (Examples, Tests, Benchmarks).
"""

REPLACE = [
    # ------------------------------------------------------------------ syntax
    {"title": "Syntax", "menu": "CL++ Syntax", "blocks": [
        ("p", "This lesson explains every rule of how a CL++ file is written. Read it once; the rest of the "
              "tutorial builds on it."),
        ("h2", "A program is a list of statements"),
        ("p", "A `.clp` file runs from the first line to the last. Each line holds one **statement**: a declaration "
              "(`number x = 1`), an assignment (`x = 2`), a call (`post(x)`) or a control statement (`if`, `while`...). "
              "Functions, defs and enums can be declared anywhere at the top level and used from anywhere in the file."),
        ("example", '''post("first")
number x = 2
post(x * 10)
post(Double(x))

number Double(number value) {
    return value * 2
}'''),
        ("h2", "Statements end at the end of the line"),
        ("p", "A line break ends a statement, so the `;` you know from C++ is **optional**. Use `;` only to put two "
              "statements on the same line."),
        ("example", '''number a = 1
number b = 2; number c = 3
post(a + b + c)'''),
        ("h2", "When a statement continues on the next line"),
        ("p", "A statement continues on the next line when the line ends inside open brackets `( [ {`, ends with an "
              "operator, or the next line starts with an operator."),
        ("example", '''number total = 1 +
    2 +
    3
string message = "CL++ "
    .: "is "
    .: "readable"
post(total)
post(message)
post(Add(
    10,
    20
))

number Add(number a, number b) {
    return a + b
}'''),
        ("note", "A call's `(` must be on the same line as the function name: `post(x)`, not `post` and `(x)` on the "
                 "next line."),
        ("h2", "Blocks"),
        ("p", "Braces `{ }` group statements into a **block**. `if`, `while`, `for`, functions, defs and `match` arms "
              "use blocks. The opening `{` goes on the same line as the statement that owns it."),
        ("example", '''number level = 5
if (level > 3) {
    post("high level")
    post("well done")
}'''),
        ("h2", "Names"),
        ("list", [
            "A name starts with a letter or `_`, followed by letters, digits or `_`: `score`, `player2`, `_temp`.",
            "Names are **case-sensitive**: `score` and `Score` are different.",
            "Keywords (`if`, `def`, `match`, ...) cannot be used as names.",
            "Convention: `camelCase` for variables, `PascalCase` for functions, defs, fields and methods, "
            "`UPPER_CASE` for constants.",
        ]),
        ("example", '''number score = 1
number Score = 2
const number MAX_SCORE = 100
post(score .: " " .: Score .: " " .: MAX_SCORE)'''),
        ("h2", "Optional module name"),
        ("p", "A file may start with `!as Name`. The name is used when another file links it without `as`. It is optional."),
        ("example", '''!as Greetings

post("Hello from the Greetings module")'''),
        ("h2", "Summary"),
        ("table", [
            ["Rule", "Example"],
            ["one statement per line", "`number x = 1`"],
            ["`;` only to join statements on one line", "`a = 1; b = 2`"],
            ["blocks in braces, `{` on the same line", "`if (x > 1) {`"],
            ["strings in `\"...\"`, `'...'` or `` `...` ``", "`\"hi\"`"],
            ["comments start with `<<`", "`<< note`"],
            ["no `main()`: the file runs top to bottom", ""],
        ]),
    ]},

    # ------------------------------------------------------------------ comments
    {"title": "Comments", "menu": "CL++ Comments", "blocks": [
        ("p", "Comments are notes for people. The program ignores them. CL++ has three kinds."),
        ("table", [
            ["Kind", "Written as", "Ends at", "Use it for"],
            ["Line comment", "`<< text`", "the end of the line", "short notes"],
            ["Block comment", "`<<[ text ]>>`", "the first `]>>`", "long explanations, turning code off"],
            ["Documentation comment", "`<<{ !tag = \"...\" }>>`", "the first `}>>`", "describing the declaration below it"],
        ]),
        ("h2", "Line comments: <<"),
        ("p", "Everything from `<<` to the end of the line is ignored. A line comment can stand alone or follow code."),
        ("example", '''<< This whole line is a comment
number lives = 3 << the rest of this line is a comment too
post(lives)'''),
        ("h2", "Block comments: <<[ ... ]>>"),
        ("p", "A block comment can span several lines. It starts at `<<[` and ends at the first `]>>`."),
        ("example", '''<<[
    Game settings.
    These values are read once, at start.
]>>
number width = 640 <<[ pixels ]>>
post(width)'''),
        ("h2", "Turning code off"),
        ("p", "Wrap code in a block comment to disable it without deleting it."),
        ("example", '''post("this runs")
<<[
post("this does not run")
post("neither does this")
]>>
post("this runs too")'''),
        ("note", "Block comments do **not** nest: the first `]>>` ends the comment, even inside another `<<[`."),
        ("h2", "Comments inside strings are text"),
        ("example", '''post("<< this is not a comment, it is text")'''),
        ("h2", "Documentation comments: <<{ ... }>>"),
        ("p", "A documentation comment describes the function, variable, def or enum right **below** it. Each line "
              "inside is a tag: `!tag = \"value\"`. In VS Code, hovering the name shows these tags."),
        ("example", '''<<{
    !name = "Distance"
    !desc = "Distance between two numbers on a line."
    !params = "from: number, to: number"
    !return = "number, never negative"
    !example = "Distance(2, 7) is 5"
}>>
number Distance(number from, number to) {
    if (to > from) {
        return to - from
    }
    return from - to
}

post(Distance(2, 7))'''),
        ("table", [
            ["Tag", "Meaning"],
            ["`!name`", "the declared name; must match it exactly"],
            ["`!desc`", "what it does"],
            ["`!params`", "the parameters"],
            ["`!return`", "what it returns"],
            ["`!example`", "a short usage example"],
            ["`!deprecated`", "why not to use it any more, and what to use instead"],
            ["`!see`", "related declarations"],
        ]),
        ("list", [
            "Values must be in quotes: `!desc = \"text\"`.",
            "Each tag may appear once per comment.",
            "A documentation comment must be followed by a declaration, not by a statement such as `post(...)`.",
            "Unknown tags such as `!author` are reported as errors by the CL++ compiler.",
        ]),
        ("h2", "Notes for later: TODO and FIXME"),
        ("p", "Write `TODO`, `FIXME`, `BUG`, `HACK` or `XXX` in any comment. The editor highlights it and lists it "
              "in the Problems panel, so unfinished work is not forgotten."),
        ("example", '''<< TODO: read the volume from the settings file
number volume = 7
<< FIXME: volume above 10 is too loud
post(volume)'''),
        ("tip", "In VS Code, `Ctrl+/` comments or uncomments the selected lines with `<<`."),
    ]},

    # ------------------------------------------------------------------ sized numbers
    {"title": "Sized Numbers", "menu": "CL++ Sized Numbers", "blocks": [
        ("p", "`number` and `float` use 64 bits, which is enough for almost everything. **Sized numbers** let you "
              "choose exactly how big a number can be. They describe data precisely (a color channel is 0 to 255) "
              "and let the computer store large amounts of numbers in less memory."),
        ("h2", "Whole numbers with a sign: i8, i16, i32, i64"),
        ("p", "`i` means *integer*: positive, negative or zero. The number is the size in bits."),
        ("table", [
            ["Type", "Bits", "Smallest", "Largest", "Good for"],
            ["`i8`", "8", "-128", "127", "small offsets, tiny counters"],
            ["`i16`", "16", "-32 768", "32 767", "tile coordinates, audio samples"],
            ["`i32`", "32", "-2 147 483 648", "2 147 483 647", "counters, scores, most whole numbers"],
            ["`i64`", "64", "-9 223 372 036 854 775 808", "9 223 372 036 854 775 807", "timestamps, ids; same as `number`"],
        ]),
        ("h2", "Whole numbers without a sign: u8, u16, u32, u64"),
        ("p", "`u` means *unsigned*: zero or positive. The same number of bits reaches twice as high."),
        ("table", [
            ["Type", "Bits", "Smallest", "Largest", "Good for"],
            ["`u8`", "8", "0", "255", "bytes, RGBA color channels"],
            ["`u16`", "16", "0", "65 535", "ports, small ids"],
            ["`u32`", "32", "0", "4 294 967 295", "entity ids, sizes, bit masks"],
            ["`u64`", "64", "0", "9 223 372 036 854 775 807 (for now)", "large ids"],
        ]),
        ("h2", "Floating point: f16, f32, f64"),
        ("p", "Floats hold decimals using the IEEE 754 standard. Fewer bits mean fewer correct digits."),
        ("table", [
            ["Type", "Bits", "Correct digits", "Largest", "Good for"],
            ["`f16`", "16", "about 3", "65 504", "colors, weights, data sent over the network"],
            ["`f32`", "32", "about 7", "about 3.4 × 10³⁸", "positions, physics, graphics"],
            ["`f64`", "64", "about 15 to 17", "about 1.8 × 10³⁰⁸", "money, science; same as `float`"],
        ]),
        ("h2", "Declaring sized numbers"),
        ("example", '''u8 red = 255
u8 green = 128
i16 offsetX = -300
u32 entityId = 4000000000
f32 speed = 7.5
f16 alpha = 0.5
post(`color ${red}, ${green}`)
post(`offset ${offsetX}, id ${entityId}`)
post(`speed ${speed}, alpha ${alpha}`)'''),
        ("h2", "Rule 1: a literal must fit"),
        ("p", "A number written directly in the code (a **literal**) must fit in the type. Otherwise it is an error, "
              "found before the program runs."),
        ("error", '''u8 tooBig = 300'''),
        ("error", '''u16 negative = -1'''),
        ("h2", "Rule 2: computed values wrap around"),
        ("p", "When a calculation stored in a sized variable goes past the limit, it **wraps around**, exactly like "
              "the CPU does: after the largest value comes the smallest one. This is called *overflow*."),
        ("example", '''u8 counter = 250
counter += 10
post(counter)

i8 small = 127
small += 1
post(small)

u8 down = 3
down -= 5
post(down)'''),
        ("p", "Why `4`? `250 + 10` is `260`, and a `u8` only counts to `255`: `260 - 256 = 4`. Why `254`? `3 - 5` is "
              "`-2`, which wraps to `256 - 2`."),
        ("tip", "Wrapping is useful for hashes, random number generators and angles in steps. If it would be a bug, "
                "use a bigger type or check the value first."),
        ("h2", "Rule 3: whole numbers become floats, never the opposite"),
        ("p", "A whole number can be stored in any float type. A decimal cannot be stored in a whole-number type: "
              "you must decide how to round it with `Math.floor`, `Math.round` or `Math.trunc`."),
        ("example", '''link @clpp.math as Math

f32 ratio = 3
post(ratio)
float measured = 2.7
i32 rounded = Math.round(measured)
post(rounded)'''),
        ("error", '''i32 rounded = 2.7'''),
        ("h2", "Precision of floats"),
        ("p", "`0.1` cannot be stored exactly in binary. The fewer bits, the further away the stored value is."),
        ("example", '''f16 half = 0.1
f32 single = 0.1
f64 double = 0.1
post(half)
post(single)
post(double)'''),
        ("example", '''f16 big = 2049
post(big)
f16 limit = 65504
post(limit)'''),
        ("p", "`2049` became `2048`: above `2048`, an `f16` can only hold even numbers. Keep `f16` for values that "
              "do not need many digits."),
        ("h2", "Mixing sizes"),
        ("p", "Arithmetic is done with full 64-bit numbers. The result is converted only when it is stored in a sized "
              "variable or field."),
        ("example", '''u8 a = 200
u8 b = 100
number sum = a + b
u8 wrapped = a + b
post(sum)
post(wrapped)'''),
        ("h2", "Sized fields"),
        ("p", "Fields of a def follow the same rules every time they change."),
        ("example", '''def Pixel {
    u8 Red
    u8 Green
    u8 Blue
    f16 Alpha = 1.0
}

Pixel p = Pixel { Red = 250, Green = 10, Blue = 0 }
p.Red += 10
post(`${p.Red} ${p.Green} ${p.Blue} ${p.Alpha}`)'''),
        ("h2", "Choosing an exact type with (Type value)"),
        ("example", '''post((u16 65535))
post((f16 0.333))
post((i8 -128))'''),
        ("h2", "Summary"),
        ("table", [
            ["Situation", "Result"],
            ["literal fits", "stored as written"],
            ["literal does not fit", "error before running"],
            ["computed value does not fit", "wraps around"],
            ["whole number into float type", "converted"],
            ["decimal into whole-number type", "error: use `Math.round` and friends"],
            ["`f16` / `f32`", "rounded to their precision"],
        ]),
    ]},

    # ------------------------------------------------------------------ strings (more detail)
    {"title": "Strings", "menu": "CL++ Strings", "blocks": [
        ("p", "A `string` holds text. Write it in double quotes, single quotes or backticks (see **Template Strings**)."),
        ("example", '''string a = "double quotes"
string b = 'single quotes'
post(a)
post(b)'''),
        ("h2", "Length and characters"),
        ("p", "`len` counts characters, also accented letters and emoji. Positions start at 0. `text[i]` is a "
              "one-character string."),
        ("example", '''string greeting = "Olá, mundo"
post(len(greeting))
post(greeting[0])
post(greeting[2])
post(greeting[len(greeting) - 1])'''),
        ("h2", "Slices"),
        ("p", "`text[start .. end]` takes the characters from `start` up to, but not including, `end`."),
        ("example", '''string word = "pizzeria"
post(word[0 .. 5])
post(word[5 .. len(word)])'''),
        ("h2", "Joining text"),
        ("p", "`.:` joins two values as text. Numbers, booleans and collections are converted automatically."),
        ("example", '''string first = "Ada"
number age = 36
post(first .: " is " .: age .: " years old")'''),
        ("h2", "Escape characters"),
        ("p", "A backslash starts an escape sequence inside quotes."),
        ("table", [["Escape", "Result"], ["`\\n`", "new line"], ["`\\t`", "tab"], ["`\\\"`", "double quote"],
                   ["`\\'`", "single quote"], ["`\\\\`", "backslash"], ["`\\0`", "null character"]]),
        ("example", '''post("line one\\nline two")
post("name:\\tAda")
post("she said \\"hi\\"")
post("C:\\\\games")'''),
        ("h2", "Comparing strings"),
        ("p", "`==` compares the text. `<` and `>` compare in alphabetical (Unicode) order."),
        ("example", '''post("abc" == "abc")
post("apple" < "banana")'''),
        ("note", "Strings cannot be changed in place: `text[0] = \"x\"` is an error. Build a new string instead."),
        ("tip", "The **Text** lesson covers `split`, `trim`, `replace`, `upper`, `format` and more."),
    ]},
]

INSERT_AFTER = [
    ("Constants", [
        {"title": "Scope", "menu": "CL++ Scope", "blocks": [
            ("p", "The **scope** of a variable is the part of the program where its name can be used."),
            ("h2", "Global variables"),
            ("p", "A variable declared at the top level of a file is **global**: every function in the file can use it."),
            ("example", '''number score = 0

void AddPoints(number points) {
    score += points
}

AddPoints(10)
AddPoints(5)
post(score)'''),
            ("h2", "Local variables"),
            ("p", "A variable declared inside braces exists only until the closing brace."),
            ("example", '''number level = 3
if (level > 2) {
    string message = "hard"
    post(message)
}
post(level)'''),
            ("error", '''if (true) {
    number inside = 1
}
post(inside)'''),
            ("h2", "Same name in an inner block"),
            ("p", "An inner block may reuse a name. Inside the block the new variable hides the outer one."),
            ("example", '''number x = 1
if (true) {
    number x = 2
    post(x)
}
post(x)'''),
        ]},
    ]),
    ("Typed Values", [
        {"title": "Type Conversion", "menu": "CL++ Type Conversion", "blocks": [
            ("p", "CL++ never changes a value's type by itself when information could be lost. Convert explicitly."),
            ("table", [
                ["From", "To", "How"],
                ["`number`", "`string`", "`value .: \"\"` or a template `` `${value}` ``"],
                ["`string`", "`number` / `float`", "`Text.toNumber(text)`"],
                ["`float`", "`number`", "`Math.floor`, `Math.ceil`, `Math.round`, `Math.trunc`"],
                ["`number`", "`float`", "automatic: `float f = 3`"],
                ["any number", "a sized type", "`(u8 value)`"],
            ]),
            ("example", '''link @clpp.text as Text
link @clpp.math as Math

number points = 42
string text = points .: ""
post(len(text))
number parsed = Text.toNumber("123")
post(parsed + 1)
post(Math.floor(9.99))
float f = 7
post(f / 2)
post(Text.isNumber("12a"))'''),
        ]},
    ]),
    ("Operators", [
        {"title": "Operator Precedence", "menu": "CL++ Precedence", "blocks": [
            ("p", "When an expression has several operators, the ones higher in this table run first. Use parentheses "
                  "to choose the order yourself."),
            ("table", [
                ["Order", "Operators"],
                ["1 (first)", "calls `f()`, members `.`, `::`, indexes `[ ]`, `++` `--` after a name"],
                ["2", "`-x`, `not x`, `!x`, `~x`"],
                ["3", "`*`, `/`, `%`"],
                ["4", "`+`, `-`, `.:`"],
                ["5", "`&`, `|`, `^`, `shl`, `shr`"],
                ["6", "`..`"],
                ["7", "`==`, `!=`, `<`, `<=`, `>`, `>=`"],
                ["8", "`and`"],
                ["9", "`or`"],
                ["10", "`? :`"],
                ["11 (last)", "`=`, `+=`, `-=`, `*=`, `/=`, `%=`"],
            ]),
            ("example", '''post(2 + 3 * 4)
post((2 + 3) * 4)
post(1 + 2 .: "!")
post(not false and true)
post(10 > 5 ? "yes" : "no")'''),
        ]},
    ]),
    ("Arrays", [
        {"title": "Arrays in Depth", "menu": "CL++ Arrays in Depth", "blocks": [
            ("h2", "Ways to create an array"),
            ("example", '''array<number> a = { 1, 2, 3 }
array<number> b = array<number>(4, 5)
array<number> c = {}
array<number> d
post(`${a} ${b} ${c} ${d}`)'''),
            ("h2", "Arrays of arrays"),
            ("example", '''array<array<number>> grid = { { 1, 2, 3 }, { 4, 5, 6 } }
post(grid[1][2])
for (array<number> row in grid) {
    post(row)
}'''),
            ("h2", "Arrays are shared"),
            ("p", "Like objects, an array assigned to another variable is the same array."),
            ("example", '''array<string> original = { "a" }
array<string> same = original
push(same, "b")
post(original)'''),
            ("h2", "Errors"),
            ("error", '''array<number> numbers = { 1, "two" }'''),
        ]},
    ]),
    ("Functions", [
        {"title": "Recursion", "menu": "CL++ Recursion", "blocks": [
            ("p", "A function that calls itself is **recursive**. It needs a stopping case, or it never ends."),
            ("example", '''number SumTo(number n) {
    if (n == 0) {
        return 0
    }
    return n + SumTo(n - 1)
}

post(SumTo(100))'''),
            ("example", '''string Reverse(string text) {
    if (len(text) <= 1) {
        return text
    }
    return Reverse(text[1 .. len(text)]) .: text[0]
}

post(Reverse("pizza"))'''),
            ("note", "Very deep recursion stops with \"call depth limit exceeded\": prefer a loop for long sequences."),
        ]},
        {"title": "Command-Line Arguments", "menu": "CL++ Arguments", "blocks": [
            ("p", "`args()` returns the words written after the file name: `clpp-boot tool.clp a b`."),
            ("example", '''array<string> words = args()
post(`received ${len(words)} arguments`)
for (string word in words) {
    post(word)
}'''),
        ]},
    ]),
]

APPEND = [
    # ------------------------------------------------------------------ examples
    {"title": "Example Hello Game Loop", "menu": "Game Loop", "group": "Examples", "blocks": [
        ("p", "A tiny simulation: a player moves every frame until reaching the goal."),
        ("example", '''def Player {
    string Name
    f32 X
    f32 Speed = 2.5
}

Player hero = Player { Name = "Ada" }
const number GOAL = 10
number frame = 0
while (hero.X < GOAL) {
    hero.X += hero.Speed
    frame += 1
    post(`frame ${frame}: x = ${hero.X}`)
}
post(`${hero.Name} reached the goal in ${frame} frames`)'''),
    ]},
    {"title": "Example Inventory", "menu": "Inventory", "blocks": [
        ("p", "Count items with a dictionary and print them sorted."),
        ("example", '''link @clpp.table as Table

array<string> picked = { "coin", "gem", "coin", "key", "coin", "gem" }
dictionary<string, number> inventory = {}
for (string item in picked) {
    inventory[item] = Table.get(inventory, item, 0) + 1
}
for (string item in Table.sorted(Table.keys(inventory))) {
    post(`${item}: ${inventory[item]}`)
}'''),
    ]},
    {"title": "Example Bank Account", "menu": "Bank Account", "blocks": [
        ("p", "Methods, errors and `try`/`catch` working together."),
        ("example", '''def Account {
    string Owner
    number Balance

    void Withdraw(number amount) {
        if (amount > @Balance) {
            report(`${@Owner} cannot withdraw ${amount}: balance is ${@Balance}`)
        }
        @Balance -= amount
    }
}

Account account = Account { Owner = "Rui", Balance = 100 }
for (number amount in array<number>(30, 50, 40)) {
    try {
        account.Withdraw(amount)
        post(`withdrew ${amount}, left ${account.Balance}`)
    } catch (error) {
        post("refused: " .: error)
    }
}'''),
    ]},
    {"title": "Example Prime Numbers", "menu": "Prime Numbers", "blocks": [
        ("p", "The sieve of Eratosthenes with an array of booleans."),
        ("example", '''array<bool> Sieve(number limit) {
    array<bool> prime = {}
    for (number i in 0 .. limit + 1) {
        push(prime, i >= 2)
    }
    number p = 2
    while (p * p <= limit) {
        if (prime[p]) {
            number multiple = p * p
            while (multiple <= limit) {
                prime[multiple] = false
                multiple += p
            }
        }
        p += 1
    }
    return prime
}

array<bool> prime = Sieve(50)
array<number> found = {}
for (number i in 0 .. len(prime)) {
    if (prime[i]) {
        push(found, i)
    }
}
post(found)'''),
    ]},
    {"title": "Example Sorting", "menu": "Sorting", "blocks": [
        ("p", "Bubble sort written by hand, compared with the built-in `sort`."),
        ("example", '''array<number> BubbleSort(array<number> input) {
    array<number> items = {}
    for (number v in input) {
        push(items, v)
    }
    for (number i in 0 .. len(items)) {
        for (number j in 0 .. len(items) - 1 - i) {
            if (items[j] > items[j + 1]) {
                number temp = items[j]
                items[j] = items[j + 1]
                items[j + 1] = temp
            }
        }
    }
    return items
}

array<number> data = array<number>(5, 3, 9, 1, 7)
post(BubbleSort(data))
sort(data)
post(data)'''),
    ]},
    {"title": "Example Text Statistics", "menu": "Text Statistics", "blocks": [
        ("example", '''link @clpp.text as Text

string text = "the cat and the dog and the bird"
array<string> words = Text.split(text, " ")
dictionary<string, number> counts = {}
for (string word in words) {
    if (find(counts, word)) {
        counts[word] += 1
    } else {
        counts[word] = 1
    }
}
post(`${len(words)} words, ${len(counts)} different`)
post(`"the" appears ${counts["the"]} times`)
post(Text.upper(text[0 .. 7]))'''),
    ]},
    {"title": "Example Save and Load", "menu": "Save and Load", "blocks": [
        ("p", "Save game data as JSON in a file and read it back."),
        ("example", '''link @clpp.json as Json
link @clpp.fs as Fs

dictionary<string, number> save = { Level = 4, Coins = 250 }
Fs.write("save.json", Json.pretty(save))
post(Fs.read("save.json"))

dictionary<string, number> loaded = Json.parse(Fs.read("save.json"))
loaded["Coins"] += 10
post(`level ${loaded["Level"]}, coins ${loaded["Coins"]}`)'''),
    ]},
    {"title": "Example Shapes", "menu": "Shapes", "blocks": [
        ("p", "Inheritance and overriding to compute areas of different shapes."),
        ("example", '''link @clpp.math as Math

def Shape {
    string Name = "shape"
    float Area() {
        return 0
    }
}

def Circle:Shape {
    float Radius
    float Area() {
        return Math.pi * @Radius * @Radius
    }
}

def Rectangle:Shape {
    float Width
    float Height
    float Area() {
        return @Width * @Height
    }
}

array<Shape> shapes = { Circle { Name = "circle", Radius = 1 }, Rectangle { Name = "rectangle", Width = 2, Height = 3 } }
float total = 0
for (Shape shape in shapes) {
    post(`${shape.Name}: ${Math.round(shape.Area() * 100) / 100.0}`)
    total += shape.Area()
}
post(`total area ${Math.floor(total)}`)'''),
    ]},

    # ------------------------------------------------------------------ tests
    {"title": "Writing Tests", "menu": "Writing Tests", "group": "Tests", "blocks": [
        ("p", "A test is a CL++ program that calls `report` when something is wrong. `clpp-boot test tests` runs every "
              "`.clp` file in the `tests` folder and shows PASS or FAIL for each one."),
        ("example", '''number Clamp(number value, number low, number high) {
    if (value < low) {
        return low
    }
    if (value > high) {
        return high
    }
    return value
}

void Expect(bool condition, string what) {
    if (not condition) {
        report("failed: " .: what)
    }
}

Expect(Clamp(5, 0, 10) == 5, "value inside the range")
Expect(Clamp(-3, 0, 10) == 0, "value below the range")
Expect(Clamp(99, 0, 10) == 10, "value above the range")
post("clamp tests passed")'''),
        ("h2", "A failing test"),
        ("error", '''void Expect(bool condition, string what) {
    if (not condition) {
        report("failed: " .: what)
    }
}

Expect(2 + 2 == 5, "arithmetic")'''),
        ("h2", "Testing that an error happens"),
        ("example", '''bool Fails(number divisor) {
    try {
        post(10 / divisor)
    } catch (error) {
        return true
    }
    return false
}

post(Fails(0))
post(Fails(2))'''),
    ]},
    {"title": "Type Error Tests", "menu": "Type Error Tests", "blocks": [
        ("p", "To test that the type checker rejects bad code, write the code in a `.fail.clp` file and put the "
              "expected error in a `<< expect:` comment on the same line."),
        ("syntax", '''number count = "ten"  << expect: expected number, got string
post(missing)  << expect: 'missing' is not defined'''),
        ("p", "Run `clpp-boot check --expect tests/typecheck/*.fail.clp`. A test fails when an expected error does "
              "not appear, or when an error appears that was not expected."),
    ]},
    {"title": "Golden Tests", "menu": "Golden Tests", "blocks": [
        ("p", "Programs in `tests/golden` run twice: in the interpreter, and compiled to bytecode on `clpp-vm`. Both "
              "must print exactly the same. A `name.out` file next to a program pins the expected output."),
        ("table", [
            ["Command", "What it checks"],
            ["`clpp-boot test tests`", "every test program runs without errors"],
            ["`clpp-boot golden`", "interpreter and VM print the same, and match `.out` files"],
            ["`clpp-boot check --expect ...`", "the type checker reports exactly the expected errors"],
            ["`clpp-boot bootstrap`", "the compiler compiles itself and the result is identical"],
            ["`python docs/learn/build.py --check`", "every example of this site runs and type checks"],
        ]),
    ]},

    # ------------------------------------------------------------------ benchmarks
    {"title": "Benchmarks", "menu": "Benchmarks", "group": "Performance", "blocks": [
        ("p", "The programs in the `benchmarks` folder measure the interpreter (`clpp-boot file.clp`) against compiled "
              "bytecode on the virtual machine (`clpp-boot build` then `clpp-vm file.clpb`)."),
        ("bench",),
        ("h2", "Run them yourself"),
        ("syntax", '''python benchmarks/run.py'''),
        ("p", "The compiled program runs faster because names, fields and jumps are resolved once at compile time, "
              "registers replace variable lookups, and field access uses inline caches."),
        ("example", '''<< One of the benchmarks: recursive calls.
number Fib(number n) {
    if (n < 2) {
        return n
    }
    return Fib(n - 1) + Fib(n - 2)
}

post(Fib(20))'''),
    ]},
]
