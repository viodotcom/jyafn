# type: ignore

from .jyafn import *

import jyafn as fn
import inspect
import types
import typing
import numpy as np

from abc import ABC, abstractmethod
from typing import Any
import datetime as pydatetime

from .np_dropin import *

class BaseAnnotation(ABC):
    """
    The base class of all annotations used to annotate parameters and return types of
    `@fn.func`. All classes derived from this, as well as `types.GenericAlias` objects
    can be converted into JYAFN Layouts.
    """

    @classmethod
    def __class_getitem__(cls, args) -> types.GenericAlias:
        """Creates a `types.GenericAlias` from this class."""
        return types.GenericAlias(cls, args)

    @classmethod
    @abstractmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        """Transform this class into a JYAFN layout."""
        pass

    @classmethod
    def transform_input(cls, input: Any) -> Any:
        """Extra transformations to the input after its declaration."""
        return input

    @classmethod
    def transform_output(cls, output: Any) -> Any:
        """Extra transformations to the output before its insertion in the graph."""
        return output


def make_layout(a: fn.Layout | type[BaseAnnotation] | types.GenericAlias) -> fn.Layout:
    """Gets an object and interprets that object as an `fn.Layout`."""
    match a:
        case fn.Layout():
            return a
        case type():
            return a.make_layout(())
        case types.GenericAlias():
            return typing.get_origin(a).make_layout(typing.get_args(a))
        case _:
            raise TypeError(f"Cannot make layout out of {a}")


class unit(BaseAnnotation):
    """Annotates the `unit` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.unit()


class scalar(BaseAnnotation):
    """Annotates the `scalar` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.scalar()


class bool(BaseAnnotation):
    """Annotates the `bool` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.bool()


class datetime(BaseAnnotation):
    """Annotates the `datetime` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case ():
                return fn.Layout.datetime()
            case (format,):
                return fn.Layout.datetime(format)
            case _:
                raise TypeError(f"Invalid args for datetime annotation: {args}")


class symbol(BaseAnnotation):
    """Annotates the `symbol` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.symbol()


class struct(BaseAnnotation):
    """Annotates the `struct` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case (fields,) if isinstance(fields, dict):
                struct = {}
                for name, field in fields.items():
                    match field:
                        case type():
                            struct[name] = field.make_layout(())
                        case types.GenericAlias():
                            struct[name] = typing.get_origin(field).make_layout(
                                typing.get_args(field)
                            )
                        case _:
                            raise TypeError(
                                f"Invalid arg for struct field annotation: {field}"
                            )
                return fn.Layout.struct_of(struct)
            case _:
                raise TypeError(f"Invalid args for struct annotation: {args}")


