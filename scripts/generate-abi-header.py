"""Generate C interop declarations from the Rust function table."""
from pathlib import Path
import re
import sys
source=Path('crates/ffi/src/lib.rs').read_text()
body=source.split('pub struct ApiTable {',1)[1].split('\n}',1)[0]
types={'u64':'uint64_t','u32':'uint32_t','i32':'int32_t','*const u8':'const uint8_t *','*mut u8':'uint8_t *'}
lines=['/* Generated from crates/ffi/src/lib.rs. Do not edit. */','#ifndef RENDER_ABI_H','#define RENDER_ABI_H','#include <stdint.h>','#ifdef __cplusplus','extern "C" {','#endif','typedef struct RenderApi {','    uint32_t struct_size;','    uint32_t abi_version;']
for name,args,result in re.findall(r'pub\s+(\w+):\s+(?:unsafe\s+)?extern\s+"C"\s+fn\((.*?)\)\s*->\s*(\w+)',body,re.S):
    parameters=', '.join(types[a.strip()] for a in args.split(',') if a.strip()) or 'void'
    lines.append(f'    {types[result]} (*{name})({parameters});')
assert len([line for line in lines if '(*' in line])==6
lines+=['} RenderApi;','const RenderApi *render_entry(uint32_t version, uint32_t minimum_size);','#ifdef __cplusplus','} /* extern C */','#endif','#endif','']
text='\n'.join(lines)
path=Path('include/render.h')
if '--check' in sys.argv:
    assert path.read_text()==text,'generated ABI header is stale'
else:
    path.parent.mkdir(parents=True,exist_ok=True);path.write_text(text)
