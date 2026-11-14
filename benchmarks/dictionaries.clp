<< Dictionaries: insert, update and lookup.
dictionary<string, number> counts = {}
for (number i in 0 .. 20000) {
    string key = `k${i % 200}`
    if (find(counts, key)) {
        counts[key] += 1
    } else {
        counts[key] = 1
    }
}
post(len(counts) .: " " .: counts["k7"])
