use napi_derive::napi;

use arrsac::Arrsac;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use sample_consensus::Consensus;

#[derive(Clone, Copy)]
struct Pt {
  x: f64,
  y: f64,
}

#[derive(Clone, Copy)]
struct Line {
  m: f64,
  b: f64,
}

impl sample_consensus::Model<Pt> for Line {
  #[inline]
  fn residual(&self, d: &Pt) -> f64 {
    let r = d.y - (self.m * d.x + self.b);
    r * r
  }
}

struct LineEstimator {
  min_slope: Option<f64>,
  max_slope: Option<f64>,
  min_intercept: Option<f64>,
  max_intercept: Option<f64>,
}

impl LineEstimator {
  fn new(
    min_slope: Option<f64>,
    max_slope: Option<f64>,
    min_intercept: Option<f64>,
    max_intercept: Option<f64>,
  ) -> Self {
    Self {
      min_slope,
      max_slope,
      min_intercept,
      max_intercept,
    }
  }
}

impl sample_consensus::Estimator<Pt> for LineEstimator {
  const MIN_SAMPLES: usize = 2;
  type Model = Line;
  type ModelIter = core::option::IntoIter<Line>;

  fn estimate<I>(&self, mut data: I) -> Self::ModelIter
  where
    I: Iterator<Item = Pt> + Clone,
  {
    let p1 = data.next().unwrap();
    let p2 = data.next().unwrap();
    let dx = p2.x - p1.x;

    let (m, b) = if dx.abs() < f64::EPSILON {
      let m = 1e16_f64.copysign(p2.y - p1.y);
      let b = p1.y - m * p1.x;
      (m, b)
    } else {
      let m = (p2.y - p1.y) / dx;
      let b = p1.y - m * p1.x;
      (m, b)
    };

    if let Some(min) = self.min_slope {
      if m < min {
        return None.into_iter();
      }
    }
    if let Some(max) = self.max_slope {
      if m > max {
        return None.into_iter();
      }
    }
    if let Some(minb) = self.min_intercept {
      if b < minb {
        return None.into_iter();
      }
    }
    if let Some(maxb) = self.max_intercept {
      if b > maxb {
        return None.into_iter();
      }
    }

    Some(Line { m, b }).into_iter()
  }
}

#[napi(object)]
pub struct ArrsacOptions {
  pub inlier_threshold: Option<f64>,
  pub min_slope: Option<f64>,
  pub max_slope: Option<f64>,
  pub min_intercept: Option<f64>,
  pub max_intercept: Option<f64>,
}

#[napi(object)]
pub struct ArrsacResult {
  // Model
  pub slope: f64,
  pub intercept: f64,

  // Indices of inliers (0-based)
  pub inliers: Vec<u32>,

  // Quality measures
  pub r2_all: f64,
  pub r2_inliers: f64, // NaN if <2 inliers
  pub rmse_all: f64,
  pub rmse_inliers: f64, // NaN if <1 inlier
  pub mad_all: f64,
  pub mad_inliers: f64, // NaN if <1 inlier
  pub iqr_all: f64,
  pub iqr_inliers: f64, // NaN if <2 inliers
  pub inlier_ratio: f64,
  pub num_inliers: u32,
  pub num_points: u32,
  pub max_abs_residual: f64,
  pub threshold: f64,
}

// ---------- helpers ----------
fn residuals(model: &Line, xs: &[f64], ys: &[f64]) -> Vec<f64> {
  xs.iter()
    .zip(ys.iter())
    .map(|(&x, &y)| y - (model.m * x + model.b))
    .collect()
}

fn mean(v: &[f64]) -> f64 {
  if v.is_empty() {
    f64::NAN
  } else {
    v.iter().sum::<f64>() / (v.len() as f64)
  }
}

fn rmse_from_resids(res: &[f64]) -> f64 {
  if res.is_empty() {
    f64::NAN
  } else {
    (res.iter().map(|r| r * r).sum::<f64>() / (res.len() as f64)).sqrt()
  }
}

fn r2_from_resids(res: &[f64], ys: &[f64]) -> f64 {
  if ys.len() < 2 {
    return f64::NAN;
  }
  let ybar = mean(ys);
  let ss_tot = ys.iter().map(|y| (y - ybar).powi(2)).sum::<f64>();
  if ss_tot == 0.0 {
    return f64::NAN;
  }
  let ss_res = res.iter().map(|r| r * r).sum::<f64>();
  1.0 - ss_res / ss_tot
}

