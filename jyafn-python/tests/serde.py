import jyafn as fn


@fn.func
def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
    return 2.0 * a + b + 1.0


print(a_fun.to_json())

a_fun.write("a_fun.jyafn")

other_fun = fn.read_fn("a_fun.jyafn")
print(a_fun(5, 6), other_fun(5, 6))
