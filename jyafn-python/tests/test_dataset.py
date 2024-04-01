import jyafn as fn


if __name__ == "__main__":

    @fn.func
    def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
        return 2.0 * a + b + 1.0

    data = fn.Dataset.build(a_fun.input_layout, [
        {"a": 3, "b": 1},
        {"a": 2, "b": 2},
        {"a": 1, "b": 3},
    ])
    print(data)
    print(data.decode())

    mapped = data.map(a_fun)
    print(mapped)
    print(mapped.decode())