fn median_sorted(sorted: &[f64]) -> f64 {
  let n = sorted.len();
  if n == 0 {
    return f64::NAN;
  }
  if n % 2 == 1 {
    sorted[n / 2]
  } else {
    0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
  }
}

fn mad(res: &[f64]) -> f64 {
  if res.is_empty() {
    return f64::NAN;
  }
  let med = {
    let mut v = res.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    median_sorted(&v)
  };
  let mut dev: Vec<f64> = res.iter().map(|r| (r - med).abs()).collect();
  dev.sort_by(|a, b| a.partial_cmp(b).unwrap());
  median_sorted(&dev)
}

fn iqr(res: &[f64]) -> f64 {
  let n = res.len();
  if n < 2 {
    return f64::NAN;
  }
  let mut v = res.to_vec();
  v.sort_by(|a, b| a.partial_cmp(b).unwrap());
  // Tukey hinges (simple split)
  let mid = n / 2;
  let (lower, upper) = if n % 2 == 0 {
    (&v[..mid], &v[mid..])
  } else {
    (&v[..mid], &v[mid + 1..])
  };
  let q1 = median_sorted(lower);
  let q3 = median_sorted(upper);
  q3 - q1
}

// ---------- main ----------
#[napi]
pub fn arrsac_line(
  x: Vec<f64>,
  y: Vec<f64>,
  options: Option<ArrsacOptions>,
) -> Option<ArrsacResult> {
  if x.len() != y.len() || x.len() < 2 {
    return None;
  }

  let seed = 0xC0FFEE_u64;
  let rng = Xoshiro256PlusPlus::seed_from_u64(seed);

  let inlier_threshold = options
    .as_ref()
    .and_then(|o| o.inlier_threshold)
    .unwrap_or(0.3);

  let min_slope = options.as_ref().and_then(|o| o.min_slope);
  let max_slope = options.as_ref().and_then(|o| o.max_slope);
  let min_intercept = options.as_ref().and_then(|o| o.min_intercept);
  let max_intercept = options.as_ref().and_then(|o| o.max_intercept);

  let mut arr = Arrsac::new(inlier_threshold, rng);
  let est = LineEstimator::new(min_slope, max_slope, min_intercept, max_intercept);

  let data_iter = x.iter().zip(y.iter()).map(|(&xi, &yi)| Pt { x: xi, y: yi });

  if let Some((model, inliers)) = arr.model_inliers(&est, data_iter.clone()) {
    // residuals over all points
    let res_all = residuals(&model, &x, &y);
    let rmse_all = rmse_from_resids(&res_all);
    let r2_all = r2_from_resids(&res_all, &y);
    let mad_all = mad(&res_all);
    let iqr_all = iqr(&res_all);
    let max_abs_residual = res_all.iter().map(|r| r.abs()).fold(0.0, f64::max);

    // residuals for inliers only (using original indices from ARR-SAC)
    let mut inlier_res: Vec<f64> = Vec::with_capacity(inliers.len());
    for &i in &inliers {
      inlier_res.push(res_all[i as usize]);
    }
    let rmse_inliers = rmse_from_resids(&inlier_res);
    let r2_inliers = if inliers.len() >= 2 {
      let ys_in: Vec<f64> = inliers.iter().map(|&i| y[i as usize]).collect();
      r2_from_resids(&inlier_res, &ys_in)
    } else {
      f64::NAN
    };
    let mad_inliers = mad(&inlier_res);
    let iqr_inliers = iqr(&inlier_res);

    // compute counts *before* moving anything
    let num_points = x.len() as u32;
    let num_inliers = inliers.len() as u32;
    let inlier_ratio = (num_inliers as f64) / (num_points as f64);
    let threshold = inlier_threshold;

    // convert indices to u32 in a *separate* variable
    let inliers_u32: Vec<u32> = inliers.iter().map(|&i| i as u32).collect();

    return Some(ArrsacResult {
      slope: model.m,
      intercept: model.b,
      inliers: inliers_u32,
      r2_all,
      r2_inliers,
      rmse_all,
      rmse_inliers,
      mad_all,
      mad_inliers,
      iqr_all,
      iqr_inliers,
      inlier_ratio,
      num_inliers,
      num_points,
      max_abs_residual,
      threshold,
    });
  }
  None
}
