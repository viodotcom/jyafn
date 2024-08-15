# import jyafn as fn
# import numpy as np
# import time

# tic = time.time()

# N = 10_000


# @fn.graph
# def big_io(x: fn.tensor[N]) -> fn.scalar:
#     return np.sum(x**2)


# @fn.graph
# def big_io(x: fn.tensor[N]) -> fn.tensor[N]:
#     return x * x


# @fn.graph
# def big_io(x: fn.tensor[N]) -> fn.unit:
#     return None


# @fn.graph
# def big_io(x: fn.unit) -> fn.tensor[N]:
#     return np.ones((N,))


# @fn.graph
# def big_io(x: fn.scalar) -> fn.tensor[N]:
#     return np.ones((N,)) * x


# toc = time.time()

# big_io = big_io.build()
# print("build graph", toc - tic)

# tic = time.time()
# big_io.render()
# toc = time.time()

# # open("data/rendered.ssa", "w").write(big_io.render())

# print("render qbe", toc - tic)


# tic = time.time()
# big_io.render_assembly()
# toc = time.time()

# print("compile", toc - tic)

# # assert big_io(np.ones((10_000))) == 10_000.0
