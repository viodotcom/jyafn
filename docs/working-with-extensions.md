# Working with extensions

## Installing extensions

The easiest way to get an extension is to install it using the `jyafn` CLI. This command comes pre-installed with the `jyafn` Python package already. To install an extension, you just need to do
```sh
jyafn ext get [URL]
```
Where `URL` is some link from where you can download the file. You can find some basic extensions regularly posted in the [release page](https://github.com/viodotcom/jyafn/releases/latest) for JYAFN. Just copy and paste the link and you are set to go!

>  ### 💡 Note
>  Extensions are _trusted code_. Just like anoy other app you install, they can execute arbitrary code on your machine, even during the installation process. Therefore, make sure you trust the source you are installing the extension from.
>

If you wish to remove an extension, you just need to type the following
```sh
jyafn ext rm [name]
```
Where `name` is the extension name. For more information on the ext command, see `jyafn ext --help`.


## Working with resources in Python code

Once you are have the extension installed you can access its resources directly from Python code. For example, consider the `dummy` extension, which provides a resource named `Dummy`, allowing you to divide a scalar by a number stored in the resource (silly, I know, but we need to start simple). A simple function accessing this resource would look like this:
```python
import jyafn as fn

# A stupidly simple resource:
dummy = fn.resource(
    "my_resource",      # the name of the resource. It has to be unique in a graph
    extension="dummy",  # the extension that provides this resource.
    resource="Dummy",   # the type of the resource.
    data=b"2.5",        # the description of the resource, in bytes. This is resource-specific.
                        # In the case of the `Dummy` resource, it is only a float.
)

@fn.func
def access_resource(x: fn.scalar) -> fn.scalar:
    # Just call the resource. The `Dummy` resource has a method named "get", which does
    # `x / 2.5` (the number definde above in the creation of the resource).
    return dummy.get(x=x)

# This works:
assert access_resource(2.5) == 1.0
```

Alternatively, you can also define the resource using the extension directy:
```python
dummy = fn.Extension("dummy").get("Dummy").load("my_resource", b"2.5")
```
Both approaches are equivalent.

## Where to go from here

As you have seen, extensions allow for powerful customization, but are also highly specific, each working in itw own little special way. You can read [the documentation](./extensions/index.md) of some extensions to understand what is available to you and how to use it.

If you wish to roll out your own extension, do not fear! The crate at [`jyafn-ext`](../jyafn-ext/) has your back. You can `cargo doc` it to go through its documentation. You can also look at the sample code in the [`extensions`](../jyafn-ext/extensions/) folder, which show how to use the crate to build an extension from end to end. You will find surprisingly short (even the lightgbm example comes at around a hundred lines, _with comments_!).
