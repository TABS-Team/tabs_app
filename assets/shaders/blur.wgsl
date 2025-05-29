#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_render::view::View

// bind group 0 (automatically provided by Bevy’s UI pipeline):
//   - ViewUniform contains `viewport: vec4<f32>` whose .zw are (width, height)
//   - GlobalsUniform is bound at @group(0) @binding(1) if you need it
@group(0) @binding(0)
var<uniform> view: View;

// bind group 1: your BlurMaterial
@group(1) @binding(0) var<uniform> color: vec4<f32>;       // RGBA tint
@group(1) @binding(1) var<uniform> radius: i32;            // σ in pixels
@group(1) @binding(2) var scene_texture: texture_2d<f32>;  // offscreen UI image
@group(1) @binding(3) var scene_sampler: sampler;          // its sampler

// A handy constant for 2π, if you ever want to include the normalization factor
const PI2: f32 = 6.283185307179586;

// The fragment entry point
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // 1) UV coords and viewport size in pixels
    let uv       = view.viewport.zw;
    let vp_size  = view.viewport.zw;     // (width, height) of the render target

    // 2) Prepare Gaussian parameters
    let sigma    = radius;               // your material’s radius = σ
    let twoSigma2 = 2.0 * f32(sigma) * f32(sigma); // 2σ²
    // Choose kernel extent: we’ll sample out to ±ceil(3σ) to capture ~99% of the weight
    let kernelRadius = i32(ceil(3.0 * f32(sigma)));

    var sum       = vec4<f32>(0.0);      // accumulated color
    var wsum      = 0.0;                 // accumulated weight

    // 3) Convolution: loop over a (2R+1)² window
    for (var x: i32 = -kernelRadius; x <= kernelRadius; x = x + 1) {
        for (var y: i32 = -kernelRadius; y <= kernelRadius; y = y + 1) {
            // compute a UV offset of exactly one pixel in X/Y
            let offset = vec2<f32>(f32(x), f32(y)) / vp_size;
            // sample the scene texture at that offset
            let sample = textureSample(scene_texture, scene_sampler, uv + offset);

            // Gaussian weight: exp(-(x² + y²)/(2σ²))
            let dist2  = f32(x * x + y * y);
            let weight = exp(-dist2 / twoSigma2);

            sum  = sum + sample * weight;
            wsum = wsum + weight;
        }
    }

    // 4) Normalize the blur and apply your color tint by its alpha
    let blurred = sum / wsum;
    return textureSample(scene_texture, scene_sampler, uv);
}