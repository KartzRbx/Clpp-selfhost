<< Golden test: defs, methods, defaults, enums, identity, collections.

enum class Element {
    Fire,
    Water,
    Earth
}

def Counter {
    string Label = "counter"
    number Value
    array<number> History

    void Add(number amount) {
        @Value += amount
        push(@History, @Value)
    }

    number Last() {
        return @History[len(@History) - 1]
    }

    Counter Twice() {
        @Add(@Value)
        return @this
    }
}

def Fighter {
    string Name
    Element Kind = Element::Fire
    number Health = 100
}

Counter counter = Counter {}
counter.Add(5)
counter.Add(10)
post(counter.Label .: " " .: counter.Value .: " " .: counter.History .: " " .: counter.Last())
post(counter.Twice().Twice().Value)

Counter alias = counter
alias.Value = 1
post("shared: " .: counter.Value .: " same: " .: (alias == counter))
post("different: " .: (Counter {} == Counter {}))

array<Fighter> team = {
    Fighter { Name = "Ada", Kind = Element::Water },
    Fighter { Name = "Rui", Health = 50 }
}
team[1].Health -= 20
team[0].Health++
for (Fighter fighter in team) {
    post(fighter.Name .: " " .: fighter.Kind .: " " .: fighter.Health)
}
post(team[0].Kind == Element::Water)
post(Element::Earth)

dictionary<string, number> stock = { Apples = 10, "Green pears" = 4, ["Pl" .: "ums"] = 7 }
stock["Apples"] += 1
stock["Kiwis"] = 2
post(stock)
post(len(stock) .: " " .: find(stock, "Kiwis") .: " " .: stock["Plums"])
for (string fruit in stock) {
    post(fruit .: " = " .: stock[fruit])
}

array<number> values = array<number>(5, 3, 9, 1)
sort(values)
insert(values, 0, 0)
post(values .: " popped " .: pop(values) .: " removed " .: remove(values, 1) .: " left " .: values)
array<string> empty
dictionary<string, bool> flags
post(len(empty) .: " " .: len(flags))
