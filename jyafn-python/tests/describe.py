import jyafn as fn
import traceback

fn_file = "data/a_fun.jyafn"

func = fn.read_fn(fn_file)
graph = func.get_graph()
metadata = graph.metadata

fn.describe(func)
fn.describe(graph)
fn.describe(fn_file)

try:
    fn.describe(None)
except TypeError:
    traceback.print_exc()
else:
    raise Exception("should raise")
