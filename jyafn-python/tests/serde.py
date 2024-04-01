import jyafn as fn


if __name__ == "__main__":

    @fn.func
    def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
        return 2.0 * a + b + 1.0

    print(a_fun.dump())
    print(a_fun.to_json())

    other_fun = fn.Function.load(a_fun.dump())
    print(other_fun(5, 6))
