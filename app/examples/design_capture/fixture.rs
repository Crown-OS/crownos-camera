use crownos_camera::util::rgba_image::RgbaImage;

const FRAME_WIDTH: u32 = 1280;
const FRAME_HEIGHT: u32 = 720;
const THUMBNAIL_FACTOR: u32 = 6;

const SKY_TOP: [f32; 3] = [34.0, 48.0, 82.0];
const SKY_HORIZON: [f32; 3] = [236.0, 156.0, 112.0];
const SUN: [f32; 3] = [255.0, 226.0, 178.0];
const FAR_HILLS: [f32; 3] = [92.0, 72.0, 98.0];
const NEAR_HILLS: [f32; 3] = [30.0, 30.0, 46.0];

/// A dusk landscape, mirror-symmetric so the mirrored preview looks the same.
pub fn viewfinder_frame() -> RgbaImage {
    landscape(FRAME_WIDTH, FRAME_HEIGHT)
}

/// The same landscape at thumbnail size.
pub fn thumbnail() -> RgbaImage {
    landscape(
        FRAME_WIDTH / THUMBNAIL_FACTOR,
        FRAME_HEIGHT / THUMBNAIL_FACTOR,
    )
}

fn landscape(width: u32, height: u32) -> RgbaImage {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        for column in 0..width {
            let x = (column as f32 + 0.5) / width as f32 - 0.5;
            let y = (row as f32 + 0.5) / height as f32;
            pixels.extend(shade(x.abs(), y).map(|channel| channel.clamp(0.0, 255.0) as u8));
            pixels.push(u8::MAX);
        }
    }
    RgbaImage {
        width,
        height,
        pixels,
    }
}

fn shade(x: f32, y: f32) -> [f32; 3] {
    let sky = mix(SKY_TOP, SKY_HORIZON, (y / 0.62).clamp(0.0, 1.0).powf(1.6));
    let sun_distance = (x * x * 3.16 + (y - 0.5) * (y - 0.5)).sqrt();
    let glow = (-sun_distance * 7.0).exp();
    let disc = smooth_step(0.075, 0.068, sun_distance);
    let mut color = mix(mix(sky, SUN, glow), SUN, disc);
    let far_ridge = 0.56 + 0.05 * (x * 9.0).cos() + 0.02 * (x * 23.0).cos();
    color = mix(
        color,
        FAR_HILLS,
        smooth_step(far_ridge, far_ridge + 0.004, y),
    );
    let near_ridge = 0.68 + 0.07 * (x * 5.0 + 1.2).cos();
    color = mix(
        color,
        NEAR_HILLS,
        smooth_step(near_ridge, near_ridge + 0.004, y),
    );
    let vignette = 1.0 - 0.35 * (x * x * 2.2 + (y - 0.5) * (y - 0.5)).sqrt();
    color.map(|channel| channel * vignette)
}

fn mix(from: [f32; 3], to: [f32; 3], amount: f32) -> [f32; 3] {
    [0, 1, 2].map(|index| from[index] + (to[index] - from[index]) * amount)
}

fn smooth_step(edge_from: f32, edge_to: f32, value: f32) -> f32 {
    let t = ((value - edge_from) / (edge_to - edge_from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
