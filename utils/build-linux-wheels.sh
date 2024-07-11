#! /usr/bin/env bash

set -e

docker run -i                                   \
    --platform linux/amd64 --rm -v $(pwd):/io   \
    --entrypoint /bin/bash ghcr.io/pyo3/maturin \
<<EOF
    cd vendored/qbe
    make clean
    make qbe
    ./qbe --help
    cd ../../jyafn-python
    maturin build --release -i=3.8
    maturin build --release -i=3.9
    maturin build --release -i=3.10
    maturin build --release -i=3.11
    maturin build --release -i=3.12
EOF

docker run -i                                   \
    --platform linux/amd64 --rm -v $(pwd):/work \
    --entrypoint /bin/bash python:3.10          \
<<EOF
    set -e
    cd /work/target/wheels/
    ls . | grep 310 | grep manylinux | xargs pip install
    cd ../../jyafn-python
    python tests/serde.py
EOF

docker run -i                                   \
    --platform linux/amd64 --rm -v $(pwd):/work \
    --entrypoint /bin/bash python:3.11          \
<<EOF
    set -e
    cd /work/target/wheels/
    ls . | grep 311 | grep manylinux | xargs pip install
    cd ../../jyafn-python
    python tests/serde.py
EOF

docker run -i                                   \
    --platform linux/amd64 --rm -v $(pwd):/work \
    --entrypoint /bin/bash python:3.12          \
<<EOF
    set -e
    cd /work/target/wheels/
    ls . | grep 312 | grep manylinux | xargs pip install
    cd ../../jyafn-python
    python tests/serde.py
EOF
