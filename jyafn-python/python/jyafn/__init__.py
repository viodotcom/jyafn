from .jyafn import *

import jyafn as fn
import inspect
import types
import typing
import json
import itertools
import numpy as np
import datetime as pydatetime


from abc import ABC, abstractmethod
from typing import Any, Callable, Iterable
from dataclasses import dataclass

from .describe import describe  # re-export


__version__ = fn.__get_version()


# This needs to be here because this module redefines the name for basic python types.
_pytuple = tuple
_pylist = list
_pybool = bool


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
            origin = typing.get_origin(a)
            if issubclass(origin, BaseAnnotation):
                return origin.make_layout(typing.get_args(a))
            else:
                raise TypeError(f"cannot make layout of a generic of {origin}")
        case _:
            raise TypeError(f"Cannot make layout out of {a}")


def _input_from_annotation(name: str, a: Any) -> fn.Layout:
    """
    Gets an annotation and a field name and creates the associated input layout structure.
    """
    match a:
        case type() if issubclass(a, BaseAnnotation):
            layout = a.make_layout(())
        case types.GenericAlias():
            layout = typing.get_origin(a).make_layout(typing.get_args(a))
        case _:
            raise Exception(f"Invalid jyafn annotation for {name}: {a}")

    input = fn.input(name, layout)

    match a:
        case type() if issubclass(a, BaseAnnotation):
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
        case type() if a is type(None):
            ret = None
            layout = unit.make_layout(())
        case type() if issubclass(a, BaseAnnotation):
            ret = a.transform_output(ret)
            layout = a.make_layout(())
        case types.GenericAlias() if True:
            origin = typing.get_origin(a)
            ret = origin.transform_output(ret)
            layout = origin.make_layout(typing.get_args(a))
        case _:
            raise Exception(f"Invalid return annotation for jyafn: {a}")

    return fn.ret(ret, layout)


@dataclass
class GraphFactory:
    """
    A fectory of graphs. This will create a new instance of a graph, given a callable that
    builds the graph, eacho time the `build` method is invoked.
    """

    original: Callable
    """The callable used to build the graph in this factory"""
    metadata: dict[str, str]
    """
    The metadata to associate with this graph. This metadata does not override the default
    JYAFN tags.
    """
    debug: bool = False
    """Whether to print the QBE IR representation of this graph."""
    cache: bool = True
    """Whether to cache the graph after the first invocation of `build`."""
    _cached: fn.Graph | None = None
    """The cached value of the graph, if caching is enabled."""

    @property
    def __doc__(self) -> str | None:
        return self.original.__doc__

    def build(self) -> fn.Graph:
        """Creates a new `fn.Graph` instance using the original function."""
        if self.cache and self._cached is not None:
            return self._cached

        type_hints = typing.get_type_hints(self.original)
        with fn.Graph(name=f"{self.original.__qualname__}") as g:
            inputs = {
                arg: _input_from_annotation(arg, param)
                for arg, param in type_hints.items()
                if arg != "return"
            }
            _ret_from_annotation(
                self.original(**inputs), type_hints.get("return", inspect._empty)
            )

        for key, value in self.metadata.items():
            g.set_metadata(str(key), str(value))
        g.set_metadata("jyafn.created_at", pydatetime.datetime.now().isoformat())
        g.set_metadata("jyafn.mem_size_estimate", str(g.get_size()))
        if self.original.__doc__ is not None:
            g.set_metadata("jyafn.doc", self.original.__doc__)

        if self.debug:
            print(g.render())

        if self.cache:
            self._cached = g

        return g

    def compile(self) -> fn.Function:
        """
        Builds the computational graph invoking `build` and compiles the resulting graph
        into an `fn.Function`.
        """
        g = self.build()
        compiled = g.compile()
        compiled.original = self.original

        return compiled

    def __call__(self, *args, **kwargs) -> Any:
        """Calls this graph as a sub-graph call of the current graph."""
        return self.build()(*args, **kwargs)


