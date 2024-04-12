import timeit

import jyafn as fn


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


print("py", a_py_fun(2, 3))
print("jyafn", a_fun(2, 3))

py_time = timeit.timeit(lambda: a_py_fun(2, 3))
jyafn_time = timeit.timeit(lambda: a_fun(2, 3))
jyafn_eval_time = timeit.timeit(lambda: a_fun.eval({"a": 2, "b": 3}))
print("py-time", py_time)
print("jyafn-time", jyafn_time)
print("jyafn-eval-time", jyafn_eval_time)
print("Slower", jyafn_time / py_time)
print("Slower-eval", jyafn_eval_time / py_time)
