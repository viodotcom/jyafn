import jyafn as fn
import traceback


@fn.func
def relu(a: fn.scalar) -> fn.scalar:
    s = fn.sqrt(a)
    return (a >= 0.0).choose(s, 0.0)


print(relu.get_graph().render())

cases = [-1.0, 0.0, 1.0]
for c in cases:
    print(f"relu({c}) = {relu(c)}")

try:

    @fn.func
    def should_fail(a: fn.scalar) -> fn.scalar:
        if a.to_bool():
            return 0.0
        else:
            return 1.0

    print(f"should_fail({1.0}) = {should_fail(1.0)}")
except Exception:
    traceback.print_exc()
else:
    raise Exception("should fail")


@fn.func
def logic_with_symbols(favorite_color: fn.symbol) -> fn.symbol:
    return (favorite_color == "blue").choose("off you go", "aaaaaah!")


assert logic_with_symbols("blue") == "off you go"
assert logic_with_symbols("yellow") == "aaaaaah!"
