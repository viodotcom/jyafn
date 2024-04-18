import jyafn as fn
import numpy as np


@fn.func(debug=True)
def index(idx: fn.scalar, foo: fn.scalar, bar: fn.scalar, baz: fn.scalar):
    return fn.index([foo, bar, baz])[idx]


print(index.get_graph().render_assembly())
print(index.get_graph().to_json())
print(index(2.5, 1, 2, 3))


e4 = np.eye(4)


@fn.func(debug=True)
def index(idx: fn.scalar):
    return fn.index(e4)[idx]


print(index.get_graph().render_assembly())
print(index.get_graph().to_json())
print(index(2.5))
