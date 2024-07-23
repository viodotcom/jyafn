# JYAFN extensions

JYAFN extensions are useful for adding any extra capabilities to your `jyafn` code that the standard implementation cannot provide. The primary usecase is when one has very complicated thrid-party code which is not compatible with `jyafn` (e.g., makes calls to native functions). This code normally _could_ be reimplmemented in `jyafn`, but it would take time and money. If this code also happens to have an implementation which is not dependent (or minimally dependent) on Python, it's normally straightforward to create a C wrapper around it and expose it to JYAFN using the extension API. A prime example of this kind of code is the LightGBM library, which has its extension implemented in the `extensions` folder.

This crate provides a way to write extensions in idiomatic Rust _without_ unsafe code at all. It will use Rust macros to generate all the boilerplate code and take care of managing all the unsafety in a correct way.

## What are extensions?

Extensions are simply shared objects that conform to a given C API. These shared objects are stored somewhere in the filesystem, by default in `~/.jyafn/extensions` and more generally in the `JYAFN_PATH` environment variable. They are then loaded by jyafn whenever a new computational graph declares an _resource_ that depends on a resource type provided by the extension. As such, extensions come with some big expectations:
1. Extensions are _trusted_ code. They don't operate in the same limited and sandboxed environment of `jyafn` functions. They can run _anything_, from awesome intrincate algorithms to bugs to viruses.
2. Extensions are dependencies and dependencies have to be managed. They need to be installed beforehand from somewhere, otherwise the graphs depending on them will not work.
3. Extensions are system-dependent. There is no code once, run everywhere. They need to be compiled for every architecture.

Therefore, extensions should have their use kept to a minimum and should be reseved only for highly-reusable code.

## What are resources?

Resources are, simply put, just like objects in Python. They can be created and can be accessed using _methods_. Unlike Python objects, they are made to be stricty _immutable_ and have to be _serializable_ (i.e., represented as bytes in a file). An extension can declare a list of resources, each of which can declare a list of methods. A resource has a simple lifecycle:
1. It's created from some external binary data representation.
2. It's queried upon with methods at every method call in the `jyafn` function.
3. It's serialized into bytes to be put in a file together with the rest of the `jyafn` graph, potentially to be read and recreated in another machine.

Thus, we have a very simple interface for creating resources:
* `from_bytes`: builds a new resource from serialized binary data.
* `dump`: represents a resource as serialized data.
* `get_method`: gets information on a method (its function pointer, input and output layout) for the execution of a call.

A last method, `size`, is also to be implemented to keep track of _heap_ consumption of each resource.


## Where to go from here

If you intend to roll out your own extension, you can check out the `extensions` folder for sample implementations. The `dummy` extension has a basic, simple example of a very simple extension, while the `lightgbm` extension shows a fully operational extension for the LightGBM library.

However, if you, as most people, are looking for information of how to work with extensions as a user, check out the "Playing with extensions" (implementation pending) to learn more.
