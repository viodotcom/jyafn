import jyafn as fn
import timeit


if __name__ == "__main__":

    # @fn.func
    # def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
    #     return 2.0 * a + b + 1.0

    # def a_py_fun(a, b):
    #     return 2.0 * a + b + 1.0

    # @fn.func
    # def a_fun(a: fn.scalar, b: fn.scalar) -> fn.scalar:
    #     for _ in range(200):
    #         a += 1
    #         b += a
    #     return b
    
    # def a_py_fun(a, b):
    #     for _ in range(200):
    #         a += 1
    #         b += a
    #     return b

    @fn.func
    def a_fun() -> fn.scalar:
        return 0.0
    
    def a_py_fun():
        return 0.0
    
    print("py", a_py_fun())
    print("jyafn", a_fun())

    py_time = timeit.timeit(lambda: a_py_fun())
    jyafn_time = timeit.timeit(lambda: a_fun())
    jyafn_eval_time = timeit.timeit(lambda: a_fun.eval({}))
    print("py-time", py_time)
    print("jyafn-time", jyafn_time)
    print("jyafn-eval-time", jyafn_eval_time)
    print("Slower", jyafn_time / py_time)
    print("Slower-eval", jyafn_eval_time / py_time)
