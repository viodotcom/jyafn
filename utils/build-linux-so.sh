#! /usr/bin/env bash

set -e

docker run -i                                   \
    --platform linux/amd64 --rm -v $(pwd):/work \
    --entrypoint /bin/bash rust:latest          \
<<EOF
    set -e
    cd /work
    make cjyafn
EOF
