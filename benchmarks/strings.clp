<< Strings: concatenation, templates and slicing.
string text = ""
for (number i in 0 .. 5000) {
    text = text .: `${i % 10}`
}
number sevens = 0
for (number i in 0 .. len(text)) {
    if (text[i] == "7") {
        sevens += 1
    }
}
post(len(text) .: " " .: sevens)
