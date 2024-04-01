import jyafn as fn


if __name__ == "__main__":

    @fn.func
    def relu(a: fn.scalar) -> fn.scalar:
        return (a >= 0.0).choose(a, 0.0)

    print(relu.get_graph().render())

    cases = [-1.0, 0.0, 1.0]
    for c in cases:
        print(f"relu({c}) = {relu(c)}")
