import jyafn as fn
import pickle


@fn.func
def a_fun(a: fn.scalar, b: fn.scalar) -> fn.struct[{"result": fn.scalar}]:
    return {"result": 2.0 * a + b + 1.0}


pickled_graph = pickle.dumps(a_fun.get_graph())
unpickled_graph: fn.Function = pickle.loads(pickled_graph)


pickled = pickle.dumps(a_fun)
unpickled: fn.Function = pickle.loads(pickled)

assert unpickled(1.0, 2.0)["result"] == 5.0
