package jyafn

import "fmt"

type Graph struct {
	ptr      GraphPtr
	isClosed bool
}

func LoadGraph(encoded []byte) (*Graph, error) {
	ptr, err := ffi.graphLoad(encoded, uintptr(len(encoded))).get()
	if err != nil {
		return nil, err
	}

	return &Graph{ptr: GraphPtr(ptr), isClosed: false}, nil
}

func (g *Graph) Close() {
	if !g.isClosed {
		ffi.graphDrop(g.ptr)
		g.isClosed = true
	}
}

func (g *Graph) panicOnClosed() {
	if g.isClosed {
		panic(fmt.Sprintf("graph %+v was already closed", g))
	}
}

func (g *Graph) Name() string {
	g.panicOnClosed()
	name := ffi.graphName(g.ptr)
	defer ffi.freeStr(name)
	return ffi.transmuteAsStr(name)
}

func (g *Graph) GetMetadata(key string) string {
	g.panicOnClosed()
	value := ffi.graphGetMetadata(g.ptr, key)
	if value == 0 {
		return ""
	}
	defer ffi.freeStr(value)
	return ffi.transmuteAsStr(value)
}

func (g *Graph) GetMetadataJSON() string {
	g.panicOnClosed()
	value := ffi.graphGetMetadataJson(g.ptr)
	defer ffi.freeStr(value)
	return ffi.transmuteAsStr(value)
}

func (g *Graph) ToJSON() string {
	g.panicOnClosed()
	value := ffi.graphToJson(g.ptr)
	defer ffi.freeStr(value)
	return ffi.transmuteAsStr(value)
}

func (g *Graph) Render() string {
	g.panicOnClosed()
	rendered := ffi.graphRender(g.ptr)
	defer ffi.freeStr(rendered)
	return ffi.transmuteAsStr(rendered)
}

func (g *Graph) Compile() (*Function, error) {
	g.panicOnClosed()
	ptr, err := ffi.graphCompile(g.ptr).get()
	if err != nil {
		return nil, err
	}
	return functionFromRaw(FunctionPtr(ptr)), nil
}
