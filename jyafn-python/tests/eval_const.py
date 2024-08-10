import jyafn as fn


@fn.func
def func(a: fn.scalar) -> fn.scalar:
    return fn.const(True).choose(fn.exp(a + 0.0) * 1.0, -1e-100)


print(func.get_graph().render())
print(func(1.0))
