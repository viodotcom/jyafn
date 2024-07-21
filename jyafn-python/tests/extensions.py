##
# You will need to compile and install the "dummy" extension for this example to work.
##

import jyafn as fn
import traceback


@fn.func
def with_resources(x: fn.scalar) -> fn.scalar:
    resource_type = fn.ResourceType.from_json(
        '{"type":"External","extension":"dummy","resource":"Dummy"}'
    )
    resource = resource_type.load("my_resource", b"2.5")
    the_result = resource.get(x=x)

    return the_result


print(with_resources.get_graph().render())
assert with_resources(2.5) == 1.0


try:

    @fn.func
    def with_resources(x: fn.scalar) -> fn.scalar:
        resource_type = fn.ResourceType.from_json(
            '{"type":"External","extension":"dummy","resource":"Dummy"}'
        )
        resource = resource_type.load("my_resource", b"2.5")
        the_result = resource.err(x=x)

        return the_result

    print(with_resources(2.5))
except Exception:
    traceback.print_exc()
else:
    raise Exception("should raise")


try:

    @fn.func
    def with_resources(x: fn.scalar) -> fn.scalar:
        resource_type = fn.ResourceType.from_json(
            '{"type":"External","extension":"dummy","resource":"Dummy"}'
        )
        resource = resource_type.load("my_resource", b"2.5")
        the_result = resource.panic(x=x)

        return the_result

    print(with_resources(2.5))
except Exception:
    traceback.print_exc()
else:
    raise Exception("should raise")