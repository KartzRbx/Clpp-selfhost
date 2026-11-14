<< Arrays: push, index, sort.
array<number> values = {}
number i = 0
while (i < 100000) {
    push(values, (i * 7919) % 100003)
    i += 1
}
sort(values)
number sum = 0
for (number v in values) {
    sum += v
}
post(sum .: " " .: values[0] .: " " .: values[len(values) - 1])
