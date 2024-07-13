/*
 * One simple example of the use of cjyafn in C directly, to debug the interface between
 * C and Go. To run, do:
 * ```
 * gcc jyafn.c ../target/release/libcjyafn.a -lm && ./a.out
 * ```
 */

#include <stdio.h>
#include "cjyafn.h"

typedef struct {
    char* buffer;
    long length;
} READ;

READ read_file() {
    char* buffer = 0;
    long length;
    READ contents;
    FILE * f = fopen("testdata/silly-map.jyafn", "rb");

    if (f) {
        fseek (f, 0, SEEK_END);
        length = ftell (f);
        fseek (f, 0, SEEK_SET);
        buffer = malloc (length);
        if (buffer) {
            fread (buffer, 1, length, f);
        }
        fclose (f);
    }

    contents.buffer = buffer;
    contents.length = length;

    return contents;
}

void main() {
    READ contents = read_file();
    if (contents.buffer == 0) {
        printf("failed to read file\n");
        exit(1);
    }

    Outcome out = function_load(contents.buffer, contents.length);
    if (out.err != 0) {
        printf(error_to_string(out.err));
        // free(out.err);
        exit(1);
    }

    void* func = out.ok;

    out = function_eval_json(func, "{\"a\": 4.0, \"x\": \"a\"}");
    if (out.err != 0) {
        printf(error_to_string(out.err));
        // free(out.err);
        exit(1);
    }

    printf("outcome = %s\n", out.ok);
}
