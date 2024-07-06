package jyafn

// #cgo CFLAGS: -I./
// #cgo LDFLAGS: -L./ -lcjyafn
// #include "cjyafn.h"
//
import "C"

import (
	"fmt"
	"unsafe"
)

type Graph struct {
	wasClosed bool
	ptr       unsafe.Pointer
	// This prevents the GC from cleaning the owned object.
	ownedBy any
}

func graphFromRaw(ptr unsafe.Pointer) *Graph {
	graph := &Graph{ptr: ptr}
	return graph
}

func LoadGraph(encoded []byte) (*Graph, error) {
	codePtr := unsafe.Pointer(&encoded[0])
	ptr, err := Outcome(
		C.graph_load((*C.uchar)(codePtr), (C.ulong)(len(encoded))),
	).get()
	if err != nil {
		return nil, err
	}

	return graphFromRaw(ptr), nil
}

func (g *Graph) Close() {
	if !g.wasClosed {
		C.graph_drop(g.ptr)
		g.wasClosed = true
	}
}

func (g *Graph) panicOnClosed() {
	if g.wasClosed {
		panic(fmt.Sprintf("graph %+v was already closed", g))
	}
}

func (g *Graph) Name() string {
	g.panicOnClosed()
	name := C.graph_name(g.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(name))
	return C.GoString(name)
}

func (g *Graph) GetMetadata(key string) string {
	g.panicOnClosed()
	keyBytes := []byte(key)
	value := C.graph_get_metadata(g.ptr, (*C.char)(unsafe.Pointer(&keyBytes[0])))
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (g *Graph) GetMetadataJSON() string {
	g.panicOnClosed()
	value := C.graph_get_metadata_json(g.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (g *Graph) ToJSON() string {
	g.panicOnClosed()
	json := C.graph_to_json(g.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(json))
	return C.GoString(json)
}

func (g *Graph) Render() (string, error) {
	g.panicOnClosed()
	rendered, err := Outcome(C.graph_render(g.ptr)).get()
	if err != nil {
		return "", err
	}
	// This is a C string. So, free works.
	defer C.free(rendered)

	return C.GoString((*C.char)(rendered)), nil
}

func (g *Graph) Compile() (*Function, error) {
	g.panicOnClosed()
	out, err := Outcome(C.graph_compile(g.ptr)).get()
	if err != nil {
		return nil, err
	}
	return functionFromRaw(out), nil
}
