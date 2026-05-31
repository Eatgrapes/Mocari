use rusty_live2d::{
    moc3::{Moc3DrawableMesh, Moc3DrawableVertex},
    render::wgpu::{
        WgpuDrawableVertex, encode_wgpu_indices, encode_wgpu_vertices, wgpu_vertices_from_drawable,
    },
};

#[test]
fn encodes_wgpu_vertices_and_indices() {
    let mesh = Moc3DrawableMesh::from_parts(
        3,
        4,
        0.75,
        20.0,
        vec![
            Moc3DrawableVertex::new([1.0, 2.0], [0.25, 0.5]),
            Moc3DrawableVertex::new([3.0, 4.0], [0.75, 1.0]),
        ],
        vec![0, 1],
        vec![7],
    );

    let vertices = wgpu_vertices_from_drawable(&mesh);
    let vertex_bytes = encode_wgpu_vertices(&vertices);
    let index_bytes = encode_wgpu_indices(mesh.indices());

    assert_eq!(
        vertices,
        vec![
            WgpuDrawableVertex::new([1.0, 2.0], [0.25, 0.5], 0.75),
            WgpuDrawableVertex::new([3.0, 4.0], [0.75, 1.0], 0.75),
        ]
    );
    assert_eq!(vertex_bytes.len(), 40);
    assert_eq!(&vertex_bytes[0..4], &1.0f32.to_ne_bytes());
    assert_eq!(&vertex_bytes[12..16], &0.5f32.to_ne_bytes());
    assert_eq!(&vertex_bytes[16..20], &0.75f32.to_ne_bytes());
    assert_eq!(index_bytes, vec![0, 0, 1, 0]);
}
