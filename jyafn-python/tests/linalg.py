import numpy as np
import jyafn as fn


a = [[2.0, 1.0], [1.0, 2.0]]
ainv = np.linalg.inv(a)
adet = np.linalg.det(a)
achol = np.linalg.cholesky(a)
b = np.array([2.0, 1.0])
solved = np.linalg.solve(a, b)


@fn.func
def inverse(a: fn.tensor[2, 2]):
    return np.isclose(np.linalg.inv(a), ainv).all()


assert inverse(a)


@fn.func
def det(a: fn.tensor[2, 2]):
    return np.isclose(np.linalg.det(a), adet)


assert det(a)


@fn.func
def cholesky(a: fn.tensor[2, 2]):
    return np.isclose(np.linalg.cholesky(a), achol).all()


assert cholesky(a)


@fn.func
def solve(a: fn.tensor[2, 2]):
    return np.isclose(np.linalg.solve(a, b), solved)


assert solve(a)
