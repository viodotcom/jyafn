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
	EXT := so
	LIBPATH := $(ROOT)/target/release/lib$(LIBNAME).$(EXT)
else ifeq ($(UNAME), Darwin)
	EXT := dylib
	LIBPATH := $(ROOT)/target/release/lib$(LIBNAME).$(EXT)
endif

build:
	cargo build --release
	mkdir -p $(ROOT)/target/extensions/
	cp $(LIBPATH) $(ROOT)/target/extensions/$(NAME).$(EXT)

install: build
	mkdir -p ~/.jyafn/extensions/
	cp $(ROOT)/target/extensions/$(NAME).$(EXT) ~/.jyafn/extensions/$(NAME)-$(VERSION).$(EXT)
