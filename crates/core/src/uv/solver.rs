//! Independently implemented bounded column-pivoted Householder QR.
//! Private numerical storage; public UV requests never expose matrix indices.
use crate::{Error, Result};
#[derive(Debug)]
pub(super) struct Solved {
    pub x: Vec<f64>,
    pub residual: f64,
    pub pivot_ratio: f64,
}
pub(super) fn solve(a: &[Vec<f64>], b: &[f64], mut cancel: impl FnMut() -> bool) -> Result<Solved> {
    let m = a.len();
    let n = a.first().map_or(0, Vec::len);
    if m == 0
        || n == 0
        || n > m
        || n > 256
        || m > 512
        || m * n * n > 16_777_216
        || b.len() != m
        || a.iter().any(|r| r.len() != n)
    {
        return Err(Error::new("budget", "UV least-squares shape"));
    }
    if a.iter().flatten().chain(b).any(|x| !x.is_finite()) {
        return Err(Error::new("uv_numeric", "UV least-squares finite"));
    }
    let mut r = a.to_vec();
    let mut y = b.to_vec();
    let mut permutation: Vec<_> = (0..n).collect();
    let mut max_pivot = 0f64;
    let mut min_pivot = f64::INFINITY;
    for k in 0..n {
        if cancel() {
            return Err(Error::new("cancelled", "UV least-squares cancelled"));
        }
        let pivot = (k..n)
            .max_by(|&i, &j| {
                let left = (k..m).map(|row| r[row][i] * r[row][i]).sum::<f64>();
                let right = (k..m).map(|row| r[row][j] * r[row][j]).sum::<f64>();
                left.total_cmp(&right)
                    .then_with(|| permutation[j].cmp(&permutation[i]))
            })
            .unwrap();
        for row in &mut r {
            row.swap(k, pivot);
        }
        permutation.swap(k, pivot);
        let norm = (k..m).map(|i| r[i][k] * r[i][k]).sum::<f64>().sqrt();
        max_pivot = max_pivot.max(norm);
        min_pivot = min_pivot.min(norm);
        if !norm.is_finite() || norm == 0. || norm <= max_pivot * 1e-10 {
            return Err(Error::new("uv_rank", "UV least-squares rank"));
        }
        let mut v: Vec<_> = (k..m).map(|i| r[i][k]).collect();
        v[0] += norm.copysign(v[0]);
        let vnorm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        for x in &mut v {
            *x /= vnorm;
        }
        let dots: Vec<_> = (k..n)
            .map(|j| {
                r[k..]
                    .iter()
                    .zip(&v)
                    .map(|(row, weight)| weight * row[j])
                    .sum::<f64>()
            })
            .collect();
        for (row, weight) in r[k..].iter_mut().zip(&v) {
            for (value, dot) in row[k..].iter_mut().zip(&dots) {
                *value -= 2. * weight * dot;
            }
        }
        let dot = (k..m).map(|i| v[i - k] * y[i]).sum::<f64>();
        for i in k..m {
            y[i] -= 2. * v[i - k] * dot;
        }
    }
    let mut permuted = vec![0.; n];
    let mut x = vec![0.; n];
    for i in (0..n).rev() {
        permuted[i] = (y[i] - (i + 1..n).map(|j| r[i][j] * permuted[j]).sum::<f64>()) / r[i][i];
    }
    for i in 0..n {
        x[permutation[i]] = permuted[i];
    }
    let residual = a
        .iter()
        .zip(b)
        .map(|(row, y)| (row.iter().zip(&x).map(|(a, x)| a * x).sum::<f64>() - y).powi(2))
        .sum::<f64>()
        .sqrt();
    if cancel() {
        return Err(Error::new("cancelled", "UV least-squares cancelled"));
    }
    if x.iter().any(|v| !v.is_finite()) || !residual.is_finite() {
        return Err(Error::new("uv_numeric", "UV least-squares finite"));
    }
    Ok(Solved {
        x,
        residual,
        pivot_ratio: min_pivot / max_pivot,
    })
}
#[cfg(test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn analytic_rectangular_least_squares() {
    // Minimize (x-1)^2+(y-2)^2+(x+y-4)^2: x=4/3, y=7/3.
    let s = solve(
        &[vec![1., 0.], vec![0., 1.], vec![1., 1.]],
        &[1., 2., 4.],
        || false,
    )
    .unwrap();
    assert!((s.x[0] - 4. / 3.).abs() < 1e-14 && (s.x[1] - 7. / 3.).abs() < 1e-14);
    assert!((s.residual - 1f64 / 3f64.sqrt()).abs() < 1e-14 && s.pivot_ratio > 0.5);
}
#[cfg(test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn rank_and_cancellation() {
    let a = [vec![1., 2.], vec![2., 4.]];
    assert_eq!(solve(&a, &[1., 2.], || false).unwrap_err().code, "uv_rank");
    assert_eq!(solve(&a, &[1., 2.], || true).unwrap_err().code, "cancelled");
    let a = [vec![1., 0.], vec![0., 1.]];
    let mut checks = 0;
    assert_eq!(
        solve(&a, &[1., 2.], || {
            checks += 1;
            checks == 3
        })
        .unwrap_err()
        .code,
        "cancelled"
    );
}
#[cfg(test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn pivot_order_does_not_change_solution() {
    let a = [vec![0.001, 2.], vec![0.002, -1.], vec![0.003, 1.]];
    let b: Vec<_> = a.iter().map(|r| r[0] * 3. + r[1] * 4.).collect();
    let s = solve(&a, &b, || false).unwrap();
    assert!((s.x[0] - 3.).abs() < 1e-10 && (s.x[1] - 4.).abs() < 1e-13 && s.residual < 1e-13);
}
