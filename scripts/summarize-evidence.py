"""Summarize successful runs; never modify the versioned evidence snapshot."""
import hashlib
import json
import re
import shutil
from pathlib import Path

source = Path("artifacts/evidence")
out = Path("artifacts/foundation-evidence")
out.mkdir(parents=True, exist_ok=True)

def read(name):
    return json.loads((source / name).read_text())

def write(name, value):
    (out / name).write_text(json.dumps(value, indent=2) + "\n")

validation = read("validation_report.json")
assert validation["status"] == "passed" and len(validation["checks"]) == 14
native = set(re.findall(r"^test (\S+) \.\.\. ok$", (source / "logs/native_tests.txt").read_text(), re.M))
wasm = set(re.findall(r"^test (\S+) \.\.\. ok$", (source / "logs/wasm_tests.txt").read_text(), re.M))
assert len(native) >= 40 and len(wasm) >= 31, (len(native), len(wasm))

def tests(*names):
    assert set(names) <= native, set(names) - native
    return {"status": "passed", "checks": [{"test": n, "native": "passed", "wasm": "passed" if n in wasm else "native_only"} for n in names],
            "sources": ["validation_report.json", "logs/native_tests.txt", "logs/wasm_tests.txt"],
            "test_source": "../../crates/core/tests/conformance.rs"}

document = tests("identity_and_rational_time", "canonical_order_independent_of_table_layout", "snapshots_copy_only_changed_entity_chunk", "negative_shear_parent_and_layer_semantics", "ordered_transform_channels_and_external_texture", "shared_browser_suite", "later_layer_restores_hidden_geometry")
geometry = tests("mesh_preserves_seams_and_loose_elements", "malformed_topology_rejected", "concave_polygon_area_and_degeneracy", "radial_nonmanifold_and_generation_safety", "foreign_edit_handle_and_hostile_accessor_are_rejected")
api = tests("transactions_are_atomic_and_revision_checked", "retries_wire_permissions_and_budgets", "staged_upload_permissions_checksums_and_budget", "jobs_events_cancellation_and_recovery", "durable_host_metadata_cannot_be_overwritten_by_stale_snapshot", "prepared_noop_candidates_cannot_overfill_retention", "cancelled_jobs_release_queue_capacity_and_retries_preserve_events")
durability = tests("durable_failure_quota_recovery_and_idempotency", "journal_sequence_is_checked", "native_publication_fault_boundaries", "process_termination_recovers_committed_root", "native_mesh_chunks_deduplicate_and_detect_corruption", "durable_retry_publishes_to_the_requested_store")
durability["native_process_termination_boundaries"] = ["AfterWrite", "AfterFileSync", "AfterRename", "AfterDirectorySync"]
durability["acknowledgment_policy"] = "Before rename: old root; after rename: new valid root may exist despite lost acknowledgment. Recover and retry the same key."
cpu = tests("renderer_reuses_geometry_after_transform", "triangle_bvh_and_parallel_slab_analytic", "diffuse_energy_pdf_and_sample_mean", "empty_scene_environment_is_analytic", "diffuse_plane_radiance_matches_analytic_solution", "unsupported_scattering_is_diagnosed", "render_reproducibility_and_color_boundaries")
io = tests("obj_and_gltf_profiles_roundtrip", "foreign_edit_handle_and_hostile_accessor_are_rejected", "ordered_transform_channels_and_external_texture")
io["profiles"] = ["OBJ polygons, UV corners, loose lines; negative indices on import", "glTF 2.0 mesh-only, nonindexed triangles, f32 POSITION/TEXCOORD_0, caller-supplied binary", "PPM P6 display sRGB to linear texture"]
io["exclusions"] = ["general scene/material fidelity", "animation, skinning, sparse/indexed glTF, required extensions", "external URI fetching"]
write("document_conformance.json", document)
write("geometry_invariants.json", geometry)
write("api_conformance.json", api)
write("job_host_conformance.json", tests("job_host::tests::durable_jobs_survive_reopen_and_cancel", "job_host::tests::second_host_cannot_recover_an_active_worker", "job_host::tests::jobs_pin_the_document_written_through_the_project_api"))
write("idempotency_faults.json", tests("retries_wire_permissions_and_budgets", "durable_failure_quota_recovery_and_idempotency"))
write("durability_faults.json", durability)
write("renderer_cpu_conformance.json", cpu)
write("interchange_profile_report.json", io)
write("roundtrip_report.json", tests("binary_mesh_chunks_preserve_precision_and_reject_truncation", "native_mesh_chunks_deduplicate_and_detect_corruption", "shared_browser_suite"))

