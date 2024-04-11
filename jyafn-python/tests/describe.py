import jyafn as fn

fn_file = "data/vbt.jyafn"

func = fn.read_fn(fn_file)
graph = func.get_graph()
metadata = graph.metadata


def fmt_layout(layout: fn.Layout, indent=0) -> str:
    layout_str = str(layout)

    if len(layout_str) <= 90 - len("    input: "):
        return layout_str
    else:
        pretty = layout.pretty()
        indent_seq = "\n" + indent * " "
        return indent_seq.join(pretty.split("\n"))


def fmt_text(t: str, indent=0) -> str:
    indent_seq = "\n" + indent * " "
    return indent_seq + indent_seq.join(line.strip() for line in t.split("\n")).strip()


def fmt_size(size: int) -> str:
    rel_size = size
    units = ["", "k", "M", "G", "T"]
    unit_id = 0
    while rel_size > 1_000.0 and unit_id < len(units):
        rel_size /= 1_000.0
        unit_id += 1
    return f"{rel_size:.2f}{units[unit_id]}B"


OMMITTED_METADATA: list[str] = [
    "jyafn.created_at",
    "jyafn.doc",
    "jyafn.mem_size_estimate",
]


def describe_fn(func: fn.Function):
    print("Function name:", func.name)
    print("Signature:")
    print("    input:", fmt_layout(func.input_layout, indent=4))
    print("    output:", fmt_layout(func.output_layout, indent=4))
    print("Size in memory:", fmt_size(func.get_size()))
    print("Created at:", func.metadata.get("jyafn.created_at", "<none>"))
    print("Docstring:", fmt_text(func.metadata.get("jyafn.doc", "<none>"), indent=4))

    if any(key not in OMMITTED_METADATA for key in func.metadata):
        print("Custom metadata:")
        for key, val in func.metadata.items():
            if key not in OMMITTED_METADATA:
                print(f"    {key}: {val}")


def describe_graph(graph: fn.Graph):
    print("Graph name:", graph.name)
    print("Signature:")
    print("    input:", fmt_layout(graph.input_layout, indent=4))
    print("    output:", fmt_layout(graph.output_layout, indent=4))
    print("Size in memory:", fmt_size(graph.get_size()))
    print("Created at:", graph.metadata.get("jyafn.created_at", "<none>"))
    print("Docstring:", fmt_text(graph.metadata.get("jyafn.doc", "<none>"), indent=4))

    if any(key not in OMMITTED_METADATA for key in graph.metadata):
        print("Custom metadata:")
        for key, val in graph.metadata.items():
            if key not in OMMITTED_METADATA:
                print(f"    {key}: {val}")


def describe(thing: str | fn.Graph | fn.Function):
    if isinstance(thing, str):
        describe_fn(fn.read_fn(thing))
    elif isinstance(thing, fn.Graph):
        describe_graph(thing)
    elif isinstance(thing, fn.Function):
        describe_fn(thing)
    else:
        raise TypeError(f"jyafn cannot descrbe object of type {type(thing)}")


# print(func.to_json())
# describe(func)
# describe(graph)
# describe(fn_file)
# describe(None)

describe(func)
