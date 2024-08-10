# The `dummy` extension

This extension is intended for testing, showcasing and debugging purposes.

!!! success "Get this extension"
    === "Linux"
        ```sh
        jyafn ext get https://github.com/viodotcom/jyafn/releases/latest/download/dummy.so
        ```
    === "macOS"
        ```sh
        jyafn ext get https://github.com/viodotcom/jyafn/releases/latest/download/dummy.dylib
        ```

## The `Dummy` resource

The only resource declared by this extension is the `Dummy` resource. The idea of this resource is to perform a simple operation to showcase how extensions work. The chose operation is the division of a scalar input by a constant. This constant is a simple floating number that is passed at resource creation.

### Input data

The input data for the dummy resource is the constant to be used in the division. This constant has to be encoded as a string. For example,
```python
fn.resource("my_resource", extension="dummy", resource="Dummy", data=b"2.5")
```

### Methods

The dummy resource has these three methods:

```rust
// Gets the divison of `x` by the number supplied in the resource creation.
get(x: scalar) -> scalar;
// Always errors.
err(x: scalar) -> scalar;
// Always panics.
// NOTE: the panic is caught by the macros in `jyafn-ext` and transformed into an
// error. Panics can never propagate to jyafn code, ever!
panic(x: scalar) -> scalar;
```