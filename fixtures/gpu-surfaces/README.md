# Original conductor and coat workflow recipes

`cases.json` records eight original native material recipes and SHA256-pinned CC0
source GLBs retained with their original provenance elsewhere in fixtures. The
recipes apply existing typed conductor or coated models to each imported PBR material,
preserving its alpha policy and all selected UV, vertex color and texture bindings.
No external glTF material extension is claimed. No images or goldens are generated.

`scripts/gpu-surfaces-workflow.py <new-directory>` performs ordinary import and
material transactions, idempotent retry, CPU/Metal rendering, archive recovery and
invalid/stale/cancel/budget tests. Secondary-only emitters use8x8,32 samples,depth2;
other cases use16x16,16 samples,depth4. Morph cases render a three-sample shutter
around t1/2, width1/16, then an ordered two-frame sequence at1/4 and3/4.
Requests, exact settings and resulting archives are retained as reproducible evidence.
