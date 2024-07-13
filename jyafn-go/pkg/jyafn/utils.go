package jyafn

func ParseDateTime(s string, fmt string) (int64, error) {
	ptr, err := ffi.parseDatetime(s, fmt).get()
	if err != nil {
		return 0, err
	}

	return ffi.consumeI64Ptr(ptr), nil
}

func FormatDateTime(t int64, fmt string) (string, error) {
	s, err := ffi.formatDatetime(t, fmt).getPtr()
	if err != nil {
		return "", err
	}
	defer ffi.freeStr(AllocatedStr(s))
	return ffi.transmuteAsStr(AllocatedStr(s)), nil
}
