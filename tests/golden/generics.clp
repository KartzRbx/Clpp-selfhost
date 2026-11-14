<< Golden test: generic defs, template functions and optional values.

def Box<T> {
    T Value
    array<T> History

    void Set(T value) {
        push(@History, @Value)
        @Value = value
    }
}

def Pair<K, V> {
    K Key
    V Value
}

def Player {
    string Name
    number Health = 100
}

template<typename T> optional<T> First(array<T> values) {
    guard (len(values) > 0) else {
        return null
    }
    return values[0]
}

optional<Player> Find(array<Player> players, string name) {
    for (Player player in players) {
        if (player.Name == name) {
            return player
        }
    }
    return null
}

Box<number> count = Box<number> { Value = 5 }
count.Set(7)
count.Set(9)
post(count.Value .: " " .: count.History)

Box<string> label = Box<string> { Value = "a" }
label.Set("b")
post(label.Value .: " " .: label.History)

Pair<string, number> entry = Pair<string, number> { Key = "hp", Value = 30 }
post(entry.Key .: " = " .: entry.Value)

optional<number> first = First(array<number>(4, 5))
optional<number> none = First(array<number>())
post(first .: " " .: none)

array<Player> team = { Player { Name = "Ada" }, Player { Name = "Rui", Health = 50 } }
optional<Player> found = Find(team, "Rui")
if (found != null) {
    post(found.Name .: " has " .: found.Health)
}
optional<Player> missing = Find(team, "Zoe")
post(missing == null)
