<< Golden test: the interpreter and the compiled bytecode must print the same thing.

number Square(number value) {
    return value * value
}

number Factorial(number n) {
    if (n <= 1) {
        return 1
    }
    return n * Factorial(n - 1)
}

string Describe(number value) {
    return value % 2 == 0 ? "even" : "odd"
}

number total = 0
number index = 1
while (index <= 10) {
    total += Square(index)
    index++
}
post("sum of squares: " .: total)
post("10! = " .: Factorial(10))
post("7 is " .: Describe(7))
post(1 / 2 .: " " .: 1.0 / 2 .: " " .: 7 % 3 .: " " .: -5)
post(0xFF .: " " .: 0b1010 .: " " .: 1_000_000 .: " " .: 2.5e3)
post((6 & 3) .: " " .: (6 | 3) .: " " .: (6 ^ 3) .: " " .: (1 shl 4) .: " " .: (256 shr 2))
post(not true or (1 < 2 and 3 >= 3))

const i32 data = 150000;
post(data)

do {
    index -= 3
} while (index > 0)
post("after do-while: " .: index)

string word = "héllo world"
post(len(word) .: " " .: word[1 .. 5] .: " " .: word[0] .: " " .: find(word, "world"))
