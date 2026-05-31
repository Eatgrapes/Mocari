use rusty_live2d::{
    core::Matrix44,
    moc3::{Moc3DrawableBlendMode, Moc3DrawableMesh, Moc3DrawableVertex},
    render::wgpu::{
        WgpuDrawableVertex, WgpuLive2dRenderer, WgpuMeshBuffers, WgpuRenderError, WgpuTextureError,
        encode_wgpu_indices, encode_wgpu_matrix, encode_wgpu_vertices, live2d_blend_state,
        live2d_wgsl_source, wgpu_vertices_from_drawable,
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

#[test]
fn live2d_wgsl_samples_texture_and_applies_opacity() {
    let source = live2d_wgsl_source();

    assert!(source.contains("@location(0) position: vec2<f32>"));
    assert!(source.contains("@location(1) uv: vec2<f32>"));
    assert!(source.contains("@location(2) opacity: f32"));
    assert!(source.contains("@group(1) @binding(0)"));
    assert!(source.contains("live2d_transform * vec4<f32>(input.position, 0.0, 1.0)"));
    assert!(source.contains("textureSample"));
    assert!(source.contains("let alpha = sample.a * input.opacity"));
    assert!(source.contains("vec4<f32>(sample.rgb * alpha, alpha)"));
}

#[test]
fn encodes_wgpu_transform_matrix() {
    let mut matrix = Matrix44::identity();
    matrix.scale(2.0, 3.0);
    matrix.translate(4.0, 5.0);

    let bytes = encode_wgpu_matrix(&matrix);

    assert_eq!(bytes.len(), 64);
    assert_eq!(&bytes[0..4], &2.0f32.to_ne_bytes());
    assert_eq!(&bytes[20..24], &3.0f32.to_ne_bytes());
    assert_eq!(&bytes[48..52], &4.0f32.to_ne_bytes());
    assert_eq!(&bytes[52..56], &5.0f32.to_ne_bytes());
}

#[test]
fn exposes_live2d_premultiplied_blend_states() {
    let normal = live2d_blend_state(Moc3DrawableBlendMode::Normal);
    assert_eq!(normal.color.src_factor, wgpu::BlendFactor::One);
    assert_eq!(normal.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
    assert_eq!(normal.alpha.src_factor, wgpu::BlendFactor::One);
    assert_eq!(normal.alpha.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);

    let additive = live2d_blend_state(Moc3DrawableBlendMode::Additive);
    assert_eq!(additive.color.src_factor, wgpu::BlendFactor::One);
    assert_eq!(additive.color.dst_factor, wgpu::BlendFactor::One);
    assert_eq!(additive.alpha.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(additive.alpha.dst_factor, wgpu::BlendFactor::One);

    let multiplicative = live2d_blend_state(Moc3DrawableBlendMode::Multiplicative);
    assert_eq!(multiplicative.color.src_factor, wgpu::BlendFactor::Dst);
    assert_eq!(
        multiplicative.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha
    );
    assert_eq!(multiplicative.alpha.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(multiplicative.alpha.dst_factor, wgpu::BlendFactor::One);
}

#[test]
fn creates_pipeline_and_encodes_draw_calls() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mesh = Moc3DrawableMesh::from_parts(
        0,
        0,
        1.0,
        10.0,
        vec![
            Moc3DrawableVertex::new([-0.5, -0.5], [0.0, 1.0]),
            Moc3DrawableVertex::new([0.5, -0.5], [1.0, 1.0]),
            Moc3DrawableVertex::new([0.0, 0.5], [0.5, 0.0]),
        ],
        vec![0, 1, 2],
        vec![],
    );
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = renderer.create_texture_bind_group(&device, &texture_view);

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer.draw(&mut pass, &buffers, &[bind_group]).unwrap();
        assert_eq!(drawn, 1);
    }

    let _ = encoder.finish();
}

#[test]
fn mesh_buffers_expose_stable_draw_order_indices() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let meshes = [
        test_mesh_with_draw_order(0, 30.0),
        test_mesh_with_draw_order(1, 10.0),
        test_mesh_with_draw_order(2, 10.0),
    ];
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();

    assert_eq!(buffers.draw_order_indices(), vec![1, 2, 0]);
}

#[test]
fn draw_returns_error_for_missing_texture_bind_group() {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mesh = test_mesh_with_draw_order(2, 0.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.missing_texture_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.missing_texture_encoder"),
    });

    let error = {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.missing_texture_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        renderer.draw(&mut pass, &buffers, &[]).unwrap_err()
    };

    assert_eq!(error, WgpuRenderError::MissingTexture { texture_index: 2 });
}

#[test]
fn creates_rgba8_texture_with_bind_group() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let texture = renderer
        .create_rgba8_texture(
            &device,
            &queue,
            2,
            2,
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .unwrap();

    assert_eq!(texture.width(), 2);
    assert_eq!(texture.height(), 2);
    let _ = texture.texture();
    let _ = texture.view();
    let _ = texture.bind_group();
}

#[test]
fn draws_with_uploaded_textures() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let mesh = test_mesh_with_draw_order(0, 0.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.uploaded_texture_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.uploaded_texture_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.uploaded_texture_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer
            .draw_with_textures(&mut pass, &buffers, &[texture])
            .unwrap();
        assert_eq!(drawn, 1);
    }

    let _ = encoder.finish();
}

