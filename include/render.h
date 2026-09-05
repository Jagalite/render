/* Generated from crates/ffi/src/lib.rs. Do not edit. */
#ifndef RENDER_ABI_ALPHA_H
#define RENDER_ABI_ALPHA_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct RenderApi {
    uint32_t struct_size;
    uint32_t abi_version;
    uint64_t (*create)(uint64_t, uint64_t);
    int32_t (*destroy)(uint64_t);
    uint64_t (*request)(uint64_t, const uint8_t *, uint64_t);
    uint64_t (*response_size)(uint64_t);
    int32_t (*response_read)(uint64_t, uint8_t *, uint64_t);
    int32_t (*release)(uint64_t);
} RenderApi;
const RenderApi *render_entry(uint32_t version, uint32_t minimum_size);
#ifdef __cplusplus
} /* extern C */
#endif
#endif
