use super::Animation;
use crate::render::Canvas;

/// Rotating wireframe Earth with simplified continents
pub struct Globe {
    rotation: f64,
}

impl Globe {
    pub fn new() -> Self {
        Globe { rotation: 0.0 }
    }
}

// Simplified continent data: (longitude_start, longitude_end, latitude_start, latitude_end)
// in degrees, roughly approximating major landmasses
const CONTINENTS: &[(f64, f64, f64, f64)] = &[
    // North America
    (-130.0, -60.0, 25.0, 55.0),
    (-125.0, -100.0, 55.0, 70.0),
    (-170.0, -140.0, 55.0, 72.0),
    // South America
    (-80.0, -35.0, -55.0, 10.0),
    // Europe
    (-10.0, 40.0, 35.0, 60.0),
    (-10.0, 30.0, 60.0, 72.0),
    // Africa
    (-20.0, 50.0, -35.0, 35.0),
    // Asia
    (40.0, 140.0, 10.0, 55.0),
    (60.0, 180.0, 55.0, 75.0),
    // Australia
    (112.0, 155.0, -40.0, -10.0),
    // India
    (70.0, 90.0, 8.0, 30.0),
];

fn is_land(lon_deg: f64, lat_deg: f64) -> bool {
    for &(lon_start, lon_end, lat_start, lat_end) in CONTINENTS {
        if lon_deg >= lon_start && lon_deg <= lon_end && lat_deg >= lat_start && lat_deg <= lat_end
        {
            return true;
        }
    }
    false
}

impl Animation for Globe {
    fn name(&self) -> &str {
        "globe"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let radius = (w.min(h) * 0.4).max(10.0);

        self.rotation += dt * 0.5; // radians per second

        canvas.clear();

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64 - cx;
                let fy = y as f64 - cy;
                let dist_sq = fx * fx + fy * fy;
                let r_sq = radius * radius;

                if dist_sq > r_sq {
                    continue;
                }

                // Map screen position to sphere coordinates
                let nx = fx / radius; // -1 to 1
                let ny = fy / radius; // -1 to 1
                let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();

                // Convert to lat/lon
                let lat = ny.asin();
                let lon = nx.atan2(nz) + self.rotation;

                let lat_deg = lat.to_degrees();
                let mut lon_deg = lon.to_degrees() % 360.0;
                if lon_deg > 180.0 {
                    lon_deg -= 360.0;
                }
                if lon_deg < -180.0 {
                    lon_deg += 360.0;
                }

                // Shading based on surface normal (simple diffuse)
                let shade = nz * 0.8 + 0.2;

                if is_land(lon_deg, lat_deg) {
                    // Land - green/brown
                    let r = (60.0 * shade) as u8;
                    let g = (140.0 * shade) as u8;
                    let b = (50.0 * shade) as u8;
                    canvas.set_colored(x, y, shade * 0.8, r, g, b);
                } else {
                    // Ocean - blue
                    let r = (20.0 * shade) as u8;
                    let g = (50.0 * shade) as u8;
                    let b = (160.0 * shade) as u8;
                    canvas.set_colored(x, y, shade * 0.5, r, g, b);
                }

                // Grid lines (latitude/longitude)
                let grid_lat = (lat_deg / 30.0).fract().abs();
                let grid_lon = (lon_deg / 30.0).fract().abs();
                if grid_lat < 0.04 || grid_lon < 0.04 {
                    canvas.set_colored(x, y, shade * 0.3, 100, 100, 120);
                }

                // Atmosphere edge glow
                let edge = 1.0 - nz;
                if edge > 0.8 {
                    let atm = (edge - 0.8) / 0.2;
                    let r = (100.0 * atm * shade) as u8;
                    let g = (150.0 * atm * shade) as u8;
                    let b = (255.0 * atm * shade) as u8;
                    canvas.set_colored(x, y, atm * 0.5, r, g, b);
                }
            }
        }
    }
}
