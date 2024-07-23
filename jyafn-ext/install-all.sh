#! /usr/bin/env bash

##
# Builds and installs all extensions in the `extensions` folder
##

set -e

BASEDIR=$PWD

for extension in ./extensions/*; do
    if test -d $extension; then
        extensions="$extensions $extension"
    fi
done

for extension in $extensions; do
    echo Building $extension...
    cd $extension
    make install
    cd $BASEDIR
done
