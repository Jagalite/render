# GPU sparse media acceptance

GPU sparse media acceptance: 167 native and 142 Wasm tests, 376 media CLI calls over 21 cases, 117 external VOL3 CLI calls and 13 shutter/sequence calls, with actual Chrome CPU/WebGPU/OPFS recovery. All 993 prior images/passes and 331 receipts remain byte identical, as do all four prior shaders and Cargo files. Maximum browser GPU analytic channel error is 3.24428673e-07. The named profile admits 64 zero-scattering cells and ordinary opaque PBR surfaces under explicit work/precision limits. Scattering, joint advanced surfaces and larger paging remain separate gates; M09 stays deferred.

See `validation_report.json` for exact commands and platform scope;
`media_cross_platform.json` for native/Wasm/Metal/WebGPU comparisons;
`resources.json` for process observations; `shader_provenance.json` for generated
source and maintained Rust inputs; `regression_identity.json` for prior artifacts;
and `source_integrity.json`/`tested_package.json` for the frozen tested sources and
packaged binaries. The original analytic fixture is independently reproduced.

The `media-workflow` directory contains full authoring requests, negative cases,
images, receipts and native archives for 17 analytic and four surface-depth cases.
`media-shutter` contains nominal/temporal precision failures, successful frames,
complete and partial sequences, and browser cancellation/OPFS evidence. The
external VOL3 workflow reports cover eight unchanged source-policy cases.

Development logs retain failed attempts and corrections, including the shared-face
rounding hole exposed by the existing VOL3 corpus. Inputs, CPU reference behavior
and tolerances were preserved. Explicit device destruction/recreation was tested;
uncontrolled driver loss was not injected. Linux/Windows evidence is compilation
only. This profile adds neither scattering nor simulation nor advanced-surface media.

Copied text logs have normalized line endings, trailing whitespace and blank lines at EOF only;
`log_normalization.json` identifies them and the preserved raw artifact directory.
The final implementation manifest includes documentation and evidence tooling
finished after validation; all tested Rust and Cargo inputs remain unchanged.
