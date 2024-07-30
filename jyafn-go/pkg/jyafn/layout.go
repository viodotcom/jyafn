package jyafn

import (
	"fmt"
	"math"
	"reflect"
)

type OwnedLayout struct {
	ptr      OwnedLayoutPtr
	isClosed bool
}

func (l *OwnedLayout) Close() {
	if !l.isClosed {
		ffi.layoutDrop(l.ptr)
		l.isClosed = true
	}
}

func (l *OwnedLayout) panicOnClosed() {
	if l.isClosed {
		panic(fmt.Sprintf("layout %+v was already closed", l))
	}
}

func (l *OwnedLayout) GetRef() Layout {
	l.panicOnClosed()
	return Layout{ptr: l.ptr.getRef()}
}

func LayoutFromJSON(json string) (*OwnedLayout, error) {
	ptr, err := ffi.layoutFromJson(json).get()
	if err != nil {
		return &OwnedLayout{}, nil
	}

	return &OwnedLayout{ptr: OwnedLayoutPtr(ptr), isClosed: false}, nil
}

func (l *OwnedLayout) MarshalJSON() ([]byte, error) {
	return l.GetRef().MarshalJSON()
}

func (l *OwnedLayout) UnmarshalJSON(json []byte) error {
	if l.ptr != 0 {
		return fmt.Errorf("cannot unmarshal layout on already-filled layout")
	}

	ptr, err := ffi.layoutFromJson(string(json)).get()
	if err != nil {
		return err
	}

	l.ptr = OwnedLayoutPtr(ptr)
	l.isClosed = false // for good measure
	return nil
}

type Layout struct {
	ptr LayoutPtr
}

func (l Layout) Clone() *OwnedLayout {
	return &OwnedLayout{ptr: ffi.layoutClone(l.ptr), isClosed: false}
}

func (l Layout) ToString() string {
	value := ffi.layoutToString(l.ptr)
	defer ffi.freeStr(value)
	return ffi.transmuteAsStr(value)
}

func (l Layout) MarshalJSON() ([]byte, error) {
	value := ffi.layoutToJson(l.ptr)
	defer ffi.freeStr(value)
	return []byte(ffi.transmuteAsStr(value)), nil
}

func (l Layout) IsUnit() bool {
	return ffi.layoutIsUnit(l.ptr)
}

func (l Layout) IsScalar() bool {
	return ffi.layoutIsScalar(l.ptr)
}

func (l Layout) IsBool() bool {
	return ffi.layoutIsBool(l.ptr)
}

func (l Layout) IsDateTime() bool {
	return ffi.layoutIsDatetime(l.ptr)
}

func (l Layout) IsSymbol() bool {
	return ffi.layoutIsSymbol(l.ptr)
}

func (l Layout) IsStruct() bool {
	return ffi.layoutIsStruct(l.ptr)
}

func (l Layout) IsTuple() bool {
	return ffi.layoutIsTuple(l.ptr)
}

func (l Layout) IsList() bool {
	return ffi.layoutIsList(l.ptr)
}

func (l Layout) AsStruct() Struct {
	return Struct{ptr: ffi.layoutAsStruct(l.ptr)}
}

func (l Layout) TupleSize() uint {
	return uint(ffi.layoutTupleSize(l.ptr))
}

func (l Layout) GetItemLayout() Layout {
	item := ffi.layoutGetTupleItem(l.ptr)
	if item == 0 {
		panic("called GetItemLayout on a Tuple out of bounds")
	}
	return Layout{ptr: item}
}

func (l Layout) DateTimeFormat() string {
	format := ffi.layoutDatetimeFormat(l.ptr)
	if format == 0 {
		panic("called DateTimeFormat on a Layout that is not a datatime")
	}
	defer ffi.freeStr(format)
	return ffi.transmuteAsStr(format)
}

func (l Layout) ListElement() Layout {
	ptr := ffi.layoutListElement(l.ptr)
	if uintptr(ptr) == 0 {
		panic("called ListElement on a Layout that is not a list")
	}
	return Layout{ptr: ptr}
}

func (l Layout) ListSize() uint {
	return uint(ffi.layoutListSize(l.ptr))
}

func (l Layout) IsSuperset(other Layout) bool {
	return bool(ffi.layoutIsSuperset(l.ptr, other.ptr))
}

type Struct struct {
	ptr StructPtr
}

func (s Struct) Size() uint {
	return uint(ffi.strctSize(s.ptr))
}

func (s Struct) GetItemName(index uint) string {
	ptr := ffi.strctGetItemName(s.ptr, uintptr(index))
	if ptr == 0 {
		panic("called GetItemName on a Struct out of bounds")
	}
	defer ffi.freeStr(ptr)
	return ffi.transmuteAsStr(ptr)
}

func (s Struct) GetItemLayout(index uint) Layout {
	ptr := ffi.strctGetItemLayout(s.ptr, uintptr(index))
	if uintptr(ptr) == 0 {
		panic("called GetItemLayout on a Struct out of bounds")
	}
	return Layout{ptr: ptr}
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

func encodeValue(value reflect.Value, layout Layout, symbols *Symbols, visitor *Visitor) error {
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

func decodeValue(ty reflect.Type, layout Layout, symbols *Symbols, visitor *Visitor) reflect.Value {
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
		formated, err := FormatDateTime(int64(timestamp), layout.DateTimeFormat())
		if err != nil {
			panic(err)
		}
		return reflect.ValueOf(formated)
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
