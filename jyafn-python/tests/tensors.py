import jyafn as fn
import numpy as np

if __name__ == "__main__":

    @fn.func
    def reduce_sum(mat: fn.tensor[2, 2]) -> fn.scalar:
        return np.sum(mat)

    print(reduce_sum.input_layout)
    print(reduce_sum.output_layout)
    print(reduce_sum(np.array([[1.0, 0.0], [0.0, 1.0]])))

    @fn.func
    def inner(vec: fn.tensor[2, 1]) -> fn.tensor[2, 1]:
        return np.array([[1, 0], [0, 1]]) @ vec

    print(inner.input_layout)
    print(inner.output_layout)
    print(inner(np.array([[1], [2]])))
