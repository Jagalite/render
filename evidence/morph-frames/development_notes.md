# Development observations

Initial core checks compiled the new schema/evaluator. The first analytic test
compile caught three test-only float literals missing their leading zero. After
correction, the five existing animated-glTF tests and three new analytic tests
passed; a cancellation test incorrectly assumed that the tiny fixture reached 40
checks. It was replaced with measurement of the successful check count followed
by cancellation at every observed boundary. All five final morph tests pass,
including point/corner correspondence, zero directions and singular/reflected
blends. No engine diagnostic or test assertion was suppressed.

The first Metal comparison reused an untextured normal-pass tolerance of 2e-5.
The original CPU normal component 0.7875349 and GPU 0.7874408 differed by
0.000094115734. Inspection confirmed existing GPU mip texels round to binary16,
while CPU mip texels use f32 (documented before this change in static_pbr.md).
An initial test-helper compile referenced a nonexistent TextureRole::Normal;
the corrected helper locates normal images from material bindings.

The final oracle independently rounds the fixture's normal samples in [0.5,1)
to the binary16 step 2^-11. The strict 2e-5 direction threshold remains unchanged
against that reference; original f32 CPU/GPU color retains the existing 0.002
RMSE gate. Independent core tests still compare authored direction math to closed
forms. Twelve dense/sparse, time and shutter Metal comparisons then passed.
The first GPU build took 12.38 seconds and failed after 0.19 test seconds; the
final focused GPU run took 2.27 build seconds and 1.15 test seconds. These are
observations of development runs, not performance comparisons. The full acceptance
run records final commands/logs and frozen-source hashes separately.
