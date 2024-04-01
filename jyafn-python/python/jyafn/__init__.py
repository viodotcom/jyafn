from .jyafn import *

import jyafn as fn
import inspect
import types
import typing

from typing import Any


class Annotations:
    class scalar:
        pass

    class list:
        def __class_getitem__(cls, size: int) -> None:
            return types.GenericAlias(cls, (size,))

    class enum:
        def __class_getitem__(cls, options: tuple[str]) -> None:
            return types.GenericAlias(cls, options)


def input_from_annotation(name: str, a: Any) -> fn.Ref:
    match a:
        case Annotations.scalar:
            return fn.input(name)
        case types.GenericAlias():
            # Urgh... no __match_args__ in GenericAlias!
            match (typing.get_origin(a), typing.get_args(a)):
                case (list, (size,)):
                    return fn.list_input(name, size)
                case (enum, options):
                    return fn.enum_input(name, options)

    raise Exception(f"Invalid jyafn annotation for {name}: {a}")


def ret_from_annotation(ref: Any, a: Any) -> None:
    match a:
        case Annotations.scalar:
            return fn.ret(ref)
        case Annotations.list:
            return fn.list_ret(size)
        case types.GenericAlias():
            # Urgh... no __match_args__ in GenericAlias!
            match (typing.get_origin(a), typing.get_args(a)):
                case (list, (size,)):
                    if len(ref) != size:
                        raise ValueError(
                            f"Incompatible size returned: got {len(ref)}, expected {size}"
                        )
                    return fn.list_ret(list)
                case (enum, options):
                    raise NotImplementedError

    raise Exception(f"Invalid return annotation for jyafn: {a}")


def func(f) -> fn.Function:
    signature = inspect.signature(f)
    with fn.Graph(name=f"{f.__qualname__}@{id(f)}") as g:
        inputs = {
            arg: input_from_annotation(arg, param.annotation)
            for arg, param in signature.parameters.items()
        }
        ret_from_annotation(f(**inputs), signature.return_annotation)

    return g.compile()


# To not pollute the namespace, we leave these for last:
scalar = Annotations.scalar
list = Annotations.list
enum = Annotations.enum
