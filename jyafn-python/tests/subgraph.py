import jyafn as fn


@fn.graph
def simple(a: fn.scalar, b: fn.scalar):
    return 2.0 * a + b


@fn.func()
def call_simple(a: fn.scalar, b: fn.scalar):
    return simple(a, b)


print(simple.build().render())
assert call_simple(2.0, 3.0) == 7.0
