// src/shader.wgsl

struct Uniforms {
    transform: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Multiplicamos la posición 3D por la matriz de transformación
    out.clip_position = uniforms.transform * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    return out;
}

// VS de debug: interpreta la posición directamente como clip-space (NDC)
@vertex
fn vs_fullscreen(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

