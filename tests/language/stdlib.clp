!as StdlibTests

link @clpp.math as Math
link @clpp.text as Text
link @clpp.table as Table

void Expect(bool condition, string what) {
    if (not condition) {
        report("failed: " .: what)
    }
}

bool Fails(number which) {
    try {
        if (which == 0) { u8 tooBig = (u8 300) }
        if (which == 1) { (number 2.5) }
        if (which == 2) { (string 5) }
        if (which == 3) { (f16 70000.0) }
        if (which == 4) { Math.sqrt(-1) }
        if (which == 5) { Text.toNumber("abc") }
        if (which == 6) { Text.format("{} {}", 1) }
        if (which == 7) { Table.min(array<number>()) }
    } catch (error) {
        return true
    }
    return false
}

<< ---------------------------------------------------------------- math
Expect(Math.pi > 3.14159 and Math.pi < 3.1416, "Math.pi")
Expect(Math.huge > Math.maxInt, "Math.huge is larger than any number")
Expect(-Math.huge < Math.minInt, "-Math.huge")
Expect(Math.isInf(Math.huge) and Math.isNan(Math.nan), "isInf / isNan")
Expect(Math.floor(2.7) == 2 and Math.ceil(2.1) == 3 and Math.round(-2.5) == -3 and Math.trunc(-2.7) == -2, "rounding")
Expect(Math.sqrt(16) == 4.0 and Math.pow(2, 10) == 1024 and Math.abs(-7) == 7, "sqrt / pow / abs")
Expect(Math.clamp(15, 0, 10) == 10 and Math.min(3, 9) == 3 and Math.max(3, 9) == 9, "clamp / min / max")
Expect(Math.abs(Math.sin(Math.pi)) < 0.000001 and Math.cos(0) == 1.0, "trigonometry")
Expect(Math.deg(Math.pi) == 180.0 and Math.lerp(0, 10, 0.25) == 2.5, "deg / lerp")
Math.seed(42)
number roll = Math.randomInt(1, 6)
Expect(roll >= 1 and roll <= 6, "randomInt in range")
float chance = Math.random()
Expect(chance >= 0.0 and chance < 1.0, "random in [0, 1)")

<< ---------------------------------------------------------------- text
Expect(Text.trim("  hi  ") == "hi" and Text.upper("abc") == "ABC" and Text.lower("ÀB") == "àb", "trim / case")
Expect(Text.split("a,b,,c", ",") == Text.split("a,b,,c", ",") or true, "split runs")
array<string> parts = Text.split("a,b,,c", ",")
Expect(len(parts) == 4 and parts[2] == "", "split keeps empty parts")
Expect(Text.join(parts, "-") == "a-b--c", "join")
Expect(Text.replace("one two two", "two", "2") == "one 2 2", "replace")
Expect(Text.contains("héllo", "éll") and Text.startsWith("héllo", "hé") and Text.endsWith("héllo", "lo"), "contains / starts / ends")
Expect(Text.indexOf("héllo", "l") == 2 and Text.lastIndexOf("héllo", "l") == 3, "indexOf counts characters")
Expect(Text.padLeft("7", 3, "0") == "007" and Text.padRight("ab", 4, ".") == "ab..", "padding")
Expect(Text.toNumber("0xFF") == 255 and Text.toNumber("-2.5") == -2.5 and Text.isNumber("12"), "toNumber")
Expect(Text.format("{} has {} hp", "Ada", 100) == "Ada has 100 hp", "format")
Expect(Text.code("A") == 65 and Text.fromCode(97) == "a" and Text.reverse("abc") == "cba", "char codes / reverse")
Expect(Text.repeat("ab", 3) == "ababab" and Text.count("banana", "an") == 2, "repeat / count")

<< ---------------------------------------------------------------- table
dictionary<string, number> stock = { Apples = 3, Pears = 5 }
Expect(Table.keys(stock) == Table.keys(stock) or true, "keys runs")
Expect(Text.join(Table.keys(stock), ",") == "Apples,Pears" and Table.sum(Table.values(stock)) == 8, "keys / values / sum")
Expect(Table.has(stock, "Pears") and not Table.has(stock, "Kiwis") and Table.get(stock, "Kiwis", 0) == 0, "has / get")
dictionary<string, number> merged = Table.merge(stock, { Pears = 9, Kiwis = 1 })
Expect(merged["Pears"] == 9 and len(merged) == 3 and stock["Pears"] == 5, "merge leaves the original alone")
array<number> values = array<number>(4, 1, 3, 1)
Expect(Table.sorted(values) == Table.sorted(values) or true, "sorted runs")
Expect(Text.join(Table.sorted(values), " ") == "1 1 3 4" and values[0] == 4, "sorted copies")
Expect(Text.join(Table.unique(values), " ") == "4 1 3" and Table.count(values, 1) == 2, "unique / count")
Expect(Text.join(Table.slice(values, 1, 99), " ") == "1 3 1" and Table.indexOf(values, 3) == 2, "slice clamps / indexOf")
Expect(Table.min(values) == 1 and Table.max(values) == 4 and Table.isEmpty(array<number>()), "min / max / isEmpty")
Expect(len(Table.range(0, 5)) == 5 and Table.fill(3, "x")[2] == "x", "range / fill")

<< ---------------------------------------------------------------- sized numbers
u8 small = 250
small += 10

Expect(small == 4, "u8 wraps around on arithmetic: " .: small)
i8 signed = 127
signed += 1
Expect(signed == -128, "i8 wraps to -128: " .: signed)
f16 half = 0.1
Expect(half == 0.0999755859375, "f16 rounds to half precision: " .: half)
f32 single = 0.1
Expect(single != 0.1 and single > 0.09999 and single < 0.10001, "f32 keeps about 7 digits")
float widened = 3
Expect(widened == 3.0 and (float 3) .: "" == "3.0", "number to float")
Expect((u16 65535) == 65535 and (i32 - 2147483648) == -2147483648, "typed expressions at the limits")
Expect((string "ok") == "ok" and (bool true), "typed expressions for other types")

def Pixel {
    u8 Red
    f16 Alpha = 1.0
}
Pixel pixel = Pixel { Red = 255 }
pixel.Red += 1
Expect(pixel.Red == 0 and pixel.Alpha == 1.0, "typed fields wrap and default")
Expect((Pixel pixel) == pixel, "typed expression with a def")

<< ---------------------------------------------------------------- errors
number index = 0
while (index < 8) {
    Expect(Fails(index), "case " .: index .: " must raise an error")
    index += 1
}

post("stdlib tests passed")
