import jyafn as fn
import traceback

try:

    @fn.func(debug=True)
    def asserts(x: fn.scalar) -> None:
        fn.assert_(x > 0.0, "x must be positive")

    print(asserts.get_graph().to_json())
    print(asserts(1.0))
    print(asserts(-1.0))
except Exception:
    traceback.print_exc()
else:
    raise Exception("should raise")