gpu, browser = read("gpu_conformance.json"), read("browser_conformance.json")
for report in [gpu, browser]:
    assert report["status"] == "passed"
    assert report["cancel_after_submit"] and report["gpu_over_limit_allocation_rejected"]
    assert report["analytic_plane_max_error"] <= report["analytic_plane_tolerance"]
assert browser["storage_quota"]["committed_root_unchanged"] and browser["tab_reload_preserves_unclosed_root"]
write("cross_backend_report.json", {"status": "passed", "native": gpu, "browser": browser, "cpu_tests": cpu,
                                    "comparison_profile": "same indexed samples, diffuse one-bounce; independent plane radiance and empty environment; no arbitrary-device or broad-material claim"})
write("kernel_cross_target_report.json", {"status": "passed", "generator": "Rust render-kernel expression/statement IR; Naga validation", "native": gpu, "browser": browser})
write("storage_recovery_report.json", {"status": "passed", "native": durability, "browser": {k: browser[k] for k in ["opfs_roundtrip", "aborted_write_preserves_root", "local_export_roundtrip", "storage_quota", "tab_reload_preserves_unclosed_root"]}, "limits": "Process termination and tab interruption; not physical power-cut testing. OPFS remains origin-managed storage."})
write("device_capability_manifest.json", {"native": gpu["device"], "browser": browser["device"], "browser_version": browser["browser"], "other_native_targets": "Linux and Windows compile only", "profile": "baseline negotiated WebGPU limits; no hardware ray queries; oversized dispatches/buffers rejected"})
write("gpu_fault_report.json", {"status": "passed", "native": {k: gpu[k] for k in ["cancel_before_submit", "cancel_after_submit", "gpu_over_limit_allocation_rejected", "memory_admission_rejected", "explicit_device_destroy_rejected", "device_recreated_equal"]}, "browser": {k: browser[k] for k in ["cancel_before_submit", "cancel_after_submit", "gpu_over_limit_allocation_rejected", "budget_rejection", "device_destroy_rejected", "device_recreated_equal"]}, "limits": "Submitted cancellation discards completed results; no hardware preemption. Explicit device destroy/recreate; no spontaneous driver reset or physical VRAM exhaustion."})
benchmark = read("representation_comparison.json")
benchmark["sharing_measurement"] = read("benchmark_results.json")
benchmark["process_resource_log"] = "logs/representation_benchmark.txt"
write("benchmark_results.json", benchmark)
write("test_inventory.json", {"native_tests": sorted(native), "wasm_tests": sorted(wasm), "native_count": len(native), "wasm_count": len(wasm), "wasm_runner": "Node wasm-bindgen-test-runner; browser shared fixture and GPU tests are separately recorded"})
for name in ["validation_report.json", "dependency_inventory.json", "abi_conformance.json", "browser_conformance.json", "gpu_conformance.json", "reference.receipt.json", "gpu.receipt.json", "reference.ppm", "reference.pfm", "gpu.ppm", "gpu.pfm", "raster.ppm", "browser.png"]:
    shutil.copyfile(source / name, out / name)
shutil.copytree(source / "logs", out / "logs", dirs_exist_ok=True)
# Trim terminal progress padding, retaining every diagnostic and result line.
for path in (out / "logs").glob("*.txt"):
    text = "\n".join(line.rstrip() for line in path.read_text().splitlines()).rstrip()
    path.write_text(text + "\n" if text else "")
manifest = []
for name in ["reference.ppm", "reference.pfm", "gpu.ppm", "gpu.pfm", "raster.ppm", "browser.png"]:
    p = out / name
    manifest.append({"path": name, "bytes": p.stat().st_size, "sha256": hashlib.sha256(p.read_bytes()).hexdigest(), "role": "observed output; analytic tests are the oracle, not an auto-updated golden"})
write("reference_images_manifest.json", {"artifacts": manifest, "fixture": "crates/core/src/fixtures.rs::demo", "commands": ["target/debug/render-host verify artifacts/evidence --gpu", "node scripts/browser-conformance.mjs"]})
paths = [Path("Cargo.toml"), Path("Cargo.lock"), Path("rust-toolchain.toml"), Path(".gitattributes"), Path(".gitignore")]
for directory in ["crates", "scripts", "include"]:
    paths.extend(p for p in Path(directory).rglob("*") if p.is_file())
paths.append(Path("web/index.html"))
write("source_manifest.json", {"files": [{"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(paths)], "purpose": "identifies implementation inputs used for these runs; not a release signature"})
print(json.dumps({"status": "summarized", "destination": str(out), "native_tests": len(native), "wasm_tests": len(wasm)}))
