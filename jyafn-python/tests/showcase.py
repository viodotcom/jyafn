import boto3
import json
import ppca_rs
import numpy as np
import jyafn as fn

from time import time

s3 = boto3.client("s3")

latest_meta = json.load(
    s3.get_object(
        Bucket="fh-ca-data", Key="cheapest_providers/prod/predictions/latest_meta.json"
    )["Body"]
)
model = ppca_rs.PPCAModel.load(
    s3.get_object(
        Bucket="fh-ca-data",
        Key=f"cheapest_providers/prod/model/{latest_meta['model_id']}.bincode",
    )["Body"].read()
)

tic = time()


@fn.func
def from_components(
    comps: fn.tensor[model.state_size],
) -> fn.tensor[model.output_size]:
    total = comps @ model.transform.T + model.mean
    return total


toc = time()
print(f"Took {toc-tic}s")

with open("from_components.ssa", "w") as f:
    f.write(from_components.get_graph().render())

with open("from_components.s", "w") as f:
    f.write(from_components.get_graph().render_assembly())

from_components.write("from_components.jyafn")
