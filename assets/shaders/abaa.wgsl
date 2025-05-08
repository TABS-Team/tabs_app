#import bevy_ui::ui_vertex_output UiVertexOutput

struct AbaaMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: AbaaMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    return material.color;
}