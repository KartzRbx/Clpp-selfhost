<< Recursive Fibonacci: function calls and arithmetic.
number Fib(number n) {
    if (n < 2) {
        return n
    }
    return Fib(n - 1) + Fib(n - 2)
}

post(Fib(24))
