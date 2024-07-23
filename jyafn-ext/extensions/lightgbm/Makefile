##
#
# You may copy this file around. It should work with all Rust `jyafn-ext` based 
# jyafn extensions.
#
##


VERSION := $(shell cargo read-manifest | jq -r '.version')
NAME := $(shell cargo read-manifest | jq -r '.name' )
LIBNAME := $(shell cargo read-manifest | jq -r '.targets[0].name')
ROOT := $(shell cargo metadata --format-version=1 | jq -r '.workspace_root')
UNAME := $(shell uname)

ifeq ($(UNAME), Linux)
	SO_EXT := so
endif
ifeq ($(UNAME), Darwin)
	SO_EXT := dylib
endif

build:
	cargo build --release

install: build
	mkdir -p ~/.jyafn/extensions/
	mv $(ROOT)/target/release/lib$(LIBNAME).$(SO_EXT) ~/.jyafn/extensions/$(NAME)-$(VERSION).$(SO_EXT)
