package jyafn

// #cgo CFLAGS: -I../../../cjyafn
// #cgo LDFLAGS: -L../../../target/release -lcjyafn
// #include "cjyafn.h"
//
import "C"

import (
	"fmt"
	"math"
	"reflect"
	"runtime"
	"unsafe"
)

type Outcome C.Outcome

func (o Outcome) get() (unsafe.Pointer, error) {
	if uintptr(o.err) != 0 {
		defer C.free(o.err)
		str := C.error_to_string(o.err)
		return o.ok, fmt.Errorf("%s", C.GoString(str))
	} else {
		return o.ok, nil
	}
}

type Graph struct {
	ptr unsafe.Pointer
	// This prevents the GC from cleaning the owned object.
	ownedBy interface{}
}

func graphFromRaw(ptr unsafe.Pointer) *Graph {
	graph := &Graph{ptr: ptr}
	runtime.SetFinalizer(graph, func(g *Graph) { C.free(g.ptr) })
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

func (g *Graph) ToJSON() string {
	json := C.graph_to_json(g.ptr)
	return C.GoString(json)
}

func (g *Graph) Render() string {
	rendered := C.graph_render(g.ptr)
	return C.GoString(rendered)
}

func (g *Graph) Compile() (*Function, error) {
	out, err := Outcome(C.graph_compile(g.ptr)).get()
	if err != nil {
		return nil, err
	}
	return functionFromRaw(out), nil
}

type Function struct {
	ptr unsafe.Pointer
}

func functionFromRaw(ptr unsafe.Pointer) *Function {
	function := &Function{ptr: ptr}
	runtime.SetFinalizer(function, func(f *Function) { C.free(f.ptr) })
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

type Layout struct {
	ptr unsafe.Pointer
	// This prevents the GC from cleaning the owned object.
	ownedBy interface{}
}

func (l *Layout) ToJSON() string {
	json := C.layout_to_json(l.ptr)
	return C.GoString(json)
}

func (l *Layout) IsUnit() bool {
	return bool(C.layout_is_unit(l.ptr))
}

func (l *Layout) IsScalar() bool {
	return bool(C.layout_is_scalar(l.ptr))
}

func (l *Layout) IsStruct() bool {
	return bool(C.layout_is_struct(l.ptr))
}

func (l *Layout) IsEnum() bool {
	return bool(C.layout_is_enum(l.ptr))
}

func (l *Layout) IsList() bool {
	return bool(C.layout_is_list(l.ptr))
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
	return C.GoString(ptr)
}

func (s *Struct) GetItemLayout(index uint) *Layout {
	ptr := C.strct_get_item_layout(s.ptr, C.ulong(index))
	if uintptr(ptr) == 0 {
		panic("called GetItemLayout on a Struct out of bounds")
	}
	return &Layout{ptr: ptr, ownedBy: s.ownedBy}
}

type Visitor struct {
	ptr     unsafe.Pointer
	ownedBy interface{}
}

func (v *Visitor) Push(value float64) {
	C.visitor_push(v.ptr, C.double(value))
}

func (v *Visitor) Pop() float64 {
	return float64(C.visitor_pop(v.ptr))
}

type GoVisitor struct {
	buf []uint64
}

func (v *GoVisitor) Push(value float64) {
	v.buf = append(v.buf, math.Float64bits(value))
}

func (v *GoVisitor) Pop() float64 {
	top := v.buf[len(v.buf)-1]
	v.buf = v.buf[:len(v.buf)-1]
	return math.Float64frombits(top)
}

func encodeValue(value reflect.Value, layout *Layout, visitor *GoVisitor) error {
	fmt.Println(value)
	fmt.Println(layout.ToJSON())
	if layout.IsUnit() {
		return nil
	}

	kind := value.Kind()

	if (kind == reflect.Float32 || kind == reflect.Float64) && layout.IsScalar() {
		visitor.Push(value.Float())
		return nil
	}

	if (kind == reflect.Struct) && layout.IsStruct() {
		strct := layout.AsStruct()
		ty := value.Type()

		// You can try and cache this calculation...
		for i := uint(0); i < strct.Size(); i++ {
			fieldName := strct.GetItemName(i)
			fieldLayout := strct.GetItemLayout(i)

			found := false
			for j := 0; j < ty.NumField(); j++ {
				field := ty.Field(j)
				tag := field.Tag.Get("jyafn")
				if tag == fieldName || field.Name == fieldName {
					err := encodeValue(value.Field(j), fieldLayout, visitor)
					if err != nil {
						return err
					}
					found = true
				}
			}

			if !found {
				return fmt.Errorf("missing field (or tag) %v in type %v", fieldName, ty)
			}
		}

		return nil
	}

	if layout.IsEnum() {
		panic("unimplemented")
	}

	if (kind == reflect.Slice) && layout.IsList() {
		element := layout.ListElement()
		size := layout.ListSize()

		if size != uint(value.Len()) {
			return fmt.Errorf("layout expected size %v, got %v", size, value.Len())
		}

		for i := 0; i < value.Len(); i++ {
			err := encodeValue(value.Index(i), element, visitor)
			if err != nil {
				return err
			}
		}

		return nil
	}

	return fmt.Errorf("no layout rules to match %v to %v", value.Type(), layout)
}

func decodeValue(ty reflect.Type, layout *Layout, visitor *GoVisitor) reflect.Value {
	if layout.IsUnit() {
		return reflect.New(ty)
	}

	kind := ty.Kind()

	if layout.IsScalar() && kind == reflect.Float64 {
		return reflect.ValueOf(visitor.Pop())
	}

	if layout.IsStruct() {
		strct := layout.AsStruct()
		obj := reflect.New(ty)
		for i := uint(0); i < strct.Size(); i++ {
			fieldName := strct.GetItemName(i)
			fieldLayout := strct.GetItemLayout(i)

			for j := 0; j < ty.NumField(); j++ {
				field := ty.Field(j)
				tag := field.Tag.Get("jyafn")
				if tag == fieldName || field.Name == fieldName {
					obj.Field(j).Set(decodeValue(field.Type, fieldLayout, visitor))
				}
			}
		}

		return obj
	}

	if layout.IsEnum() {
		panic("unimplemented")
	}

	if layout.IsList() {
		slice := reflect.MakeSlice(ty, 0, int(layout.ListSize()))
		for i := 0; i < int(layout.ListSize()); i++ {
			slice = reflect.Append(slice, decodeValue(ty.Elem(), layout.ListElement(), visitor))
		}

		return slice
	}

	panic(fmt.Errorf("could not decode value of type %v and layout %v", ty, layout))
}

func Call[O any](f *Function, arg any) (O, error) {
	visitor := &GoVisitor{buf: make([]uint64, 0)}
	err := encodeValue(reflect.ValueOf(arg), f.InputLayout(), visitor)

	if err != nil {
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf(
				"failed to encode %v to layout %v: %v",
				reflect.ValueOf(arg),
				f.InputLayout().ToJSON(),
				err,
			)
	}

	out := make([]uint64, f.OutputSize()/8, f.OutputSize()/8)
	status := C.function_call_raw(
		f.ptr,
		(*C.uchar)(unsafe.Pointer(&visitor.buf[0])),
		(*C.uchar)(unsafe.Pointer(&out[0])),
	)
	if status != 0 {
		return *reflect.New(reflect.TypeFor[O]()).Interface().(*O),
			fmt.Errorf("function raised status %v", status)
	}

	decoded := decodeValue(reflect.TypeFor[O](), f.OutputLayout(), &GoVisitor{buf: out})
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
	inBytes := []byte(in)
	out, err := Outcome(C.function_eval_json(
		f.ptr,
		(*C.char)(unsafe.Pointer(&inBytes[0])),
	)).get()
	if err != nil {
		return "", err
	}

	return C.GoString((*C.char)(out)), nil
}
