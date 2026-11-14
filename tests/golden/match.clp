<< Golden test: match on enums, strings and numbers.

enum class Element {
    Fire,
    Water,
    Earth
}

string Weakness(Element element) {
    match (element) {
        Element::Fire => {
            return "water"
        }
        Element::Water => {
            return "earth"
        }
        Element::Earth => {
            return "fire"
        }
    }
    return "none"
}

for (string command in Text.split("go,stop,jump,fly", ",")) {
    match (command) {
        "go", "jump" => post(command .: ": moving")
        "stop" => post(command .: ": stopped")
        _ => post(command .: ": unknown")
    }
}

number total = 0
for (number roll in 0 .. 6) {
    match (roll % 3) {
        0 => total += 10
        1 => {
            total += 1
            if (roll > 3) {
                continue
            }
            total += 100
        }
        _ => {}
    }
}
post("total: " .: total)
post(Weakness(Element::Fire) .: " " .: Weakness(Element::Earth))

link @clpp.text as Text
