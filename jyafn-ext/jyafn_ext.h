/*
 *  This is a tentative implementation of jyafn extensions in pure C, for the purists. 
 */

#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>


#define QUOTE(...) #__VA_ARGS__


typedef const void* Outcome;

extern const char* outcome_get_err(Outcome);
extern void* outcome_get_ok(Outcome);
extern void outcome_drop(Outcome);

#define OUTCOME_OF(T) Outcome
#define OUTCOME_MANIFEST QUOTE({        \
    "fn_get_err": "outcome_get_err",    \
    "fn_get_ok": "outcome_get_ok",      \
    "fn_drop": "outcome_drop"           \
})


typedef const void* Dumped;

extern size_t dumped_get_len(Dumped);
extern const unsigned char* dumped_get_ptr(Dumped);
extern void dumped_drop(Dumped);

#define DUMPED_MANIFEST   QUOTE({   \
    "fn_get_len": "dumped_get_len", \
    "fn_get_ptr": "dumped_get_ptr", \
    "fn_drop": "dumped_drop"        \
})


extern void string_drop(char*);

#define STRING_MANIFEST   QUOTE({   \
    "fn_drop": "string_drop"        \
})


typedef const void* RawResource;

typedef OUTCOME_OF(RawResource) (*FnFromBytes)(const unsigned char*, size_t);
typedef OUTCOME_OF(Dumped) (*FnDump)(RawResource);
typedef size_t (*FnSize)(RawResource);
typedef char* (*FnGetMethodDef)(RawResource, const char*);
typedef void (*FnDrop)(RawResource);

#define DEF_SYMBOL_T(FN_TY) typedef struct { FN_TY fn_ptr; char* name; } Symbol##FN_TY
#define SYMBOL_T(FN_TY) Symbol##FN_TY
#define SYMBOL(FUNC) { fn_ptr: &FUNC, name: #FUNC }
DEF_SYMBOL_T(FnFromBytes);
DEF_SYMBOL_T(FnDump);
DEF_SYMBOL_T(FnSize);
DEF_SYMBOL_T(FnGetMethodDef);
DEF_SYMBOL_T(FnDrop);


#define MANIFEST_BEGIN "{"          \
    "\"outcome\": "OUTCOME_MANIFEST \
    ", \"dumped\": "DUMPED_MANIFEST \
    ", \"string\": "STRING_MANIFEST \
    ", \"resources\": {"
#define MANIFEST_END "}}"

typedef char* DeclaredResource;

DeclaredResource declare_resource(
    char* resource_name,
    SYMBOL_T(FnFromBytes) fn_from_bytes,
    SYMBOL_T(FnDump) fn_dump,
    SYMBOL_T(FnSize) fn_size,
    SYMBOL_T(FnGetMethodDef) fn_get_method_def,
    SYMBOL_T(FnDrop) fn_drop
) {
    const char* fmt_entry = "\"%s\": " QUOTE({
        "fn_from_bytes": %s,
        "fn_dump": %s,
        "fn_size": %s,
        "fn_get_method_def": %s,
        "fn_drop": %s,
    });

    size_t needed = snprintf(
        NULL,
        0,
        fmt_entry,
        resource_name,
        fn_from_bytes,
        fn_dump,
        fn_size,
        fn_get_method_def,
        fn_drop
    ) + 1;
    char  *buffer = malloc(needed);
    sprintf(
        buffer,
        fmt_entry,
        resource_name,
        fn_from_bytes,
        fn_dump,
        fn_size,
        fn_get_method_def,
        fn_drop
    );

    return (DeclaredResource)buffer;
}

void joinstr(char** buf, size_t* cap, size_t* len, char* src) {
    if (*buf == NULL) {
        *buf = malloc(10);
        *cap = 10;
        *len = 0;
    }

    size_t i = 0;
    while (src[i] != '\0') {
        if (*len == *cap) {
            *cap *= 2;
            char* newbuf = malloc(*cap);
            memcpy(newbuf, *buf, *len);
            free(*buf);
            *buf = newbuf;
        }

        (*buf)[*len] = src[i];
        *len += 1;
    }
}

char* build_manifest(DeclaredResource* resources, size_t n_resources) {
    char* buf = NULL;
    size_t cap = 0;
    size_t len = 0;

    joinstr(&buf, &cap, &len, &*(MANIFEST_BEGIN));

    for (size_t i = 0; i < n_resources - 1; i++) {
        joinstr(&buf, &cap, &len, resources[i]);
        free(resources[i]);
        joinstr(&buf, &cap, &len, ", ");
    }

    if (n_resources > 0) {
        joinstr(&buf, &cap, &len, resources[n_resources - 1]);
    }

    joinstr(&buf, &cap, &len, MANIFEST_END);
    joinstr(&buf, &cap, &len, "\0");

    return buf;
}
