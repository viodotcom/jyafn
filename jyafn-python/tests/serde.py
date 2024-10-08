import jyafn as fn
import os

os.makedirs("data", exist_ok=True)


@fn.func
def a_fun(a: fn.scalar, b: fn.scalar, c: fn.symbol) -> fn.scalar:
    return 2.0 * a + b + 1.0


print(a_fun.to_json())

a_fun.write("data/a_fun.jyafn")

other_fun = fn.read_fn("data/a_fun.jyafn")
print(a_fun(5, 6, "a"), other_fun(5, 6, "a"))
