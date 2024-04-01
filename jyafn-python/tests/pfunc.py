import jyafn as fn

a = fn.input("a")
fn.ret(fn.sqrt(a) ** 2)
print(fn.current_graph().to_json())
print(fn.current_graph().render())
func = fn.current_graph().compile()
print(func.eval({"a": 4}))
