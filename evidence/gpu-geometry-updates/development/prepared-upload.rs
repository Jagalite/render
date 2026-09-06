//! Private aligned geometry-buffer upload planning and observable resource counts.
use std::ops::Range;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Reuse,
    Allocate,
    Patch,
    Rewrite,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub kind: Kind,
    pub ranges: Vec<Range<usize>>,
    pub compared_bytes: usize,
}
impl Plan {
    pub fn uploaded_bytes(&self) -> usize {
        self.ranges.iter().map(|r| r.end - r.start).sum()
    }
}

// The caller supplies nonempty arrays of serialized vec4 records. Public scene
// and resource admission runs before this private, infallible cache planner.
pub fn plan(previous: Option<&[u8]>, current: &[u8]) -> Plan {
    assert!(!current.is_empty() && current.len().is_multiple_of(16));
    let Some(previous) = previous.filter(|p| p.len() == current.len()) else {
        return Plan {
            kind: Kind::Allocate,
            ranges: std::iter::once(0..current.len()).collect(),
            compared_bytes: 0,
        };
    };
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut changed_bytes = 0;
    for (i, (before, after)) in previous.chunks_exact(16).zip(current.chunks_exact(16)).enumerate() {
        if before == after {
            continue;
        }
        changed_bytes += 16;
        let start = i * 16;
        if let Some(last) = ranges.last_mut().filter(|last| last.end == start) {
            last.end += 16;
        } else {
            ranges.push(start..start + 16);
        }
        // Bound planner memory too: stop collecting ranges once a full rewrite
        // is inevitable. Comparison cost records visited 16-byte records.
        if ranges.len() > 256 || changed_bytes >= current.len() - current.len() / 4 {
            return Plan {
                kind: Kind::Rewrite,
                ranges: std::iter::once(0..current.len()).collect(),
                compared_bytes: (i + 1) * 16,
            };
        }
    }
    let kind = if ranges.is_empty() { Kind::Reuse } else { Kind::Patch };
    Plan { kind, ranges, compared_bytes: current.len() }
}


#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub kind: Kind,
    pub packed_buffer_bytes: u64,
    pub compared_record_bytes: u64,
    pub uploaded_bytes: u64,
    pub queue_write_calls: u64,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct Statistics {
    pub buffer_allocations: u64,
    pub partial_updates: u64,
    pub full_rewrites: u64,
    pub exact_reuses: u64,
    pub uploaded_bytes: u64,
    pub compared_record_bytes: u64,
    pub queue_write_calls: u64,
    pub retained_shadow_bytes: u64,
    pub last: Option<Report>,
}
impl Statistics {
    pub fn observe(&mut self, plan: &Plan, bytes: usize) {
        match plan.kind {
            Kind::Allocate => self.buffer_allocations += 1,
            Kind::Patch => self.partial_updates += 1,
            Kind::Rewrite => self.full_rewrites += 1,
            Kind::Reuse => self.exact_reuses += 1,
        }
        let writes = if matches!(plan.kind, Kind::Patch | Kind::Rewrite) {
            plan.ranges.len() as u64
        } else { 0 };
        let uploaded = plan.uploaded_bytes() as u64;
        self.uploaded_bytes += uploaded;
        self.compared_record_bytes += plan.compared_bytes as u64;
        self.queue_write_calls += writes;
        self.last = Some(Report {
            kind: plan.kind,
            packed_buffer_bytes: bytes as u64,
            compared_record_bytes: plan.compared_bytes as u64,
            uploaded_bytes: uploaded,
            queue_write_calls: writes,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_writes_reconstruct_exact_bytes_and_preserve_distant_rows() {
        let old = vec![0; 16 * 128];
        let mut new = old.clone();
        new[17] = 1;
        new[32] = 2;
        new[1000] = 3;
        let p = plan(Some(&old), &new);
        assert_eq!(p.kind, Kind::Patch);
        assert_eq!(p.ranges, vec![16..48, 992..1008]);
        assert_eq!(p.uploaded_bytes(), 48);
        assert_eq!(p.compared_bytes, new.len());
        let mut reconstructed = old;
        for range in p.ranges {
            reconstructed[range.clone()].copy_from_slice(&new[range]);
        }
        assert_eq!(reconstructed, new);
        assert_eq!(plan(Some(&new), &new).kind, Kind::Reuse);
    }
    #[test]
    fn first_resize_dense_and_fragmented_inputs_have_explicit_costs() {
        let old = vec![0; 16 * 1024];
        assert_eq!(plan(None, &old).kind, Kind::Allocate);
        assert_eq!(plan(Some(&old[..16]), &old).compared_bytes, 0);
        let mut dense = old.clone();
        dense[..16 * 768].fill(1);
        let p = plan(Some(&old), &dense);
        assert_eq!(p.kind, Kind::Rewrite);
        assert_eq!(p.uploaded_bytes(), old.len());
        let mut scattered = old.clone();
        for i in 0..257 { scattered[i * 32] = 1; }
        assert_eq!(plan(Some(&old), &scattered).kind, Kind::Rewrite);
    }
    #[test]
    fn float_bits_are_compared_without_numeric_equality_or_nan_rules() {
        let mut a = vec![0; 16 * 8];
        let mut b = a.clone();
        b[..4].copy_from_slice(&(-0.0f32).to_le_bytes());
        assert_eq!(plan(Some(&a), &b).kind, Kind::Patch);
        a[4..8].copy_from_slice(&f32::from_bits(0x7fc00001).to_le_bytes());
        assert_eq!(plan(Some(&a), &a).kind, Kind::Reuse);
    }
    #[test]
    fn every_small_dirty_pattern_reconstructs_the_target() {
        let old = vec![0; 16 * 8];
        for bits in 0u16..256 {
            let mut new = old.clone();
            for i in 0..8 {
                if bits & (1 << i) != 0 { new[i * 16 + 3] = 99; }
            }
            let p = plan(Some(&old), &new);
            let mut actual = old.clone();
            for range in &p.ranges {
                assert_eq!(range.start % 16, 0);
                assert_eq!(range.end % 16, 0);
                actual[range.clone()].copy_from_slice(&new[range.clone()]);
            }
            assert_eq!(actual, new);
            assert!(p.compared_bytes <= new.len());
        }
    }

}
