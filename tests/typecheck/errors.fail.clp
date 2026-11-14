<< Type errors the checker must report: `clpp-boot check --expect` compares each
<< `expect:` comment with the errors found on that line, and fails on any other error.

link @clpp.math as Math
link @clpp.text as Text

def Player {
    string Name
    number Health = "full"  << expect: default of field 'Health'

    void Heal(number amount) {
        @Health += amount
        return amount  << expect: void function cannot return
    }

    string Title() {
        return @Health  << expect: return value: expected string
    }
}

number Twice(number value) {
    return value * 2
}

number count = "ten"  << expect: 'count': expected number, got string
string label = 42  << expect: 'label': expected string
bool ready = 1  << expect: 'ready': expected bool
float ratio = 3
number whole = 2.5  << expect: whole
Player hero = Player { Name = 7 }  << expect: field 'Name'
hero.Health = "low"  << expect: assignment: expected number
hero.Heal("ten")  << expect: argument 1 of Heal
Twice("two")  << expect: argument 1 of Twice
array<number> scores = { 1, "two", 3 }  << expect: array element
push(scores, "four")  << expect: pushed value
dictionary<string, number> ages = {}
ages[5] = 1  << expect: dictionary key
if (count) {  << expect: if needs a bool
    post("never")
}
post("total: " + count)  << expect: does not join text
post(ready and count)  << expect: 'and' needs bool
post(label < count)  << expect: cannot compare string with number
for (number item in 5) {  << expect: cannot loop over number
    post(item)
}
Text.trim(5)  << expect: argument 1 of trim
Math.sqrt("nine")  << expect: argument 1 of sqrt
hero.Mana = 3  << expect: has no member 'Mana'
