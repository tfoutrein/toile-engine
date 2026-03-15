/// An RGBA color gradient with keypoints, sampled over 0..1.
///
/// Used for particle color-over-lifetime and other color animations.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gradient {
    /// Sorted stops: (time, [r, g, b, a]) where time is in 0..1 and colors are 0..1.
    pub stops: Vec<(f32, [f32; 4])>,
}

impl Gradient {
    /// Create a solid color gradient.
    pub fn solid(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            stops: vec![(0.0, [r, g, b, a]), (1.0, [r, g, b, a])],
        }
    }

    /// White to transparent (common for particles).
    pub fn fade_out() -> Self {
        Self {
            stops: vec![
                (0.0, [1.0, 1.0, 1.0, 1.0]),
                (1.0, [1.0, 1.0, 1.0, 0.0]),
            ],
        }
    }

    /// Create a gradient from color stops.
    pub fn from_stops(mut stops: Vec<(f32, [f32; 4])>) -> Self {
        stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if stops.is_empty() {
            stops = vec![(0.0, [1.0, 1.0, 1.0, 1.0]), (1.0, [1.0, 1.0, 1.0, 1.0])];
        }
        Self { stops }
    }

    /// Sample the gradient at normalized time `t` (0..1). Returns [r, g, b, a].
    pub fn sample(&self, t: f32) -> [f32; 4] {
        let t = t.clamp(0.0, 1.0);

        if self.stops.len() <= 1 {
            return self.stops.first().map(|s| s.1).unwrap_or([1.0; 4]);
        }

        for i in 1..self.stops.len() {
            if t <= self.stops[i].0 {
                let (t0, c0) = &self.stops[i - 1];
                let (t1, c1) = &self.stops[i];
                let range = t1 - t0;
                if range < 0.0001 {
                    return *c1;
                }
                let local_t = (t - t0) / range;
                return [
                    c0[0] + (c1[0] - c0[0]) * local_t,
                    c0[1] + (c1[1] - c0[1]) * local_t,
                    c0[2] + (c1[2] - c0[2]) * local_t,
                    c0[3] + (c1[3] - c0[3]) * local_t,
                ];
            }
        }

        self.stops.last().unwrap().1
    }

    /// Convert a sampled [r,g,b,a] (0..1) to a packed u32 ABGR color.
    pub fn sample_packed(&self, t: f32) -> u32 {
        let [r, g, b, a] = self.sample(t);
        let r = (r * 255.0) as u32;
        let g = (g * 255.0) as u32;
        let b = (b * 255.0) as u32;
        let a = (a * 255.0) as u32;
        r | (g << 8) | (b << 16) | (a << 24)
    }
}

impl Default for Gradient {
    fn default() -> Self {
        Self::solid(1.0, 1.0, 1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_gradient() {
        let g = Gradient::solid(1.0, 0.0, 0.0, 1.0);
        let c = g.sample(0.5);
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1]).abs() < 0.01);
    }

    #[test]
    fn fade_out_gradient() {
        let g = Gradient::fade_out();
        let start = g.sample(0.0);
        let end = g.sample(1.0);
        assert!((start[3] - 1.0).abs() < 0.01);
        assert!((end[3]).abs() < 0.01);
    }

    #[test]
    fn interpolation() {
        let g = Gradient::from_stops(vec![
            (0.0, [0.0, 0.0, 0.0, 1.0]),
            (1.0, [1.0, 1.0, 1.0, 1.0]),
        ]);
        let mid = g.sample(0.5);
        assert!((mid[0] - 0.5).abs() < 0.01);
    }
}
