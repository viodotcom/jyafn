package jyafn

// #cgo CFLAGS: -I./
// #cgo LDFLAGS: -L./ -lcjyafn
// #include "cjyafn.h"
//
import "C"

import (
	"fmt"
	"math"
	"reflect"
	"unsafe"
)

type Layout struct {
	ptr unsafe.Pointer
	// This prevents the GC from cleaning the owned object.
	ownedBy  any
	isClosed bool
}

func (l *Layout) Close() {
	if l.ownedBy == nil && !l.isClosed {
		C.layout_drop(l.ptr)
		l.isClosed = true
	}
}

func (l *Layout) panicOnClosed() {
	if l.isClosed {
		panic(fmt.Sprintf("layout %+v was already closed", l))
	}
}

func (l *Layout) ToString() string {
	l.panicOnClosed()
	str := C.layout_to_string(l.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(str))
	return C.GoString(str)
}

func (l *Layout) MarshalJSON() ([]byte, error) {
	l.panicOnClosed()
	json := C.layout_to_json(l.ptr)
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(json))
	return []byte(C.GoString(json)), nil
}

func (l *Layout) UnmarshalJSON(json []byte) error {
	l.panicOnClosed()
	fmt.Println(string(json), "xxx")

	value, err := Outcome(C.layout_from_json((*C.char)(unsafe.Pointer(&json[0])))).get()
	fmt.Println(string(json), "xxx")

	if err != nil {
		return err
	}

	fmt.Println(string(json), "xxx")

	l.ptr = value

	return nil
}

func (l *Layout) IsUnit() bool {
	l.panicOnClosed()
	return bool(C.layout_is_unit(l.ptr))
}

func (l *Layout) IsScalar() bool {
	l.panicOnClosed()
	return bool(C.layout_is_scalar(l.ptr))
}

func (l *Layout) IsBool() bool {
	l.panicOnClosed()
	return bool(C.layout_is_bool(l.ptr))
}

func (l *Layout) IsDateTime() bool {
	l.panicOnClosed()
	return bool(C.layout_is_datetime(l.ptr))
}

func (l *Layout) IsSymbol() bool {
	l.panicOnClosed()
	return bool(C.layout_is_symbol(l.ptr))
}

func (l *Layout) IsStruct() bool {
	l.panicOnClosed()
	return bool(C.layout_is_struct(l.ptr))
}

func (l *Layout) IsList() bool {
	l.panicOnClosed()
	return bool(C.layout_is_list(l.ptr))
}

func (l *Layout) DateTimeFormat() string {
	l.panicOnClosed()
	ptr := C.layout_datetime_format(l.ptr)
	if uintptr(unsafe.Pointer(ptr)) == 0 {
		panic("called DateTimeFormat on a Layout that is not a datatime")
	}
	// This is a C string. So, free works.
	defer C.free(unsafe.Pointer(ptr))
	return C.GoString(ptr)
}

func (l *Layout) AsStruct() *Struct {
	l.panicOnClosed()
	ptr := C.layout_as_struct(l.ptr)
	if uintptr(ptr) == 0 {
		panic("called AsStruct on a Layout that is not a struct")
	}
	return &Struct{ptr: ptr, ownedBy: l.ownedBy}
}

func (l *Layout) ListElement() *Layout {
	l.panicOnClosed()
	ptr := C.layout_list_element(l.ptr)
	if uintptr(ptr) == 0 {
		panic("called ListElement on a Layout that is not a list")
	}
	return &Layout{ptr: ptr, ownedBy: l.ownedBy}
}

func (l *Layout) ListSize() uint {
	l.panicOnClosed()
	return uint(C.layout_list_size(l.ptr))
}

func (l *Layout) IsSuperset(other *Layout) bool {
	l.panicOnClosed()
	return bool(C.layout_is_superset(l.ptr, other.ptr))
}

type Struct struct {
	ptr     unsafe.Pointer
	ownedBy any
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

type Symbols struct {
	top []string
	new []string
}

func NewSymbolView(top []string) Symbols {
	return Symbols{top: top}
}

func (s *Symbols) Find(name string) int {
	for i := range s.top {
		if s.top[i] == name {
			return i
		}
	}

	for i := range s.new {
		if s.top[i] == name {
			return i + len(s.top)
		}
	}

	id := len(s.new)
	s.new = append(s.new, name)

	return id + len(s.top)
}

func (s *Symbols) Get(id int) (string, error) {
	if id < len(s.top) {
		return s.top[id], nil
	}

	if id-len(s.top) < len(s.new) {
		return s.new[id-len(s.top)], nil
	}

	return "", fmt.Errorf("not found")
}

type Visitor struct {
	buf []uint64
}

func (v *Visitor) Push(value float64) {
	v.buf = append(v.buf, math.Float64bits(value))
}

func (v *Visitor) PushInt(value int) {
	// What about negative values? I think it's all right.
	v.buf = append(v.buf, uint64(value))
}

func (v *Visitor) Pop() float64 {
	top := v.buf[len(v.buf)-1]
	v.buf = v.buf[:len(v.buf)-1]
	return math.Float64frombits(top)
}

func (v *Visitor) PopInt() int {
	top := v.buf[len(v.buf)-1]
	v.buf = v.buf[:len(v.buf)-1]
	return int(top)
}

func encodeValue(value reflect.Value, layout *Layout, symbols *Symbols, visitor *Visitor) error {
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
					err := encodeValue(value.Field(j), fieldLayout, symbols, visitor)
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

	if (kind == reflect.String) && layout.IsSymbol() {
		id := symbols.Find(value.String())
		visitor.PushInt(id)
	}

	if (kind == reflect.String) && layout.IsDateTime() {
		timestamp, err := ParseDateTime(value.String(), layout.DateTimeFormat())
		if err != nil {
			return err
		}
		visitor.PushInt(int(timestamp))
	}

	if (kind == reflect.Slice) && layout.IsList() {
		element := layout.ListElement()
		size := layout.ListSize()

		if size != uint(value.Len()) {
			return fmt.Errorf("layout expected size %v, got %v", size, value.Len())
		}

		for i := 0; i < value.Len(); i++ {
			err := encodeValue(value.Index(i), element, symbols, visitor)
			if err != nil {
				return err
			}
		}

		return nil
	}

	return fmt.Errorf("no layout rules to match %v to %v", value.Type(), layout)
}

func decodeValue(ty reflect.Type, layout *Layout, symbols *Symbols, visitor *Visitor) reflect.Value {
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
					obj.Field(j).Set(decodeValue(field.Type, fieldLayout, symbols, visitor))
				}
			}
		}

		return obj
	}

	if (kind == reflect.String) && layout.IsDateTime() {
		timestamp := visitor.PopInt()
		return reflect.ValueOf(FormatDateTime(int64(timestamp), layout.DateTimeFormat()))
	}

	if layout.IsSymbol() && kind == reflect.String {
		symbol, err := symbols.Get(visitor.PopInt())
		if err != nil {
			panic(err)
		}
		return reflect.ValueOf(symbol)
	}

	if layout.IsList() {
		slice := reflect.MakeSlice(ty, 0, int(layout.ListSize()))
		for i := 0; i < int(layout.ListSize()); i++ {
			slice = reflect.Append(
				slice,
				decodeValue(ty.Elem(), layout.ListElement(), symbols, visitor),
			)
		}

		return slice
	}

	panic(fmt.Errorf("could not decode value of type %v and layout %v", ty, layout))
}