def graph(
    *args, metadata: dict = {}, debug: bool = False, cache: bool = False
) -> GraphFactory:
    """
    Decorates a Python function and creates an `fn.GraphFactory` out of it. This factory
    can be used to create a graph by invoking the `build` method.

    Annotating _all_ input arguments is mandatory, while annotating the output value is
    optional, but will be checked.

    Examples:
    ```
    @fn.graph
    def two_x_plus_y(x: fn.scalar, y: fn.scalar) -> fn.scalar:
        return 2.0 * x + y

    @fn.graph(metadata={"foo": "bar"})
    def with_custom_metadata(x: fn.scalar, y: fn.scalar):
        return 2.0 * x + y

    # call the compiled JYAFN:
    assert two_x_plus_y.compile()(2.0, 1.0) == 5.0

    # call the compiled JYAFN:
    assert two_x_plus_y.compile()(2.0, 1.0) == 5.0
    ```
    """

    def inner(f: Any) -> GraphFactory:
        return GraphFactory(f, metadata=metadata, debug=debug, cache=cache)

    return inner(args[0]) if len(args) == 1 else inner


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
    def with_custom_metadata(x: fn.scalar, y: fn.scalar):
        return 2.0 * x + y

    # call the compiled JYAFN:
    assert two_x_plus_y(2.0, 1.0) == 5.0

    # call the original Python function:
    assert two_x_plus_y.original(2.0, 1.0) == 5.0
    ```
    """

    def inner(f: Any) -> fn.Function:
        return GraphFactory(f, metadata=metadata, debug=debug).compile()

    return inner(args[0]) if len(args) == 1 else inner


ANONYMOUS_COUNTER: dict[str, int] = {}


def __anonymous_name(kind: str) -> str:
    """Generates an anononymous name for a "kind" of thing."""
    num = ANONYMOUS_COUNTER.setdefault(kind, 0)
    name = f"{kind}_{num}"
    ANONYMOUS_COUNTER[kind] += 1

    return name


def py_val_putative_layout(obj: Any) -> fn.Layout:
    """
    Creates an `fn.Layout` out of any Python object, making the correct translation
    between Python and jyafn.

    For, example:
    ```
    fn.py_val_putative_layout(1)            # scalar
    fn.py_val_putative_layout(None)         # unit
    fn.py_val_putative_layout((True, 1))    # (bool, scalar)

    fn.py_val_putative_layout({"a":1, "b": True}) # struct { a: scalar, b: bool }
    ```
    """
    match obj:
        case fn.Layout():
            return obj
        case fn.Ref():
            return fn.putative_layout(obj)
        case None:
            return fn.Layout.unit()
        case int() | float():
            return fn.Layout.scalar()
        case _pybool():
            return fn.Layout.bool()
        case pydatetime.datetime():
            return fn.Layout.datetime()
        case pydatetime.date():
            return Layout.datetime("%Y-%m-%dT%H:%M:%S%.f")
        case str():
            return fn.Layout.symbol()
        case _pylist() | np.ndarray() if len(obj) > 0:
            return fn.Layout.list_of(putative_layout(obj[0]), len(obj))
        case _pylist():
            return fn.Layout.list_of(fn.Layout.scalar(), 0)
        case tuple():
            return fn.Layout.tuple_of(tuple(putative_layout(item) for item in obj))
        case dict():
            return fn.Layout.struct_of(
                {key: putative_layout(value) for key, value in obj.items()}
            )
        case _:
            raise TypeError(f"Cannot create putative layout of {obj}")


def mapping(
    obj: Any = {},
    *,
    key_layout: fn.Layout | type[BaseAnnotation] | types.GenericAlias | None = None,
    value_layout: fn.Layout | type[BaseAnnotation] | types.GenericAlias | None = None,
    name: str | None = None,
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
    match obj:
        case dict():
            it = iter(obj.items())
        case _:
            it = iter(obj)

    if name is None:
        name = __anonymous_name("mapping")

    if key_layout is None or value_layout is None:
        try:
            key, value = next(it)
        except StopIteration:
            raise Exception("empty iterator")

        if key_layout is None:
            key_layout = py_val_putative_layout(key)

        if value_layout is None:
            value_layout = py_val_putative_layout(value)

        it = itertools.chain([(key, value)], it)

    return fn.LazyMapping(name, make_layout(key_layout), make_layout(value_layout), it)


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
MILLISECOND: float = SECOND / 1_000
"""A millisecond, in seconds"""
MICROSECOND: float = SECOND / 1_000_000
"""A microsecond, in seconds"""


def index(indexable: list | np.ndarray | Iterable) -> IndexedList:
    """
    Creates an object that can be indexed by `fn.Ref`. Normally, `lists` and `ndarrays`
    can only be indexed by numbers and passing an `fn.Ref` as an index will return an
    `IndexError`. This function creates an object that understands that.

    Note however that _most of the time_ what you need is an `fn.mapping`. The
    implementation of `fn.index` can be quite costly, involving copying all of the
    indexable data in the stack. If your data is knwon beforehand (i.e., it's a big CSV
    file), you are surely better off with a mapping. However, if you data is comprised of
    non-constant `fn.Ref`s, `fn.index` is the way to go.
    """
    match indexable:
        case list():
            return IndexedList(indexable)
        case np.ndarray():
            return IndexedList(indexable.tolist())
        case _:
            # `list` has been redefined at this point, so cannot use the `list` constructor.
            return IndexedList([item for item in indexable])


def resource_type(
    *,
    type: str = "External",
    extension: str | None = None,
    resource: str | None = None,
    **kwargs,
) -> fn.ResourceType:
    """
    Creates a resource type. Call `load` on the object returned from this function to
    create a new resource of the current type.

    For more information, see `fn.resource`.
    """
    if type == "External" and extension is None:
        raise ValueError(f"resource type is External, but extension is not set")
    if type == "External" and resource is None:
        raise ValueError(f"resource type is External, but resource is not set")

    kwargs["type"] = type
    if extension is not None:
        kwargs["extension"] = extension
    if resource is not None:
        kwargs["resource"] = resource

    return fn.ResourceType.from_json(json.dumps(kwargs))


def resource(
    name: str | None = None,
    *,
    data: bytes,
    type: str = "External",
    extension: str | None = None,
    resource: str | None = None,
    **kwargs,
) -> fn.LazyResource:
    """
    Creates a resource of a given name to be used in a graph. Resources in JYAFN work
    very much like ordinary Python objects, except that they are immutable and strongly
    typed. Aside from that, you can call predefined methods on them.

    Resources were made to work together with extensions to extend JYAFN with niche
    functionality not offered by JYAFN. For your graph to work with extensions, you need
    to make sure that they are installed in your environment (you can use the `jyafn get`
    CLI utility for managing extensions).
    """
    if name is None:
        name = __anonymous_name("resource")
    return resource_type(
        type=type, extension=extension, resource=resource, **kwargs
    ).load(name, data)


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

    @classmethod
    def transform_output(cls, output: Any) -> Any:
        """Extra transformations to the output before its insertion in the graph."""
        if (
            isinstance(output, np.ndarray)
            and output.shape == ()
            and np.prod(output.shape) == 1
        ):
            return cls.transform_output(output.item())
        else:
            return output


class bool(BaseAnnotation):
    """Annotates the `bool` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.bool()

    @classmethod
    def transform_output(cls, output: Any) -> Any:
        """Extra transformations to the output before its insertion in the graph."""
        if (
            isinstance(output, np.ndarray)
            and output.shape == ()
            and np.prod(output.shape) == 1
        ):
            return cls.transform_output(output.item())
        else:
            return output


class datetime(BaseAnnotation):
    """Annotates the `datetime` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case ():
                return fn.Layout.datetime()
            case (format,) if isinstance(format, str):
                return fn.Layout.datetime(format)
            case (format,) if isinstance(format, bytes):
                return fn.Layout.datetime(format.decode("utf8"))
            case _:
                raise TypeError(f"Invalid args for datetime annotation: {args}")


class date(BaseAnnotation):
    """Annotates the `datetime` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        return fn.Layout.datetime("%Y-%m-%d")


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


class tensor(BaseAnnotation, np.ndarray):
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
        return array(input)  # from np_dropin

    @classmethod
    def transform_output(cls, output: Any) -> Any:
        if isinstance(output, np.ndarray):
            return output.tolist()
        else:
            return output

    def __eq__(self, other: Any) -> "tensor":
        return equal(self, other)

    def __ne__(self, other: Any) -> "tensor":
        return not_equal(self, other)

    def __gt__(self, other: Any) -> "tensor":
        return greater(self, other)

    def __ge__(self, other: Any) -> "tensor":
        return greater_equal(self, other)

    def __lt__(self, other: Any) -> "tensor":
        return less(self, other)

    def __le__(self, other: Any) -> "tensor":
        return less_equal(self, other)

    @property
    def max(self):
        return maximum.reduce

    @property
    def min(self):
        return maximum.reduce

    @property
    def any(self):
        return logical_or.reduce

    @property
    def all(self):
        return logical_and.reduce

    @staticmethod
    def __numpythonize(a):
        """
        This avoid infnite recusrion, by disabling __array_function__ invocation in
        case the drop-in is not found.
        """
        if isinstance(a, tensor):
            return a.view(np.ndarray)
        else:
            return a

    def __array_function__(self, func, types, args, kwargs):
        drop_in = DROP_IN.get(func, func)
        return drop_in(
            *map(self.__numpythonize, args),
            **{key: self.__numpythonize(val) for key, val in kwargs.items()},
        )

    def __array_ufunc__(self, ufunc, method, *args, **kwargs):
        drop_in = DROP_IN.get(ufunc, ufunc)
        return getattr(drop_in, method)(
            *map(self.__numpythonize, args),
            **{key: self.__numpythonize(val) for key, val in kwargs.items()},
        )


class tuple(BaseAnnotation):
    """Annotates the `tuple` layout."""

    @classmethod
    def make_layout(cls, args: tuple[Any, ...]) -> fn.Layout:
        match args:
            case fields if isinstance(fields, _pytuple):
                tup = []
                for field in fields:
                    match field:
                        case type():
                            tup.append(field.make_layout(()))
                        case types.GenericAlias():
                            tup.append(
                                typing.get_origin(field).make_layout(
                                    typing.get_args(field)
                                )
                            )
                        case _:
                            raise TypeError(
                                f"Invalid arg for tuple field annotation: {field}"
                            )
                return fn.Layout.tuple_of(_pytuple(tup))
            case _:
                raise TypeError(f"Invalid args for tuple annotation: {args}")


# This needs to be down here, after everything is said and done, because it overrides
# even _more_ Python stuff.
from .np_dropin import *
