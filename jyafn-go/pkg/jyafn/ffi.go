package jyafn

// #cgo CFLAGS: -I./
// #cgo LDFLAGS: -L./ -lcjyafn
// #include "cjyafn.h"
//
import "C"

import (
	"encoding/json"
	"fmt"
	"reflect"
	"runtime"
	"unsafe"
)

type Outcome C.Outcome

func (o Outcome) get() (unsafe.Pointer, error) {
	if uintptr(o.err) != 0 {
		defer C.error_drop(o.err)
		str := C.error_to_string(o.err)
		return o.ok, fmt.Errorf("%s", C.GoString(str))
	} else {
		return o.ok, nil
	}
}

func ParseDateTime(s string, fmt string) (int64, error) {
	sBytes := []byte(s)
	fmtBytes := []byte(fmt)
	timestamp, err := Outcome(C.parse_datetime(
		(*C.char)(unsafe.Pointer(&sBytes[0])),
		(*C.char)(unsafe.Pointer(&fmtBytes[0])),
	)).get()
	if err != nil {
		return 0, err
	}
	// This is a pointer to a single int. So, free works.
	defer C.free(timestamp)

	return int64(*(*C.int64_t)(timestamp)), nil
}

func FormatDateTime(timestamp int64, fmt string) string {
	fmtBytes := []byte(fmt)
	formatted, err := Outcome(C.format_datetime(
		(C.int64_t)(timestamp),
		(*C.char)(unsafe.Pointer(&fmtBytes[0])),
	)).get()
	if err != nil {
		panic(err)
	}
	// This is a pointer to a C string. So, free works.
	defer C.free(formatted)

	return C.GoString((*C.char)(formatted))
}

type Graph struct {
	ptr unsafe.Pointer
	// This prevents the GC from cleaning the owned object.
	ownedBy interface{}
}

