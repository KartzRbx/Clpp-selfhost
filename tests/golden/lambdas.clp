<< Golden test: lambdas, captures by value, callbacks, lambdas in fields and methods.

def Button {
    string Label
    auto OnClick = null

    void Click() {
        @OnClick(@Label)
    }
}

def Counter {
    number Count

    auto Incrementer(number step) {
        return func () -> void {
            @Count += step
        }
    }
}

array<number> Map(array<number> values, auto transform) {
    array<number> result = {}
    for (number value in values) {
        push(result, transform(value))
    }
    return result
}

void Run() {
    number factor = 3
    auto triple = func (number x) -> number {
        return x * factor
    }
    factor = 100
    post(triple(5))
    post(Map(array<number>(1, 2, 3), triple))
    post(Map(array<number>(1, 2, 3), func (number x) -> number { return x + 1 }))

    array<string> log = {}
    auto remember = func (string text) -> void {
        push(log, text)
    }
    remember("a")
    remember("b")
    post(log)

    Button ok = Button { Label = "OK", OnClick = func (string label) -> void { post("clicked " .: label) } }
    ok.Click()
    ok.OnClick("directly")

    Counter counter = Counter {}
    auto addTwo = counter.Incrementer(2)
    addTwo()
    addTwo()
    post(counter.Count)
    post(triple)
}

auto greet = func (string name) -> string {
    return "hi " .: name
}
post(greet("Ada"))
Run()
