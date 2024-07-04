#! /usr/bin/env bash

docker run -i                                   \
    --platform linux/amd64 --rm -v $(pwd):/io   \
    --entrypoint /bin/bash ghcr.io/pyo3/maturin \
<<EOF
    cd jyafn-python
    maturin build -i=3.8
    maturin build -i=3.9
    maturin build -i=3.10
    maturin build -i=3.11
    maturin build -i=3.12
EOF
