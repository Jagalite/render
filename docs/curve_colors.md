# Polyline control colors and coverage

Native polyline controls may carry an optional `color: [r,g,b,a]` value: finite,
linear, straight RGBA f32 in [0,1]. Missing colors mean white with full coverage.
The field serializes absent when unused, preserving earlier native asset identities.
Colored geometry assets and groom guides require snapshot v15; ordinary PutGeometry
and SetGroom transactions advance the version. Older versions cannot hide new data.

The polyline evaluator interpolates each segment's RGBA through its existing
radius/tilt subdivision, copies sample colors to polygon tube rings, and uses endpoint
colors for cap centers. A closed polyline interpolates its closing segment as well.
Mixed colored/uncolored curves use white for the latter. The derived mesh uses the
existing point-domain Vec4 `color_rgba` semantic, stable attribute ID200 and linear
transfer. Polygon triangles interpolate these values; this is a polygon-surface
color contract, not analytic fiber shading or a color-space conversion in a shader.

Explicit colors on Bezier and rational B-spline controls are rejected. Geometric
flatness alone cannot bound color approximation, even for geometrically straight
splines. Those bases retain their uncolored implementation until a separate color
error/subdivision contract is established. Point-cloud colors are not added here.

Groom guides preserve the same polyline controls. Generated children inherit guide
colors while the existing root-frame and child-offset operations affect geometry.
Merged strand meshes retain their colors and stable conversion ranges. Color edits
change source/cache identities; native archives and OPFS retain authored values.

CPU, Metal and WebGPU consume the derived attribute through existing PBR material
and coverage paths. RGB multiplies base color; alpha participates only in a material
coverage mode that uses it. Emission remains independent of vertex color. No new
kernel, computational dependency or privileged mutation operation is introduced.
Evaluated GLB export emits COLOR_0; native documents retain the original guide and
curve authoring that evaluated GLB intentionally loses.

The optional HAIR policy field `"point_attributes":"linear_rgba_f32"` selects
`hair-polyline-rgba-v1`. It accepts source point RGB/transparency variation within a
strand, converts RGB under the explicit source color policy, and stores alpha as
f32(1 - f64(source transparency)). The report's `point_attribute_conversion` records
control_count and max_alpha_absolute_error, measuring this subtraction's f32 rounding.
It does not average values or quantize a generated texture. Native white materials
use ordinary opaque PBR for fully covered strands and BLEND with factor1 otherwise.
Grouping separates these two coverage modes. Existing input/sweep budgets apply.

An absent or null policy field preserves `hair-polyline-surface-v1`, including its
uniform-per-strand admission rule, source IDs, serialized reports and rendered
outputs. In the new profile, fully opaque materials use the canonical PBR shape
also recovered from GLB, keeping the same sampler. An unnecessary explicit extended
opaque surface would select a different finite sample sequence; the RGB-only
round-trip fixture guards this distinction without relaxing image tolerances.

Reproduce the original CC0 gradient corpus with
`python3 fixtures/curve-colors/generate.py <new-directory>`.
Run `python3 scripts/validate-curve-colors.py` with the exact packaged web build at
localhost:8777 and isolated Chrome CDP at9224, then collect evidence with
`python3 scripts/collect-curve-colors-evidence.py <passed-run-directory>`.
As in the original HAIR profile, observed_derived_json_bytes is a platform-local
measurement checked against actual evaluation receipts. Synchronous bounded browser
import does not imply asynchronous cancellation; GPU previews remain cancellable.
