use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Stable mesh-domain identity, encoded without JavaScript integer rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElementId(pub u64);
impl Serialize for ElementId {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for ElementId {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let n = s.parse::<u64>().map_err(serde::de::Error::custom)?;
        if n == 0 || n.to_string() != s {
            return Err(serde::de::Error::custom(
                "expected a positive canonical decimal u64 string",
            ));
        }
        Ok(Self(n))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub max_source_bytes: u64,
    pub max_build_bytes: u64,
    pub max_triangulation_work: u64,
    pub max_visited_nodes: u64,
    pub max_item_tests: u64,
    pub max_hits: u64,
    pub max_result_bytes: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_source_bytes: 8 << 20,
            max_build_bytes: 32 << 20,
            max_triangulation_work: 16_777_216,
            max_visited_nodes: 262_144,
            max_item_tests: 131_072,
            max_hits: 4096,
            max_result_bytes: 1 << 20,
        }
    }
}
impl Budget {
    pub(super) fn validate(&self) -> Result<()> {
        let cap = Self::default();
        for (n, max) in [
            (self.max_source_bytes, cap.max_source_bytes),
            (self.max_build_bytes, cap.max_build_bytes),
            (self.max_triangulation_work, cap.max_triangulation_work),
            (self.max_visited_nodes, cap.max_visited_nodes),
            (self.max_item_tests, cap.max_item_tests),
            (self.max_hits, cap.max_hits),
            (self.max_result_bytes, cap.max_result_bytes),
        ] {
            if n == 0 || n > max {
                return Err(Error::new(
                    "budget",
                    "query budgets must be positive and within profile caps",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    PointsInSphere {
        center: [f64; 3],
        radius_meters: f64,
    },
    NearestSurface {
        point: [f64; 3],
        max_distance_meters: f64,
    },
}
impl Query {
    pub(super) fn metrics(&self) -> Result<(glam::DVec3, f64)> {
        let (p, r) = match *self {
            Self::PointsInSphere {
                center,
                radius_meters,
            } => (center, radius_meters),
            Self::NearestSurface {
                point,
                max_distance_meters,
            } => (point, max_distance_meters),
        };
        if p.iter().any(|v| !v.is_finite() || v.abs() > 1e9)
            || !r.is_finite()
            || !(0.0..=1e9).contains(&r)
        {
            return Err(Error::new(
                "query_metric",
                "finite local meter coordinates within +/-1e9 and distance in 0..=1e9 required",
            ));
        }
        Ok((glam::DVec3::from_array(p), r))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub base_revision: String,
    pub mesh: String,
    pub query: Query,
    pub budget: Budget,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PointHit {
    pub point: ElementId,
    pub position_meters: [f64; 3],
    pub distance_meters: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SurfaceHit {
    pub face: ElementId,
    pub corners: [ElementId; 3],
    pub barycentric: [f64; 3],
    pub position_meters: [f64; 3],
    pub geometric_normal: [f64; 3],
    pub distance_meters: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Hits {
    PointsInSphere {
        points: Vec<PointHit>,
    },
    NearestSurface {
        surface: Option<SurfaceHit>,
        exact_tied_triangles: u64,
    },
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    pub cache_reused: bool,
    pub whole_snapshot_validated: bool,
    pub additional_source_identity_validation: bool,
    pub source_bytes: u64,
    pub build_byte_charge: u64,
    pub triangulation_work_charge: u64,
    pub built_points: u64,
    pub built_triangles: u64,
    pub retained_index_layout_bytes: u64,
    pub visited_nodes: u64,
    pub item_tests: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub profile: String,
    pub revision: String,
    pub mesh: String,
    pub hits: Hits,
    pub cost: Cost,
}
