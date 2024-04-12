import jyafn as fn

fn_file = "data/vbt.jyafn"

func = fn.read_fn(fn_file)
graph = func.get_graph()
metadata = graph.metadata

fn.describe(func)
fn.describe(graph)
fn.describe(fn_file)
fn.describe(None)
