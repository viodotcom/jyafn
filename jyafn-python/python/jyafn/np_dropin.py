from .jyafn import *
import jyafn as fn

from typing import Any, Callable
import numpy as np
from functools import wraps


def array(a: Any) -> fn.tensor:
    """
    Creates an `fn.tensor`. This is similar to `np.array`. Also, `fn.tensor` inherits from
    `np.ndarray`. Therefore you can use both interchangebly. At the same time, `fn.tensor`
    performs some convenient overrdes that avoid some surprising behaviors (mostly
    related to logic operations) that arise when dealing with naïve `np.ndarray`.
    """
    return np.array(a).view(fn.tensor)


def reduction(
    identity=None,
) -> Callable[[Callable[[fn.Ref, fn.Ref], fn.Ref]], np.ufunc]:
    """Creates a numpy ufunc of the kind (x, y) -> z"""

    def _reduction(f: Callable[[fn.Ref, fn.Ref], fn.Ref]) -> np.ufunc:
        @wraps(f)
        def _f(a: fn.Ref, b: fn.Ref) -> fn.Ref:
            a, b = fn.make(a), fn.make(b)
            return f(a, b)

        if identity is None:
            return np.frompyfunc(_f, 2, 1)
        else:
            return np.frompyfunc(_f, 2, 1, identity=identity)

    return _reduction


def transformation(
    identity=None,
) -> Callable[[Callable[[fn.Ref], fn.Ref]], np.ufunc]:
    """Creates a numpy ufunc of the kind (x) -> y"""

    def _reduction(f: Callable[[fn.Ref], fn.Ref]) -> np.ufunc:
        @wraps(f)
        def _f(a: fn.Ref) -> fn.Ref:
            return f(fn.make(a))

        if identity is None:
            return np.frompyfunc(_f, 1, 1)
        else:
            return np.frompyfunc(_f, 1, 1, identity=identity)

    return _reduction


@reduction()
def equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a == b


@reduction()
def not_equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a != b


@reduction()
def greater(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a > b


@reduction()
def greater_equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a >= b


@reduction()
def less(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a < b


@reduction()
def less_equal(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a <= b


@reduction()
def minimum(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return (a > b).choose(b, a)


@reduction()
def maximum(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return (a > b).choose(a, b)


@reduction(identity=True)
def logical_and(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a & b


@reduction(identity=False)
def logical_or(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a | b


@reduction(identity=0.0)
def add(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a + b


@reduction(identity=1.0)
def multiply(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return a * b


@transformation()
def isnan(a: fn.Ref) -> fn.Ref:
    return fn.is_nan(a)


@transformation()
def isfinite(a: fn.Ref) -> fn.Ref:
    return fn.is_finite(a)


@transformation()
def isinf(a: fn.Ref) -> fn.Ref:
    return fn.is_infinite(a)


def __nan_to_num(x: Any, /, nan=0.0) -> fn.Ref:
    return fn.is_nan(x).choose(nan, x)


nan_to_num = np.vectorize(__nan_to_num)


@reduction(identity=0.0)
def unanadd(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return __nan_to_num(a) + __nan_to_num(b)


nansum = unanadd.reduce
nancumsum = unanadd.accumulate


@reduction(identity=1.0)
def unanmul(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return __nan_to_num(a) * __nan_to_num(b)


nanprod = unanmul.reduce
nancumprod = unanmul.accumulate


@reduction(identity=0.0)
def unnotancount(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return (~fn.is_nan(a)).to_float() + (~fn.is_nan(b)).to_float()


notnancount = unnotancount.reduce


def nanmean(a, axis=None):
    return nansum(a, axis) / notnancount(a, axis)


@reduction()
def unanmin(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return fn.is_nan(a).choose(b, (a > b).choose(a, b))


nanmin = unanmin.reduce


@reduction()
def unanmax(a: fn.Ref, b: fn.Ref) -> fn.Ref:
    return fn.is_nan(a).choose(b, (a > b).choose(b, a))


nanmax = unanmax.reduce


class linalg:
    @staticmethod
    def inv(a: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        return fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).inv(
            a=a.tolist()
        )

    @staticmethod
    def det(a: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        return fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).det(
            a=a.tolist()
        )

    @staticmethod
    def cholesky(a: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        return fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).cholesky(
            a=a.tolist()
        )

    @staticmethod
    def solve(a: np.ndarray, b: np.ndarray):
        if a.shape[0] != a.shape[1]:
            raise Exception(f"Matrix of shape {a.shape} is not square")
        if len(b.shape) != 2 or a.shape[1] != b.shape[0]:
            raise Exception(f"Incomptible shapes {a.shape} and {b.shape}")
        return fn.resource(type="SquareMatrix", data=str(a.shape[0]).encode()).solve(
            a=a.tolist(), v=b.tolist()
        )


DROP_IN: dict[np.ufunc, np.ufunc] = {
    np.equal: equal,
    np.not_equal: not_equal,
    np.greater: greater,
    np.greater_equal: greater_equal,
    np.less: less,
    np.less_equal: less_equal,
    np.maximum: maximum,
    np.minimum: minimum,
    np.max: maximum.reduce,
    np.min: minimum.reduce,
    np.logical_and: logical_and,
    np.logical_or: logical_or,
    np.all: logical_and.reduce,
    np.any: logical_or.reduce,
    np.add: add,
    np.multiply: multiply,
    np.isnan: isnan,
    np.isfinite: isfinite,
    np.isinf: isinf,
    np.nansum: nansum,
    np.nanprod: nanprod,
    np.nanmax: nanmax,
    np.nanmin: nanmin,
    np.linalg.inv: linalg.inv,
    np.linalg.det: linalg.det,
    np.linalg.cholesky: linalg.cholesky,
    np.linalg.solve: linalg.solve,
}
