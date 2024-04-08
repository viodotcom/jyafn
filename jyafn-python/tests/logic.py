import jyafn as fn


if __name__ == "__main__":

    @fn.func
    def relu(a: fn.scalar) -> fn.scalar:
        s = fn.sqrt(a)
        return (a >= 0.0).choose(s, 0.0)

    print(relu.get_graph().render())

    cases = [-1.0, 0.0, 1.0]
    for c in cases:
        print(f"relu({c}) = {relu(c)}")

    @fn.func
    def should_fail(a: fn.scalar) -> fn.scalar:
        if a.to_bool():
            return 0.0
        else:
            return 1.0

    print(f"should_fail({1.0}) = {should_fail(1.0)}")
