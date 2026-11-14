<< Golden test: template strings with ${} interpolation.

def Player {
    string Name
    number Health = 100

    string Status() {
        return `${@Name}: ${@Health} hp`
    }
}

string name = "Ada"
number level = 3
post(`Hello, ${name}!`)
post(`Level ${level} -> next is ${level + 1}`)
post(`nested: ${`inner ${name}`} done`)
post(`braces in code: ${{ 1, 2 }} and dictionary ${len({ A = 1, B = 2 })}`)
post(`escapes: \${not code} \` tab:\tend`)
post(`${level}${level}`)
post(``)
Player hero = Player { Name = "Rui" }
post(hero.Status())
array<string> names = { "a", "b" }
post(`first ${names[0]}, count ${len(names)}, ternary ${level > 2 ? "high" : "low"}`)
