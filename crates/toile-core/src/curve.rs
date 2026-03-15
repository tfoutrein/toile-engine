/// A piecewise-linear curve with keypoints, sampled over 0..1.
///
/// Used for particle size-over-lifetime and other property animations.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Curve {
    /// Sorted keypoints: (time, value) where time is in 0..1.
    pub points: Vec<(f32, f32)>,
}

impl Curve {
    /// Create a constant curve.
    pub fn constant(value: f32) -> Self {
        Self {
            points: vec![(0.0, value), (1.0, value)],
        }
    }

    /// Create a linear curve from start to end.
    pub fn linear(start: f32, end: f32) -> Self {
        Self {
            points: vec![(0.0, start), (1.0, end)],
        }
    }

    /// Create a curve from keypoints. Points are sorted by time automatically.
    pub fn from_points(mut points: Vec<(f32, f32)>) -> Self {
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if points.is_empty() {
            points = vec![(0.0, 0.0), (1.0, 0.0)];
        }
        Self { points }
    }

    /// Sample the curve at normalized time `t` (0..1).
    pub fn sample(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        if self.points.len() <= 1 {
            return self.points.first().map(|p| p.1).unwrap_or(0.0);
        }

        // Find the two surrounding keypoints
        for i in 1..self.points.len() {
            if t <= self.points[i].0 {
                let (t0, v0) = self.points[i - 1];
                let (t1, v1) = self.points[i];
                let range = t1 - t0;
                if range < 0.0001 {
                    return v1;
                }
                let local_t = (t - t0) / range;
                return v0 + (v1 - v0) * local_t;
            }
        }

        self.points.last().unwrap().1
    }
}

impl Default for Curve {
    fn default() -> Self {
        Self::constant(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_curve() {
        let c = Curve::constant(5.0);
        assert!((c.sample(0.0) - 5.0).abs() < 0.01);
        assert!((c.sample(0.5) - 5.0).abs() < 0.01);
        assert!((c.sample(1.0) - 5.0).abs() < 0.01);
    }

    #[test]
    fn linear_curve() {
        let c = Curve::linear(0.0, 10.0);
        assert!((c.sample(0.0)).abs() < 0.01);
        assert!((c.sample(0.5) - 5.0).abs() < 0.01);
        assert!((c.sample(1.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn multi_point_curve() {
        let c = Curve::from_points(vec![(0.0, 0.0), (0.5, 10.0), (1.0, 0.0)]);
        assert!((c.sample(0.25) - 5.0).abs() < 0.01);
        assert!((c.sample(0.5) - 10.0).abs() < 0.01);
        assert!((c.sample(0.75) - 5.0).abs() < 0.01);
    }
}
