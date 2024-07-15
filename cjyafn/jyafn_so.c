/*
 * One simple example of the use of cjyafn in C directly, to debug the interface between
 * C and Go. To run, do:
 * ```
 * gcc jyafn_so.c && ./a.out
 * ```
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <dlfcn.h>

#define FUNC_PATH "jyafn-go/pkg/jyafn/testdata/simple-ttl.jyafn"
#define JSON "{\"virtual_provider_code\":\"BKX\",\"is_available\":false,\"day_distance\":1234}"
// #define JSON "{\"virtual_provider_code\":\"BKS\",\"is_available\":false,\"day_distance\":1234}"

typedef struct {
    char* buffer;
    long length;
} READ;

READ read_file() {
    char* buffer = 0;
    long length;
    READ contents;
    FILE * f = fopen(FUNC_PATH, "rb");

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

int main(int argc, char** argv) {
    // Load shared object:
    void* handle;
    void* (*function_eval_json)(void*, char*);
    void* (*function_load)(char*, size_t);
    bool (*outcome_is_ok)(void*);
    void* (*outcome_consume_ok)(void*);
    char* (*outcome_consume_err)(void*);

    handle = dlopen("/usr/local/lib/libcjyafn.so", RTLD_LAZY);
    if (!handle) {
        printf("so not found");
        return 1;
    }

    *(void**)(&function_eval_json) = dlsym(handle, "function_eval_json");
    if (!function_eval_json) {
        printf("function_eval_json not found");
        return 1;
    }

    *(void**)(&function_load) = dlsym(handle, "function_load");
    if (!function_load) {
        printf("function_load not found");
        return 1;
    }

    *(void**)(&outcome_is_ok) = dlsym(handle, "outcome_is_ok");
    if (!outcome_is_ok) {
        printf("outcome_is_ok not found");
        return 1;
    }

    *(void**)(&outcome_consume_ok) = dlsym(handle, "outcome_consume_ok");
    if (!outcome_consume_ok) {
        printf("outcome_is_ok not found");
        return 1;
    }

    *(void**)(&outcome_consume_err) = dlsym(handle, "outcome_consume_err");
    if (!outcome_consume_err) {
        printf("outcome_consume_err not found");
        return 1;
    }

    // Load file:

    READ contents = read_file();
    if (contents.buffer == 0) {
        printf("failed to read file\n");
        return 1;
    }

    void* outcome = function_load(contents.buffer, contents.length);
    if (!outcome_is_ok(outcome)) {
        char* err =(char*)(outcome_consume_err(outcome));
        printf("error loading function: %s", err);
        return 1;
    }

    void* func = outcome_consume_ok(outcome);
    
    outcome = function_eval_json(func, JSON);
    if (!outcome_is_ok(outcome)) {
        char* err =(char*)(outcome_consume_err(outcome));
        printf("error calling function: %s", err);
        return 1;
    }

    char** result = (char**)(outcome_consume_ok(outcome));
    printf("%s", *result);

    return 0;
}
