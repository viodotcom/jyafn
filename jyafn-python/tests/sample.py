import jyafn as fn
import timeit


@fn.func
def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
    return 2.0 * a + b + 1.0


def a_py_fun(a, b):
    return 2.0 * a + b + 1.0


@fn.func
def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
    for _ in range(200):
        a += 1
        b += a
    return b


def a_py_fun(a, b):
    for _ in range(200):
        a += 1
        b += a
    return b
