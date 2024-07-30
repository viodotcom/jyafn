import jyafn as fn


@fn.func
def tuples(tup: fn.tuple[fn.scalar, fn.scalar]) -> fn.tuple[fn.scalar, fn.scalar]:
    return tup[0] + tup[1], tup[0] - tup[1]


assert tuples((1.0, 3.0)) == (4.0, -2.0)
