import jyafn as fn


@fn.func
def illegal(x: fn.scalar) -> None:
    fn.assert_(2.0 + 2.0 == 5.0, "ingsoc")
