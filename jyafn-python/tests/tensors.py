import jyafn as fn
import numpy as np


@fn.func
def reduce_sum(mat: fn.tensor[2, 2]) -> fn.scalar:
    return np.sum(mat)


print(reduce_sum.input_layout)
print(reduce_sum.output_layout)
print(reduce_sum(np.array([[1.0, 0.0], [0.0, 1.0]])))


@fn.func
def reduce_min(mat: fn.tensor[2, 2]) -> fn.scalar:
    return np.sum(mat)


print(reduce_min.input_layout)
print(reduce_min.output_layout)
print(reduce_min(np.array([[1.0, 0.0], [0.0, 1.0]])))


@fn.func
def is_nan(mat: fn.tensor[2, 2]) -> fn.tensor[2, 2]:
    return np.isnan(mat).to_float()


print(is_nan.input_layout)
print(is_nan.output_layout)
print(is_nan(np.array([[1.0, 0.0], [np.nan, 1.0]])))


@fn.func
def reduce_nansum(mat: fn.tensor[2, 2]) -> fn.scalar:
    return np.nansum(mat)


print(reduce_nansum.input_layout)
print(reduce_nansum.output_layout)
print(reduce_nansum(np.array([[1.0, 0.0], [0.0, np.nan]])))


@fn.func
def reduce_nanmin(mat: fn.tensor[2, 2]):
    return np.nanmin(mat)


print(reduce_nanmin.input_layout)
print(reduce_nanmin.output_layout)
print(reduce_nanmin(np.array([[1.0, 0.0], [np.nan, 1.0]])))


@fn.func
def inner(vec: fn.tensor[2, 1]) -> fn.tensor[2, 1]:
    return np.array([[1, 0], [0, 1]]) @ vec


print(inner.input_layout)
print(inner.output_layout)
print(inner(np.array([[1], [2]])))
