# Original indirect-light cavity

`create.json` is an original synthetic fixture, dedicated to the public domain
under CC0-1.0. It uses ordinary transactional commands, not imported executable
metadata or a privileged fixture API. The camera sees the floor; emission from
the ceiling requires a reflected path. Floor/ceiling use opaque PBR and two side
walls use Lambertian materials. Camera clipping must not clip secondary rays.

Reproduce with `python3 fixtures/multibounce-pbr/generate.py <new-directory>` and
compare `create.json` bytes. Existing files are never overwritten by the generator.
No external reference images or golden outputs are generated. Acceptance thresholds
and independent analytic tests are described in `docs/multibounce_pbr.md`.