#[test]
fn draws_with_uploaded_textures_and_transform() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let mut matrix = Matrix44::identity();
    matrix.scale(0.5, 0.5);
    let transform = renderer.create_transform(&device, &matrix);
    let mesh = test_mesh_with_draw_order(0, 0.0);
    let buffers = WgpuMeshBuffers::from_drawables(&device, &[mesh]).unwrap();

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.transform_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.transform_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.transform_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer
            .draw_with_textures_and_transform(&mut pass, &buffers, &[texture], &transform)
            .unwrap();
        assert_eq!(drawn, 1);
    }

    let _ = encoder.finish();
}

#[test]
fn draws_additive_and_multiplicative_drawables() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = renderer
        .create_rgba8_texture(&device, &queue, 1, 1, &[255, 255, 255, 255])
        .unwrap();
    let meshes = [
        test_mesh_with_flags(0, 1 << 0, 0.0),
        test_mesh_with_flags(0, 1 << 1, 1.0),
    ];
    let buffers = WgpuMeshBuffers::from_drawables(&device, &meshes).unwrap();
    assert_eq!(
        buffers.drawables()[0].blend_mode(),
        Moc3DrawableBlendMode::Additive
    );
    assert_eq!(
        buffers.drawables()[1].blend_mode(),
        Moc3DrawableBlendMode::Multiplicative
    );

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("live2d.test.blend_pipeline_target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.test.blend_pipeline_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.test.blend_pipeline_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let drawn = renderer
            .draw_with_textures(&mut pass, &buffers, &[texture])
            .unwrap();
        assert_eq!(drawn, 2);
    }

    let _ = encoder.finish();
}

#[test]
fn rejects_rgba8_texture_with_wrong_byte_len() {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let renderer = WgpuLive2dRenderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);

    let error = renderer
        .create_rgba8_texture(&device, &queue, 2, 2, &[0; 15])
        .unwrap_err();

    assert_eq!(
        error,
        WgpuTextureError::InvalidRgbaLength {
            width: 2,
            height: 2,
            expected: 16,
            actual: 15
        }
    );
}

fn test_mesh_with_draw_order(texture_index: u8, draw_order: f32) -> Moc3DrawableMesh {
    test_mesh_with_flags(texture_index, 0, draw_order)
}

fn test_mesh_with_flags(
    texture_index: u8,
    drawable_flags: u8,
    draw_order: f32,
) -> Moc3DrawableMesh {
    Moc3DrawableMesh::from_parts(
        i32::from(texture_index),
        drawable_flags,
        1.0,
        draw_order,
        vec![
            Moc3DrawableVertex::new([-0.5, -0.5], [0.0, 1.0]),
            Moc3DrawableVertex::new([0.5, -0.5], [1.0, 1.0]),
            Moc3DrawableVertex::new([0.0, 0.5], [0.5, 0.0]),
        ],
        vec![0, 1, 2],
        vec![],
    )
}
