import jyafn as fn

silly_map = fn.mapping(fn.symbol, fn.scalar, {"a": 2, "b": 4})


@fn.func
def foo(x: fn.symbol) -> fn.scalar:
    return silly_map[x]


# print("Output:")
# print(foo.get_graph().to_json())
# print(foo.get_graph().render())
# print(foo.get_graph().render_assembly())
# print(foo("a"))
# print(foo("b"))
# print(foo("c"))


@fn.func(metadata={"foo": "bar", "qux": "quz"})
def foo(x: fn.symbol):
    """
    This is a veeeery, very, very silly function, which returns nothing but `bar`, a
    nonsensical small integer.
    """
    return {"bar": silly_map.get(x, 6)}


print("Output:")
print(foo.get_graph().to_json())
print(foo.get_graph().render())
print(foo.get_graph().render_assembly())
print(foo("a"))
print(foo("b"))
print(foo("c"))

foo.write("silly-map.jyafn")
