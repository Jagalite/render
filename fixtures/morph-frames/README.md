# Original morph-frame fixture

CC0-1.0. Geometry, direction deltas, joint scales, animation channels and PNG pixels
are original repository-authored inputs. `generate.py` uses standard-library binary
and PNG encoding, never a renderer. Regenerate into a new directory and compare
with `data/provenance.json`; existing reference files are not updated automatically.

```
python3 fixtures/morph-frames/generate.py /tmp/render-morph-frame-fixture-check
```

A quad carries four independent UV sets and three material images from the named-UV
analytic construction. Three morph targets independently displace position by
(0.1,0,0), normal by (0.5,0,0), and tangent by (0,0.25,0). The latter two are
direction-only targets. Default weights are (0.25,0.5,0.75); the clip linearly
animates all weights from zero to one over one second.

Two joints with scales (2,1,1) and (1,3,1), identity inverse binds and equal weights
yield diag(1.5,2,1). The mesh node's translation is deliberately seven meters in X;
glTF skin semantics ignore that independent node transform. At clip time t, world
positions are (1.5*(x+0.1*t),2*y,0), normals normalize (t/3,0,1), and tangents
normalize (1.5,0.5*t,0), with positive handedness. Reflection tests use negative
joint X scales; singular tests blend opposing equal X scales. Native corner-domain
tests independently expand stable-ID correspondence without changing source files.

Dense and sparse variants carry the same values. The sparse variant supplies its
normal deltas through an accessor with a zero base and four checked overrides.
Their decoded geometry and rendered pixels must agree despite different source
identities. Analytical direction tests, malformed/cancelled/stale workflows,
recovery, Metal/WebGPU frames and shutters provide acceptance; no golden rendering
is generated from these inputs.