func graphFromRaw(ptr unsafe.Pointer) *Graph {
	graph := &Graph{ptr: ptr}
	runtime.SetFinalizer(graph, func(g *Graph) { C.graph_drop(g.ptr) })
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

func (g *Graph) GetMetadata(key string) string {
	keyBytes := []byte(key)
	value := C.graph_get_metadata(g.ptr, (*C.char)(unsafe.Pointer(&keyBytes[0])))
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (g *Graph) GetMetadataJSON() string {
	value := C.graph_get_metadata_json(g.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (g *Graph) ToJSON() string {
	json := C.graph_to_json(g.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(json))
	return C.GoString(json)
}

func (g *Graph) Render() (string, error) {
	rendered, err := Outcome(C.graph_render(g.ptr)).get()
	if err != nil {
		return "", err
	}
	// This is a C string. So, free works.
	defer C.free(rendered)

	return C.GoString((*C.char)(rendered)), nil
}

func (g *Graph) Compile() (*Function, error) {
	out, err := Outcome(C.graph_compile(g.ptr)).get()
	if err != nil {
		return nil, err
	}
	return functionFromRaw(out), nil
}

type Function struct {
	ptr     unsafe.Pointer
	symbols []string
}

func functionFromRaw(ptr unsafe.Pointer) *Function {
	function := &Function{ptr: ptr}
	runtime.SetFinalizer(function, func(f *Function) { C.function_drop(f.ptr) })

	// Read symbols to the Go side.
	symbolsJSONPtr, err := Outcome(C.function_symbols_json(ptr)).get()
	if err != nil {
		panic(err)
	}
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(symbolsJSONPtr))

	err = json.Unmarshal([]byte(C.GoString((*C.char)(symbolsJSONPtr))), &function.symbols)
	if err != nil {
		panic(err)
	}

	return function
}

func LoadFunction(encoded []byte) (*Function, error) {
	codePtr := unsafe.Pointer(&encoded[0])
	ptr, err := Outcome(
		C.function_load((*C.uchar)(codePtr), (C.ulong)(len(encoded))),
	).get()
	if err != nil {
		return nil, err
	}

	return functionFromRaw(ptr), nil
}

func (f *Function) InputSize() uint {
	return uint(C.function_input_size(f.ptr))
}

func (f *Function) OutputSize() uint {
	return uint(C.function_output_size(f.ptr))
}

func (f *Function) InputLayout() *Layout {
	return &Layout{ptr: C.function_input_layout(f.ptr), ownedBy: f}
}

func (f *Function) OutputLayout() *Layout {
	return &Layout{ptr: C.function_output_layout(f.ptr), ownedBy: f}
}

func (f *Function) Graph() *Graph {
	return &Graph{ptr: C.function_graph(f.ptr), ownedBy: f}
}

func (f *Function) GetMetadata(key string) string {
	keyBytes := []byte(key)
	value := C.function_get_metadata(f.ptr, (*C.char)(unsafe.Pointer(&keyBytes[0])))
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (f *Function) GetMetadataJSON() string {
	value := C.function_get_metadata_json(f.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(value))

	return C.GoString(value)
}

func (f *Function) GetSize() int {
	return int(C.function_get_size(f.ptr))
}

type Layout struct {
	ptr unsafe.Pointer
	// This prevents the GC from cleaning the owned object.
	ownedBy interface{}
}

func (l *Layout) ToJSON() string {
	json := C.layout_to_json(l.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(json))
	return C.GoString(json)
}

func (l *Layout) IsUnit() bool {
	return bool(C.layout_is_unit(l.ptr))
}

func (l *Layout) IsScalar() bool {
	return bool(C.layout_is_scalar(l.ptr))
}

func (l *Layout) IsBool() bool {
	return bool(C.layout_is_bool(l.ptr))
}

func (l *Layout) IsDateTime() bool {
	return bool(C.layout_is_datetime(l.ptr))
}

func (l *Layout) IsSymbol() bool {
	return bool(C.layout_is_symbol(l.ptr))
}

func (l *Layout) IsStruct() bool {
	return bool(C.layout_is_struct(l.ptr))
}

func (l *Layout) IsList() bool {
	return bool(C.layout_is_list(l.ptr))
}

func (l *Layout) DateTimeFormat() string {
	ptr := C.layout_datetime_format(l.ptr)
	if uintptr(unsafe.Pointer(ptr)) == 0 {
		panic("called DateTimeFormat on a Layout that is not a datatime")
	}
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(ptr))
	return C.GoString(ptr)
}

func (l *Layout) AsStruct() *Struct {
	ptr := C.layout_as_struct(l.ptr)
	if uintptr(ptr) == 0 {
		panic("called AsStruct on a Layout that is not a struct")
	}
	return &Struct{ptr: ptr, ownedBy: l.ownedBy}
}

func (l *Layout) ListElement() *Layout {
	ptr := C.layout_list_element(l.ptr)
	if uintptr(ptr) == 0 {
		panic("called ListElement on a Layout that is not a list")
	}
	return &Layout{ptr: ptr, ownedBy: l.ownedBy}
}

func (l *Layout) ListSize() uint {
	return uint(C.layout_list_size(l.ptr))
}

type Struct struct {
	ptr     unsafe.Pointer
	ownedBy interface{}
}

func (s *Struct) Size() uint {
	return uint(C.strct_size(s.ptr))
}

func (s *Struct) GetItemName(index uint) string {
	ptr := C.strct_get_item_name(s.ptr, C.ulong(index))
	if uintptr(unsafe.Pointer(ptr)) == 0 {
		panic("called GetItemName on a Struct out of bounds")
	}
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(ptr))
	return C.GoString(ptr)
}

func (s *Struct) GetItemLayout(index uint) *Layout {
	ptr := C.strct_get_item_layout(s.ptr, C.ulong(index))
	if uintptr(ptr) == 0 {
		panic("called GetItemLayout on a Struct out of bounds")
	}
	return &Layout{ptr: ptr, ownedBy: s.ownedBy}
}

func Call[O any](f *Function, arg any) (O, error) {
	visitor := &Visitor{buf: make([]uint64, 0)}
	symbols := &Symbols{top: f.symbols}
	err := encodeValue(reflect.ValueOf(arg), f.InputLayout(), symbols, visitor)

	if err != nil {
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf(
				"failed to encode %v to layout %v: %v",
				reflect.ValueOf(arg),
				f.InputLayout().ToJSON(),
				err,
			)
	}

	out := make([]uint64, f.OutputSize()/8)
	status := C.function_call_raw(
		f.ptr,
		(*C.uchar)(unsafe.Pointer(&visitor.buf[0])),
		(*C.uchar)(unsafe.Pointer(&out[0])),
	)
	if status != nil {
		goStatus := C.GoString(status)
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf("function raised status %v", goStatus)
	}

	decoded := decodeValue(reflect.TypeFor[O](), f.OutputLayout(), symbols, &Visitor{buf: out})
	output, isOk := decoded.Interface().(O)
	if !isOk {
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf(
				"failed to decode %v from layout %v",
				reflect.TypeFor[O](),
				f.OutputLayout().ToJSON(),
			)
	}

	return output, nil
}

func CallJSON(f *Function, in string) (string, error) {
	// This prevents the panic of calling a pointer to the first byte of the slice later.
	if in == "" {
		return "", fmt.Errorf("input to CallJSON cannot be empty")
	}

	inBytes := []byte(in)
	out, err := Outcome(C.function_eval_json(
		f.ptr,
		(*C.char)(unsafe.Pointer(&inBytes[0])),
	)).get()
	if err != nil {
		return "", err
	}
	// This is a C string. So, free works.
	defer C.free(out)

	return C.GoString((*C.char)(out)), nil
}
