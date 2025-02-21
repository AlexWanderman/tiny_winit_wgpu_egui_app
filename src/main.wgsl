struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VertexOutput {    
    var vertex = array<VertexOutput, 4>(
        VertexOutput(vec4<f32>(-1.0,  1.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0)), // top    left
        VertexOutput(vec4<f32>(-1.0, -1.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0)), // bottom left
        VertexOutput(vec4<f32>( 1.0,  1.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, 1.0)), // top    right
        VertexOutput(vec4<f32>( 1.0, -1.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, 0.0))  // bottom right
    );

    return vertex[i];
}

@fragment
fn fs_main(vo: VertexOutput) -> @location(0) vec4<f32> {
    let color = vo.color;

    return vec4<f32>(color.r, color.g, color.b, 1.0);
}
