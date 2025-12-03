// Texture shader - for rendering images with pan/zoom and adjustments

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

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<uniform> adjustments: ImageAdjustments;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.transform * vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}

// Fragment shader

@group(1) @binding(0)
var t_texture: texture_2d<f32>;

@group(1) @binding(1)
var t_sampler: sampler;

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
    var color = textureSample(t_texture, t_sampler, in.tex_coords);

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
