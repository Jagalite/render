//! Bounded VOL3 float grids interpreted as native cell-constant sparse media.
//! File filtering, wrapping, units and optical meaning are explicit caller policy.
use crate::{
    Error, Id, Result, canonical, digest,
    document::{Command, Entity, Transform},
    volumes::{Asset, Cell},
};
use serde::{Deserialize, Serialize};

pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
pub const PROFILE: &str = "vol3-cell-constant-v1";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundsPolicy {
    File,
    UnitCube,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reconstruction {
    CellConstantZeroOutside,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub bounds: BoundsPolicy,
    pub reconstruction: Reconstruction,
    pub meters_per_unit: f64,
    pub density_scale: f64,
    /// Linear-sRGB radiance emitted per world meter, independently of density.
    pub emission_scale: [f64; 3],
    pub absorption: [f64; 3],
    pub scattering: [f64; 3],
    pub anisotropy: f64,
    pub max_step_meters: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub profile: String,
    pub source_digest: String,
    pub density_digest: String,
    pub emission_digest: Option<String>,
    pub policy: Policy,
    pub source_dimensions: [u32; 3],
    pub source_bounds: [f64; 6],
    pub domain_meters: [f64; 6],
    pub voxel_size_meters: [f64; 3],
    pub entity: Id,
    pub volume_asset: Option<String>,
    pub input_bytes: u64,
    pub source_cells: u64,
    pub occupied_cells: u32,
    pub asset_json_bytes: u64,
}
pub struct Imported {
    pub commands: Vec<Command>,
    pub report: Report,
}
fn error(code: &str, message: &str) -> Error {
    Error::new(code, message).at("import")
}
fn check(cancelled: &mut impl FnMut() -> bool) -> Result<()> {
    if cancelled() {
        Err(error(
            "cancelled",
            "VOL import cancelled before publication",
        ))
    } else {
        Ok(())
    }
}
struct Grid<'a> {
    bytes: &'a [u8],
    dimensions: [u32; 3],
    bounds: [f64; 6],
    cells: usize,
    channels: usize,
}
impl<'a> Grid<'a> {
    fn parse(bytes: &'a [u8], channels: usize) -> Result<Self> {
        if bytes.len() < 48 || &bytes[..3] != b"VOL" {
            return Err(error("vol", "missing or truncated VOL header"));
        }
        let integer = |offset| i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        if bytes[3] != 3 || integer(4) != 1 || integer(20) != channels as i32 {
            return Err(error(
                "unsupported_vol",
                "requires VOL3 float32 density (1 channel) or emission (3 channels)",
            ));
        }
        let signed = [integer(8), integer(12), integer(16)];
        if signed.iter().any(|n| *n <= 0) {
            return Err(error("vol", "grid dimensions must be positive"));
        }
        let dimensions = signed.map(|n| n as u32);
        let count = dimensions
            .iter()
            .try_fold(1_u64, |n, &d| n.checked_mul(u64::from(d)))
            .ok_or_else(|| error("budget", "grid dimensions overflow"))?;
        let expected = count
            .checked_mul(channels as u64 * 4)
            .and_then(|n| n.checked_add(48))
            .ok_or_else(|| error("budget", "grid byte count overflows"))?;
        if expected > MAX_INPUT_BYTES as u64 {
            return Err(error("budget", "grid exceeds 4 MiB input profile"));
        }
        if expected != bytes.len() as u64 {
            return Err(error("vol", "truncated payload or trailing VOL bytes"));
        }
        let bounds: [f64; 6] = std::array::from_fn(|i| {
            f64::from(f32::from_le_bytes(
                bytes[24 + i * 4..28 + i * 4].try_into().unwrap(),
            ))
        });
        if bounds.iter().any(|v| !v.is_finite()) || (0..3).any(|i| bounds[i] >= bounds[i + 3]) {
            return Err(error(
                "vol",
                "file bounding box must have finite positive extents",
            ));
        }
        Ok(Self {
            bytes,
            dimensions,
            bounds,
            cells: count as usize,
            channels,
        })
    }
    fn value(&self, cell: usize, channel: usize) -> Result<f64> {
        let offset = 48 + (cell * self.channels + channel) * 4;
        let value = f64::from(f32::from_le_bytes(
            self.bytes[offset..offset + 4].try_into().unwrap(),
        ));
        if !value.is_finite() || value < 0. {
            return Err(error("vol", "grid values must be finite and nonnegative")
                .with_context("cell", cell.to_string())
                .with_context("channel", channel.to_string()));
        }
        Ok(value)
    }
}

