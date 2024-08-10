import jyafn as fn
import traceback

try:

    @fn.func
    def illegal(x: fn.scalar) -> None:
        fn.assert_(2.0 + 2.0 == 5.0, "ingsoc")

except Exception:
    traceback.print_exc()
else:
    raise Exception("should fail")
