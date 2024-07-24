import jyafn as fn
import traceback


@fn.func
def with_resources(x: fn.scalar) -> fn.scalar:
    resource_type = fn.ResourceType.from_json('{"type": "Dummy"}')
    resource = resource_type.load("my_resource", b"2.5")
    the_result = resource.get(x=x)

    return the_result


print(with_resources.get_graph().render())
assert with_resources(2.5) == 1.0

try:

    @fn.func
    def with_resources(x: fn.scalar) -> fn.scalar:
        resource_type = fn.ResourceType.from_json('{"type": "Dummy"}')
        resource = resource_type.load("my_resource", b"0.0")
        the_result = resource.get(x=x)

        return the_result

    print(with_resources(2.5))
except Exception:
    traceback.print_exc()
else:
    raise Exception("should raise")


try:

    @fn.func
    def with_resources(x: fn.scalar) -> fn.scalar:
        resource_type = fn.ResourceType.from_json('{"type": "Dummy"}')
        resource = resource_type.load("my_resource", b"0.0")
        the_result = resource.doesnt_exist(x=x)

        return the_result

    print(with_resources(2.5))
except Exception:
    traceback.print_exc()
else:
    raise Exception("should raise")


try:

    @fn.func
    def with_resources(x: fn.scalar) -> fn.scalar:
        resource_type = fn.ResourceType.from_json('{"type": "Dummy"}')
        resource = resource_type.load("my_resource", b"0.0")
        the_result = resource.panic()

        return the_result

    print(with_resources(2.5))
except Exception:
    traceback.print_exc()
else:
    raise Exception("should raise")


@fn.func
def with_resources(x: fn.scalar) -> fn.scalar:
    resource_type = fn.ResourceType.from_json('{"type": "Dummy"}')
    resource = resource_type.load("my_resource", b"2.5")
    the_result = resource.get(x=x)

    return the_result


serialized = with_resources.write("with_resources.jyafn")
deserialized = fn.read_fn("with_resources.jyafn")
assert deserialized(2.5) == 1.0