pub fn import(
    density: &[u8],
    emission: Option<&[u8]>,
    document_id: Id,
    policy: &Policy,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Imported> {
    check(&mut cancelled)?;
    let input_bytes = density
        .len()
        .checked_add(emission.map_or(0, <[u8]>::len))
        .ok_or_else(|| error("budget", "aggregate VOL input overflows"))?;
    if input_bytes > MAX_INPUT_BYTES {
        return Err(error("budget", "aggregate VOL input exceeds 4 MiB"));
    }
    let grid = Grid::parse(density, 1)?;
    let light = emission.map(|bytes| Grid::parse(bytes, 3)).transpose()?;
    if light
        .as_ref()
        .is_some_and(|g| g.dimensions != grid.dimensions || g.bounds != grid.bounds)
    {
        return Err(error(
            "vol",
            "density and emission grids must have identical dimensions and bounds",
        ));
    }
    if !policy.meters_per_unit.is_finite()
        || policy.meters_per_unit <= 0.
        || std::iter::once(&policy.density_scale)
            .chain(&policy.emission_scale)
            .any(|v| !v.is_finite() || !(0.0..=1e6).contains(v))
    {
        return Err(error(
            "volume_policy",
            "invalid metric unit or density/emission scale",
        ));
    }
    let domain = match policy.bounds {
        BoundsPolicy::File => grid.bounds,
        BoundsPolicy::UnitCube => [0., 0., 0., 1., 1., 1.],
    }
    .map(|v| v * policy.meters_per_unit);
    let voxel =
        std::array::from_fn(|i| (domain[i + 3] - domain[i]) / f64::from(grid.dimensions[i]));
    let mut asset = Asset {
        origin: [domain[0], domain[1], domain[2]],
        voxel_size: voxel,
        cells: vec![Cell {
            coordinate: [0; 3],
            density: 1.,
            emission: [0.; 3],
        }],
        absorption: policy.absorption,
        scattering: policy.scattering,
        anisotropy: policy.anisotropy,
        max_step_meters: policy.max_step_meters,
    };
    asset.validate().map_err(|e| e.at("import"))?;
    if domain.iter().any(|v| !v.is_finite() || v.abs() > 1e9) {
        return Err(error(
            "volume_policy",
            "metric domain exceeds finite coordinate profile",
        ));
    }
    asset.cells.clear();
    for i in 0..grid.cells {
        if i % 256 == 0 {
            check(&mut cancelled)?;
        }
        let density = grid.value(i, 0)? * policy.density_scale;
        let mut emission = [0.; 3];
        if let Some(light) = &light {
            for (c, out) in emission.iter_mut().enumerate() {
                *out = light.value(i, c)? * policy.emission_scale[c];
            }
        }
        if std::iter::once(&density)
            .chain(&emission)
            .any(|v| !v.is_finite() || *v > 1e6)
        {
            return Err(error(
                "volume_policy",
                "scaled grid channel exceeds native volume profile",
            )
            .with_context("cell", i.to_string()));
        }
        if density == 0. && emission == [0.; 3] {
            continue;
        }
        if asset.cells.len() == 16384 {
            return Err(error("budget", "VOL import exceeds 16384 occupied cells"));
        }
        let [x, y, _] = grid.dimensions.map(|v| v as usize);
        asset.cells.push(Cell {
            coordinate: [(i % x) as i32, (i / x % y) as i32, (i / x / y) as i32],
            density,
            emission,
        });
    }
    asset.cells.sort_by_key(|cell| cell.coordinate);
    check(&mut cancelled)?;
    let density_digest = digest(density);
    let emission_digest = emission.map(digest);
    let source_digest = digest(&canonical(&(
        density_digest.clone(),
        emission_digest.clone(),
    ))?);
    let identity = digest(&canonical(&(document_id, PROFILE, &source_digest, policy))?);
    let entity = Id(u128::from_str_radix(&identity[7..39], 16).expect("SHA256 hex"));
    let occupied_cells = asset.cells.len() as u32;
    let (volume_asset, asset_json_bytes) = if occupied_cells == 0 {
        (None, 0)
    } else {
        (Some(asset.content_id()?), canonical(&asset)?.len() as u64)
    };
    let mut commands = vec![Command::CreateEntity {
        entity: Entity {
            id: entity,
            name: "Imported VOL volume".into(),
            parent: None,
            mesh: None,
            material: None,
            transform: Transform::default(),
        },
    }];
    if let Some(id) = &volume_asset {
        commands.push(Command::PutVolume { asset });
        commands.push(Command::SetVolume {
            entity,
            asset: Some(id.clone()),
        });
    }
    check(&mut cancelled)?;
    Ok(Imported {
        commands,
        report: Report {
            profile: PROFILE.into(),
            source_digest,
            density_digest,
            emission_digest,
            policy: policy.clone(),
            source_dimensions: grid.dimensions,
            source_bounds: grid.bounds,
            domain_meters: domain,
            voxel_size_meters: voxel,
            entity,
            volume_asset,
            input_bytes: input_bytes as u64,
            source_cells: grid.cells as u64,
            occupied_cells,
            asset_json_bytes,
        },
    })
}