class list(BaseAnnotation):
    """Annotates the `list` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case (size,):
                return fn.Layout.list_of(fn.Layout.scalar(), size)
            case (
                ann,
                size,
            ) if isinstance(ann, types.GenericAlias):
                return fn.Layout.list_of(
                    typing.get_origin(ann).make_layout(typing.get_args(ann)),
                    size,
                )
            case (
                ty,
                size,
            ) if isinstance(ty, type):
                return fn.Layout.list_of(ty.make_layout(()), size)
            case _:
                raise TypeError(f"Invalid args for list annotation: {args}")


class tensor(BaseAnnotation):
    """
    Does not annotate any specific layout, but creates an input that is an `np.ndarray`
    populated with `fn.Ref`s. This can be used to make tensor operations backed by 
    `numpy`.
    """

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        layout = fn.Layout.scalar()
        for dim_size in reversed(args):
            layout = fn.Layout.list_of(layout, dim_size)

        return layout

    @classmethod
    def transform_input(cls, input: Any) -> Any:
        return np.array(input)

    @classmethod
    def transform_output(cls, output: Any) -> Any:
        if isinstance(output, np.ndarray):
            return output.tolist()
        else:
            return output


def _input_from_annotation(name: str, a: Any) -> fn.Layout:
    """
    Gets an annotation and a field name and creates the associated input layout structure.
    """
    match a:
        case type():
            layout = a.make_layout(())
        case types.GenericAlias():
            layout = typing.get_origin(a).make_layout(typing.get_args(a))
        case _:
            raise Exception(f"Invalid jyafn annotation for {name}: {a}")

    input = fn.input(name, layout)

    match a:
        case type():
            return a.transform_input(input)
        case types.GenericAlias():
            return typing.get_origin(a).transform_input(input)
        case _:
            raise Exception(f"Invalid jyafn annotation for {name}: {a}")


def _ret_from_annotation(ret: Any, a: Any) -> None:
    """
    Gets a "depythonizable" Python object filled with `fn.Ref`s and sets it as the return
    of the current graph, given an optionally annotated output layout.
    """
    match a:
        case inspect._empty:
            layout = fn.putative_layout(ret)
        case type():
            ret = a.transform_output(ret)
            layout = a.make_layout(())
        case types.GenericAlias():
            origin = typing.get_origin(a)
            ret = origin.transform_output(ret)
            layout = origin.make_layout(typing.get_args(a))
        case None:
            ret = unit.transform_output(a)
            layout = unit.make_layout(ret, ())
        case _:
            raise Exception(f"Invalid return annotation for jyafn: {a}")

    return fn.ret(ret, layout)


def func(*args, metadata: dict = {}, debug: bool = False) -> fn.Function:
    """
    Decorates a Python function and creates an `fn.Function` out of it, managing graph
    creation and compilation. You can still access the original function using the 
    `original` property in the returned object.

    Annotating _all_ input arguments is mandatory, while annotating the output value is
    optional, but will be checked.

    Examples:
    ```
    @fn.func
    def two_x_plus_y(x: fn.scalar, y: fn.scalar) -> fn.scalar:
        return 2.0 * x + y

    @fn.func(metadata={"foo": "bar"})
    def with_custom_metadata(x: fn.scalar, y: fn.scalar) -> fn.scalar:
        return 2.0 * x + y

    # call the compiled JYAFN:
    assert two_x_plus_y(2.0, 1.0) == 5.0

    # call the original Python function:
    assert two_x_plus_y.original(2.0, 1.0) == 5.0
    ```
    """
    def inner(f: Any) -> fn.Function:
        signature = inspect.signature(f)
        with fn.Graph(name=f"{f.__qualname__}") as g:
            inputs = {
                arg: _input_from_annotation(arg, param.annotation)
                for arg, param in signature.parameters.items()
            }
            _ret_from_annotation(f(**inputs), signature.return_annotation)

        for key, value in metadata.items():
            g.set_metadata(str(key), str(value))
        g.set_metadata("jyafn.created_at", pydatetime.datetime.now().isoformat())
        g.set_metadata("jyafn.mem_size_estimate", str(g.get_size()))
        if f.__doc__ is not None:
            g.set_metadata("jyafn.doc", f.__doc__)

        if debug:
            print(g.render())

        compiled = g.compile()
        compiled.original = f

        return compiled

    return inner(args[0]) if len(args) == 1 else inner


def mapping(
    name: str,
    key_layout: fn.Layout | type[BaseAnnotation] | types.GenericAlias,
    value_layout: fn.Layout | type[BaseAnnotation] | types.GenericAlias,
    obj: Any,
) -> fn.LazyMapping:
    """
    Creates a new key-value mapping to be used in a graph. Mappings in JYAFN work very
    much like ordinary Python dictionaries, except that they are immutable and strongly
    typed (you must always conform to the declared layouts).

    If you pass a Python dictionary as an `obj`, you _can_ use this mapping in multiple
    times in multiple graphs. However, if you pass a Generator (or other iterable), the
    mapping will be marked as consumed and an exception will be raised on reuse. This is
    done to avoid errors stemming from already spent iterators.
    """
    return fn.LazyMapping(name, make_layout(key_layout), make_layout(value_layout), obj)


def make_timestamp(
    time: pydatetime.date | pydatetime.datetime | pydatetime.timedelta,
) -> float:
    """
    Creates a JYAFN timestamp _constant_ out of a python datetime, date or timedelta
    object from the `datetime` package. This basically converts the input into its 
    correspondent value in microseconds.
    """
    match time:
        case pydatetime.datetime():
            return time.timestamp()
        case pydatetime.date():
            return (
                pydatetime.datetime.combine(time, pydatetime.time.min)
                .replace(tzinfo=pydatetime.timezone.utc)
                .timestamp()
            )
        case pydatetime.timedelta():
            return time.seconds
        case _:
            raise TypeError(f"Cannot make constant timestamp out of {type(time)}")


def make_datetime(
    time: pydatetime.date | pydatetime.datetime | pydatetime.timedelta,
) -> fn.Ref:
    """
    Creates a JYAFN datetime _constant_ out of a python datetime, date or timedelta
    object from the `datetime` package. This basically converts the input into its 
    correspondent value in microseconds.
    """
    return fn.fromtimestamp(fn.const(make_timestamp(time)))


SECOND: float = 1.0
"""A second"""
MINUTE: float = 60.0 * SECOND
"""A minute, in seconds"""
HOUR: float = 60.0 * MINUTE
"""An hout, in seconds"""
DAY: float = 24.0 * HOUR
"""A day, in seconds"""
