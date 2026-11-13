<< Golden test: loops, break/continue, try/catch, guard, short-circuit order.

number calls = 0

bool Touch(bool result) {
    calls += 1
    return result
}

number FirstNegative(array<number> values) {
    number index = 0
    for (number value in values) {
        if (value < 0) {
            return index
        }
        index += 1
    }
    return -1
}

number SafeDivide(number a, number b) {
    guard (b != 0) else {
        return 0
    }
    return a / b
}

string Risky(number level) {
    if (level > 2) {
        report("level " .: level .: " is too high")
    }
    return "ok " .: level
}

<< Short-circuit: the right side must not run.
bool result = Touch(false) and Touch(true)
result = Touch(true) or Touch(false)
post("calls: " .: calls .: " result: " .: result)

array<number> picked = {}
number n = 0
while (true) {
    n += 1
    if (n % 2 == 0) {
        continue
    }
    if (n > 9) {
        break
    }
    push(picked, n)
}
post(picked)

for (number i in 0 .. 4) {
    for (string letter in "ab") {
        if (i == 2) {
            break
        }
        post(i .: letter)
    }
}

post(FirstNegative(array<number>(3, 4, -1, 5)) .: " " .: FirstNegative(array<number>(1)))
post(SafeDivide(10, 3) .: " " .: SafeDivide(1, 0))

number level = 0
while (level < 5) {
    try {
        post(Risky(level))
        if (level == 1) {
            level += 1
            continue
        }
    } catch (error) {
        post("caught: " .: error)
        break
    }
    level += 1
}
post("stopped at " .: level)

try {
    array<number> small = { 1 }
    post(small[3])
} catch (problem) {
    post("runtime error caught")
}

try {
    try {
        report("inner")
    } catch (first) {
        report("rethrown " .: first)
    }
} catch (second) {
    post(second)
}

number depth = 0
number Recurse(number left) {
    if (left == 0) {
        return 0
    }
    return 1 + Recurse(left - 1)
}
post("recursion: " .: Recurse(500))
