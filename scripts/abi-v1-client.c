/* Independent client conformance tooling, not product computation. */
#include "render.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    assert(render_entry(2, 0) == NULL);
    assert(render_entry(1, UINT32_MAX) == NULL);
    const RenderApi *api = render_entry(1, sizeof(RenderApi));
    assert(api && api->abi_version == 1 && api->struct_size == sizeof(RenderApi));
    assert(render_entry(1, 8) == api);
    assert(render_entry(0, sizeof(RenderApi))->abi_version == 0);
    uint64_t engine = api->create(0, 12);
    assert(engine);
    const char *query = "{\"method\":\"inspect\"}";
    uint64_t response = api->request(engine, (const uint8_t *)query, strlen(query));
    uint64_t size = api->response_size(response);
    assert(size);
    uint8_t *out = calloc((size_t)size + 1, 1);
    assert(out);
    assert(api->response_read(response, out, size - 1) == -2);
    assert(api->response_read(response, out, size) == 0);
    assert(strstr((char *)out, "\"Ok\"") && strstr((char *)out, "\"revision\""));
    assert(api->destroy(response) == -1);
    assert(api->release(engine) == -1);
    assert(api->destroy(engine) == 0);
    /* A response owns its bytes independently of its originating engine. */
    assert(api->response_read(response, out, size) == 0);
    assert(api->release(response) == 0);
    assert(api->release(response) == -1);
    assert(api->response_size(response) == 0);
    assert(api->destroy(engine) == -1);
    uint64_t replacement = api->create(0, 12);
    assert(replacement && replacement != engine);
    assert(api->destroy(replacement) == 0);
    free(out);
    puts("{\"status\":\"passed\",\"client\":\"C11\",\"abi_version\":1,\"checks\":[\"version and prefix negotiation\",\"wrong handle kind\",\"capacity\",\"buffer outlives engine\",\"stale generation\"]}");
    return 0;
}
