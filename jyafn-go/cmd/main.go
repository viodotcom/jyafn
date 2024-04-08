package main

import (
	"fmt"
	"log"
	"os"

	"github.com/FindHotel/jyafn/jyafn-go/pkg/jyafn"
)

func main() {
	// Read exported data:
	code, err := os.ReadFile("../jyafn-python/a_fun.jyafn")
	if err != nil {
		log.Fatal(err)
	}

	// Load the function:
	fn, err := jyafn.LoadFunction(code)
	if err != nil {
		log.Fatal(err)
	}

	// Call the function:
	result, err := jyafn.Call[float64](
		fn,
		struct {
			a float64
			b float64
		}{a: 2.0, b: 3.0},
	)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println(result, "==", 8.0)
}
