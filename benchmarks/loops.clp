<< Tight loop: additions, comparisons and a modulo.
number total = 0
number i = 0
while (i < 1000000) {
    if (i % 3 == 0) {
        total += i
    }
    i += 1
}
post(total)
