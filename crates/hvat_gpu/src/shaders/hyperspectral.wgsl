// Hyperspectral band compositing shader
//
// This shader samples from multiple band textures and composites them
// into an RGB output based on selected band indices.
//
// Bands are packed into RGBA textures (4 bands per texture):
// - band_textures_0: bands 0-3 in R, G, B, A channels
// - band_textures_1: bands 4-7 in R, G, B, A channels
//
// The band_selection uniform specifies which bands to use for R, G, B output.

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct Uniforms {
    transform: mat4x4<f32>,
}

struct ImageAdjustments {
    brightness: f32,
    contrast: f32,
    gamma: f32,
    hue_shift: f32,
}

struct BandSelection {
    red_band: u32,
    green_band: u32,
    blue_band: u32,
    num_bands: u32,
}

// Group 0: Uniforms
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<uniform> adjustments: ImageAdjustments;

@group(0) @binding(2)
var<uniform> band_selection: BandSelection;

// Group 1: Band textures (packed 4 bands per RGBA texture)
@group(1) @binding(0)
var band_texture_0: texture_2d<f32>;  // Bands 0-3

@group(1) @binding(1)
var band_texture_1: texture_2d<f32>;  // Bands 4-7

@group(1) @binding(2)
var band_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.transform * vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

// Sample a specific band from the packed textures
fn sample_band(tex_coords: vec2<f32>, band_index: u32) -> f32 {
    let texture_index = band_index / 4u;
    let channel_index = band_index % 4u;

    var sample: vec4<f32>;
    if texture_index == 0u {
        sample = textureSample(band_texture_0, band_sampler, tex_coords);
    } else {
        sample = textureSample(band_texture_1, band_sampler, tex_coords);
    }

    // Extract the correct channel
    if channel_index == 0u {
        return sample.r;
    } else if channel_index == 1u {
        return sample.g;
    } else if channel_index == 2u {
        return sample.b;
    } else {
        return sample.a;
    }
}

// Convert RGB to HSV
fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let max_c = max(max(rgb.r, rgb.g), rgb.b);
    let min_c = min(min(rgb.r, rgb.g), rgb.b);
    let delta = max_c - min_c;

    var h: f32 = 0.0;
    var s: f32 = 0.0;
    let v = max_c;

    if delta > 0.00001 {
        s = delta / max_c;

        if max_c == rgb.r {
            h = (rgb.g - rgb.b) / delta;
            if rgb.g < rgb.b {
                h = h + 6.0;
            }
        } else if max_c == rgb.g {
            h = 2.0 + (rgb.b - rgb.r) / delta;
        } else {
            h = 4.0 + (rgb.r - rgb.g) / delta;
        }
        h = h / 6.0;
    }

    return vec3<f32>(h, s, v);
}

// Convert HSV to RGB
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;

    let c = v * s;
    let x = c * (1.0 - abs(h % 2.0 - 1.0));
    let m = v - c;

    var rgb: vec3<f32>;

    if h < 1.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if h < 2.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if h < 3.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if h < 4.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if h < 5.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + vec3<f32>(m, m, m);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample selected bands for RGB composite
    let r = sample_band(in.tex_coords, band_selection.red_band);
    let g = sample_band(in.tex_coords, band_selection.green_band);
    let b = sample_band(in.tex_coords, band_selection.blue_band);

    var color = vec4<f32>(r, g, b, 1.0);

    // Apply brightness (additive)
    color = vec4<f32>(color.rgb + vec3<f32>(adjustments.brightness), color.a);

    // Apply contrast (multiplicative around 0.5)
    color = vec4<f32>((color.rgb - 0.5) * adjustments.contrast + 0.5, color.a);

    // Apply gamma correction
    color = vec4<f32>(pow(max(color.rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / adjustments.gamma)), color.a);

    // Apply hue shift
    if abs(adjustments.hue_shift) > 0.001 {
        var hsv = rgb_to_hsv(color.rgb);
        hsv.x = hsv.x + adjustments.hue_shift / 360.0;
        // Wrap hue to 0-1 range
        hsv.x = hsv.x - floor(hsv.x);
        color = vec4<f32>(hsv_to_rgb(hsv), color.a);
    }

    // Clamp final output
    return clamp(color, vec4<f32>(0.0), vec4<f32>(1.0));
}
