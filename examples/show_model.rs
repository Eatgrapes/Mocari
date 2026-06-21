use std::{error::Error, fmt, path::PathBuf, sync::Arc, time::Instant};

use ab_glyph::{Font, FontArc, Glyph, ScaleFont, point};
use mocari::{
    ModelRuntime, MotionPlayer,
    assets::{DecodedTexture, load_model_runtime},
    core::Matrix44,
    moc3::{Moc3DrawableMesh, Moc3DrawableVertex},
    motion::load_motion,
    render::wgpu::{
        WgpuClippingPlan, WgpuClippingResources, WgpuLive2dRenderer, WgpuMaskRenderTarget,
        WgpuMeshBuffers, WgpuTexture, WgpuTransform, preferred_surface_format,
    },
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 900;
const MASK_TEXTURE_SIZE: u32 = 256;
const MODEL_VIEW_FILL: f32 = 1.85;
const BUTTON_X: f64 = 16.0;
const SWITCH_BUTTON_Y: f64 = 16.0;
const MOTION_BUTTON_Y: f64 = 70.0;
const BUTTON_WIDTH: f64 = 168.0;
const BUTTON_HEIGHT: f64 = 42.0;
const BUTTON_RGBA: &[u8] = &[46, 65, 78, 235];
const TEXT_HEIGHT_PX: f32 = 22.0;
const TEXT_RGBA: [u8; 4] = [232, 238, 242, 255];

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct ModelSpec {
    name: &'static str,
    path: &'static str,
}

const MODEL_SPECS: &[ModelSpec] = &[
    ModelSpec {
        name: "Wanko",
        path: "assets/models/Wanko/Wanko.model3.json",
    },
    ModelSpec {
        name: "Hiyori",
        path: "assets/models/Hiyori/Hiyori.model3.json",
    },
    ModelSpec {
        name: "Haru",
        path: "assets/models/Haru/Haru.model3.json",
    },
    ModelSpec {
        name: "Ren",
        path: "assets/models/Ren/Ren.model3.json",
    },
    ModelSpec {
        name: "Mark",
        path: "assets/models/Mark/Mark.model3.json",
    },
    ModelSpec {
        name: "Rice",
        path: "assets/models/Rice/Rice.model3.json",
    },
    ModelSpec {
        name: "Natori",
        path: "assets/models/Natori/Natori.model3.json",
    },
    ModelSpec {
        name: "Mao",
        path: "assets/models/Mao/Mao.model3.json",
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = ShowModelApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct ShowModelApp {
    state: Option<WindowState>,
}

impl ApplicationHandler for ShowModelApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(window_title(MODEL_SPECS[0]))
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };

        match pollster::block_on(WindowState::new(window)) {
            Ok(state) => self.state = Some(state),
            Err(error) => {
                eprintln!("failed to initialize renderer: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Err(error) = state.resize(size) {
                    eprintln!("resize failed: {error}");
                    event_loop.exit();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Err(error) = state.resize(state.window.inner_size()) {
                    eprintln!("resize failed: {error}");
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => state.cursor_position = Some(position),
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                if button_state == ElementState::Pressed && button == MouseButton::Left {
                    let result = if state.button_hit(SWITCH_BUTTON_Y) {
                        state.switch_to_next_model()
                    } else if state.button_hit(MOTION_BUTTON_Y) {
                        state.play_random_motion()
                    } else {
                        Ok(())
                    };
                    if let Err(error) = result {
                        eprintln!("button action failed: {error}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.render() {
                    eprintln!("render failed: {error}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

struct WindowState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: WgpuLive2dRenderer,
    font: FontArc,
    model_index: usize,
    model: LoadedModel,
    button_buffers: WgpuMeshBuffers,
    button_texture: WgpuTexture,
    switch_transform: WgpuTransform,
    motion_transform: WgpuTransform,
    switch_label: LabelQuad,
    motion_label: LabelQuad,
    cursor_position: Option<PhysicalPosition<f64>>,
    last_frame: Instant,
    rng: u64,
}

struct LoadedModel {
    runtime: ModelRuntime,
    motions: Vec<PathBuf>,
    player: Option<MotionPlayer>,
    mesh_buffers: WgpuMeshBuffers,
    textures: Vec<WgpuTexture>,
    clipping_resources: WgpuClippingResources,
    mask_target: WgpuMaskRenderTarget,
    transform: WgpuTransform,
    model_bounds: ModelBounds,
}

struct LabelQuad {
    buffers: WgpuMeshBuffers,
    texture: WgpuTexture,
}

impl WindowState {
    async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("show_model.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await?;

        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(ExampleError("surface is not supported by this adapter"))?;
        let capabilities = surface.get_capabilities(&adapter);
        config.format = preferred_surface_format(&capabilities.formats)
            .ok_or(ExampleError("surface exposes no texture formats"))?;
        surface.configure(&device, &config);

        let renderer = WgpuLive2dRenderer::new(&device, config.format);
        let font = load_font()?;
        let model_index = 0;
        let model =
            load_rendered_model(&renderer, &device, &queue, MODEL_SPECS[model_index], size)?;
        let button_buffers = create_button_quad_buffers(&device, size, SWITCH_BUTTON_Y)?;
        let button_texture = renderer.create_rgba8_texture(&device, &queue, 1, 1, BUTTON_RGBA)?;
        let switch_transform = renderer.create_transform(&device, &Matrix44::identity());
        let motion_transform =
            renderer.create_transform(&device, &button_offset_matrix(size, MOTION_BUTTON_Y));
        let switch_label = create_label_quad(
            &renderer,
            &device,
            &queue,
            &font,
            "Switch Model",
            size,
            SWITCH_BUTTON_Y,
        )?;
        let motion_label = create_label_quad(
            &renderer,
            &device,
            &queue,
            &font,
            "Play Motion",
            size,
            MOTION_BUTTON_Y,
        )?;
        window.set_title(&window_title(MODEL_SPECS[model_index]));

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            font,
            model_index,
            model,
            button_buffers,
            button_texture,
            switch_transform,
            motion_transform,
            switch_label,
            motion_label,
            cursor_position: None,
            last_frame: Instant::now(),
            rng: 0x9e37_79b9_7f4a_7c15,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), Box<dyn Error>> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.model.transform = self.renderer.create_transform(
            &self.device,
            &fit_model_matrix(self.model.model_bounds, size),
        );
        self.button_buffers = create_button_quad_buffers(&self.device, size, SWITCH_BUTTON_Y)?;
        self.motion_transform = self
            .renderer
            .create_transform(&self.device, &button_offset_matrix(size, MOTION_BUTTON_Y));
        self.switch_label = create_label_quad(
            &self.renderer,
            &self.device,
            &self.queue,
            &self.font,
            "Switch Model",
            size,
            SWITCH_BUTTON_Y,
        )?;
        self.motion_label = create_label_quad(
            &self.renderer,
            &self.device,
            &self.queue,
            &self.font,
            "Play Motion",
            size,
            MOTION_BUTTON_Y,
        )?;
        Ok(())
    }

    fn button_hit(&self, top: f64) -> bool {
        self.cursor_position
            .map(|position| button_rect(top).contains(position.x, position.y))
            .unwrap_or(false)
    }

    fn play_random_motion(&mut self) -> Result<(), Box<dyn Error>> {
        if self.model.motions.is_empty() {
            return Ok(());
        }
        let pick = (self.next_rng() % self.model.motions.len() as u64) as usize;
        let motion = load_motion(&self.model.motions[pick])?;
        self.model.player = Some(MotionPlayer::new(motion));
        self.last_frame = Instant::now();
        self.window.request_redraw();
        Ok(())
    }

    fn next_rng(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn advance_motion(&mut self) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.model.runtime.reset_parameters();
        self.model.runtime.reset_part_opacities();
        if let Some(player) = self.model.player.as_mut() {
            player.tick(delta);
            player.apply(&mut self.model.runtime);
            if player.is_finished() {
                self.model.player = None;
            }
        }
        self.model.runtime.apply_pose(delta);
        if self.model.runtime.update_meshes().is_none() {
            return Err(Box::new(ExampleError("failed to rebuild model meshes")));
        }
        rebuild_model_gpu(&self.renderer, &self.device, &mut self.model)?;
        Ok(())
    }

    fn switch_to_next_model(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(next_index) = next_model_index(self.model_index, MODEL_SPECS.len()) else {
            return Ok(());
        };
        let spec = MODEL_SPECS[next_index];
        let model = load_rendered_model(
            &self.renderer,
            &self.device,
            &self.queue,
            spec,
            self.window.inner_size(),
        )?;

        self.model_index = next_index;
        self.model = model;
        self.window.set_title(&window_title(spec));
        self.window.request_redraw();
        Ok(())
    }

    fn render(&mut self) -> Result<(), Box<dyn Error>> {
        self.advance_motion()?;

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.resize(self.window.inner_size())?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(Box::new(ExampleError("failed to acquire surface texture")));
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("show_model.encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("show_model.mask_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.model.mask_target.view(),
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

            self.renderer.draw_masks_with_textures(
                &mut pass,
                &self.model.mesh_buffers,
                &self.model.clipping_resources,
                &self.model.textures,
            )?;
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("show_model.main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.09,
                            b: 0.10,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.renderer.draw_with_textures_clipping_and_transform(
                &mut pass,
                &self.model.mesh_buffers,
                &self.model.textures,
                &self.model.clipping_resources,
                &self.model.mask_target,
                &self.model.transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.button_buffers,
                std::slice::from_ref(&self.button_texture),
                &self.switch_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.button_buffers,
                std::slice::from_ref(&self.button_texture),
                &self.motion_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.switch_label.buffers,
                std::slice::from_ref(&self.switch_label.texture),
                &self.switch_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.motion_label.buffers,
                std::slice::from_ref(&self.motion_label.texture),
                &self.switch_transform,
            )?;
        }

        self.window.pre_present_notify();
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }
}

fn window_title(model: ModelSpec) -> String {
    format!("Live2D - {}", model.name)
}

fn next_model_index(current: usize, count: usize) -> Option<usize> {
    if count == 0 {
        None
    } else {
        Some((current + 1) % count)
    }
}

fn load_rendered_model(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    spec: ModelSpec,
    surface_size: PhysicalSize<u32>,
) -> Result<LoadedModel, Box<dyn Error>> {
    let loaded = load_model_runtime(spec.path)?;
    let runtime = loaded.runtime().clone();
    let model_bounds = ModelBounds::from_drawables(runtime.meshes())
        .ok_or(ExampleError("model has no drawable bounds"))?;
    let textures = create_textures(renderer, device, queue, loaded.textures())?;
    let motions = motion_paths(&runtime, loaded.model_dir());

    let mut model = LoadedModel {
        runtime,
        motions,
        player: None,
        mesh_buffers: WgpuMeshBuffers::from_drawables(device, &[])
            .ok_or(ExampleError("failed to create mesh buffers"))?,
        textures,
        clipping_resources: renderer.create_clipping_resources(
            device,
            &WgpuClippingPlan::from_mesh_buffers(
                &WgpuMeshBuffers::from_drawables(device, &[])
                    .ok_or(ExampleError("failed to create mesh buffers"))?,
            ),
        )?,
        mask_target: renderer.create_mask_render_target(device, MASK_TEXTURE_SIZE)?,
        transform: renderer.create_transform(device, &fit_model_matrix(model_bounds, surface_size)),
        model_bounds,
    };
    rebuild_model_gpu(renderer, device, &mut model)?;
    Ok(model)
}

fn rebuild_model_gpu(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    model: &mut LoadedModel,
) -> Result<(), Box<dyn Error>> {
    let mesh_buffers = WgpuMeshBuffers::from_drawables(device, model.runtime.meshes())
        .ok_or(ExampleError("failed to create mesh buffers"))?;
    let mut clipping_plan = WgpuClippingPlan::from_mesh_buffers(&mesh_buffers);
    clipping_plan.prepare_single_texture_masks(&mesh_buffers)?;
    model.clipping_resources = renderer.create_clipping_resources(device, &clipping_plan)?;
    model.mesh_buffers = mesh_buffers;
    Ok(())
}

fn motion_paths(runtime: &ModelRuntime, model_dir: Option<&std::path::Path>) -> Vec<PathBuf> {
    let Some(model_dir) = model_dir else {
        return Vec::new();
    };
    runtime
        .model()
        .motions()
        .values()
        .flatten()
        .map(|reference| model_dir.join(reference.file()))
        .collect()
}

fn load_font() -> Result<FontArc, Box<dyn Error>> {
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(font) = FontArc::try_from_vec(bytes)
        {
            return Ok(font);
        }
    }
    Err(Box::new(ExampleError("no usable system font found")))
}

fn create_label_quad(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &FontArc,
    text: &str,
    surface_size: PhysicalSize<u32>,
    top: f64,
) -> Result<LabelQuad, Box<dyn Error>> {
    let (width, height, rgba) = rasterize_text(font, text);
    let texture = renderer.create_rgba8_texture(device, queue, width, height, &rgba)?;

    let pad_x = BUTTON_X + 14.0;
    let pad_y = top + (BUTTON_HEIGHT - f64::from(height)) * 0.5;
    let mesh = textured_quad_mesh(
        surface_size,
        pad_x,
        pad_y,
        f64::from(width),
        f64::from(height),
    )
    .ok_or(ExampleError("invalid label size"))?;
    let buffers = WgpuMeshBuffers::from_drawables(device, &[mesh])
        .ok_or(ExampleError("failed to create label buffers"))?;

    Ok(LabelQuad { buffers, texture })
}

fn rasterize_text(font: &FontArc, text: &str) -> (u32, u32, Vec<u8>) {
    let scale = TEXT_HEIGHT_PX;
    let ascent = font.as_scaled(scale).ascent();
    let mut pen_x = 2.0f32;
    let baseline = ascent + 2.0;
    let mut placed: Vec<Glyph> = Vec::new();

    for character in text.chars() {
        let glyph = font
            .glyph_id(character)
            .with_scale_and_position(scale, point(pen_x, baseline));
        pen_x += font.as_scaled(scale).h_advance(glyph.id);
        placed.push(glyph);
    }

    let width = (pen_x.ceil() as u32 + 2).max(1);
    let height = (TEXT_HEIGHT_PX.ceil() as u32 + 6).max(1);
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    for glyph in placed {
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                let px = x as i32 + bounds.min.x as i32;
                let py = y as i32 + bounds.min.y as i32;
                if px < 0 || py < 0 || px as u32 >= width || py as u32 >= height {
                    return;
                }
                let index = ((py as u32 * width + px as u32) * 4) as usize;
                let alpha = (coverage * 255.0) as u8;
                if alpha > rgba[index + 3] {
                    rgba[index] = TEXT_RGBA[0];
                    rgba[index + 1] = TEXT_RGBA[1];
                    rgba[index + 2] = TEXT_RGBA[2];
                    rgba[index + 3] = alpha;
                }
            });
        }
    }

    (width, height, rgba)
}

fn create_textures(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: &[DecodedTexture],
) -> Result<Vec<WgpuTexture>, Box<dyn Error>> {
    textures
        .iter()
        .map(|texture| {
            renderer
                .create_rgba8_texture(
                    device,
                    queue,
                    texture.width(),
                    texture.height(),
                    texture.rgba(),
                )
                .map_err(|error| Box::new(error) as Box<dyn Error>)
        })
        .collect()
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct ButtonRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl ButtonRect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

fn button_rect(top: f64) -> ButtonRect {
    ButtonRect {
        x: BUTTON_X,
        y: top,
        width: BUTTON_WIDTH,
        height: BUTTON_HEIGHT,
    }
}

fn create_button_quad_buffers(
    device: &wgpu::Device,
    surface_size: PhysicalSize<u32>,
    top: f64,
) -> Result<WgpuMeshBuffers, Box<dyn Error>> {
    let mesh = textured_quad_mesh(surface_size, BUTTON_X, top, BUTTON_WIDTH, BUTTON_HEIGHT)
        .ok_or(ExampleError("invalid button size"))?;
    WgpuMeshBuffers::from_drawables(device, &[mesh])
        .ok_or_else(|| Box::new(ExampleError("failed to create button buffers")).into())
}

fn button_offset_matrix(surface_size: PhysicalSize<u32>, top: f64) -> Matrix44 {
    let delta = (top - SWITCH_BUTTON_Y) / f64::from(surface_size.height.max(1)) * 2.0;
    let mut matrix = Matrix44::identity();
    matrix.translate(0.0, -delta as f32);
    matrix
}

fn textured_quad_mesh(
    surface_size: PhysicalSize<u32>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<Moc3DrawableMesh> {
    if surface_size.width == 0 || surface_size.height == 0 {
        return None;
    }

    let right = (x + width).min(f64::from(surface_size.width));
    let bottom = (y + height).min(f64::from(surface_size.height));
    if right <= x || bottom <= y {
        return None;
    }

    let left = pixel_x_to_ndc(x, surface_size.width);
    let right = pixel_x_to_ndc(right, surface_size.width);
    let top = pixel_y_to_ndc(y, surface_size.height);
    let bottom = pixel_y_to_ndc(bottom, surface_size.height);

    Some(Moc3DrawableMesh::from_parts(
        0,
        0,
        1.0,
        0.0,
        vec![
            Moc3DrawableVertex::new([left, top], [0.0, 0.0]),
            Moc3DrawableVertex::new([right, top], [1.0, 0.0]),
            Moc3DrawableVertex::new([right, bottom], [1.0, 1.0]),
            Moc3DrawableVertex::new([left, bottom], [0.0, 1.0]),
        ],
        vec![0, 1, 2, 0, 2, 3],
        Vec::new(),
    ))
}

fn pixel_x_to_ndc(x: f64, width: u32) -> f32 {
    ((x / f64::from(width)) * 2.0 - 1.0) as f32
}

fn pixel_y_to_ndc(y: f64, height: u32) -> f32 {
    (1.0 - (y / f64::from(height)) * 2.0) as f32
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct ModelBounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl ModelBounds {
    fn from_drawables(drawables: &[Moc3DrawableMesh]) -> Option<Self> {
        let mut bounds: Option<Self> = None;

        for vertex in drawables.iter().flat_map(Moc3DrawableMesh::vertices) {
            let [x, y] = vertex.position();
            bounds = Some(match bounds {
                Some(bounds) => Self {
                    min_x: bounds.min_x.min(x),
                    min_y: bounds.min_y.min(y),
                    max_x: bounds.max_x.max(x),
                    max_y: bounds.max_y.max(y),
                },
                None => Self {
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                },
            });
        }

        bounds.filter(|bounds| bounds.width() > 0.0 && bounds.height() > 0.0)
    }

    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    fn center_x(self) -> f32 {
        (self.min_x + self.max_x) * 0.5
    }

    fn center_y(self) -> f32 {
        (self.min_y + self.max_y) * 0.5
    }
}

fn fit_model_matrix(bounds: ModelBounds, surface_size: PhysicalSize<u32>) -> Matrix44 {
    let aspect = surface_size.width as f32 / surface_size.height as f32;
    let fit_x = MODEL_VIEW_FILL / (bounds.width() * aspect);
    let fit_y = MODEL_VIEW_FILL / bounds.height();
    let scale_y = fit_x.min(fit_y);
    let scale_x = scale_y / aspect;

    let mut matrix = Matrix44::identity();
    matrix.scale(scale_x, scale_y);
    matrix.translate(-bounds.center_x() * scale_x, -bounds.center_y() * scale_y);
    matrix
}

#[derive(Debug)]
struct ExampleError(&'static str);

impl fmt::Display for ExampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for ExampleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_model_matrix_centers_and_fits_square_surface() {
        let bounds = ModelBounds {
            min_x: -2.0,
            min_y: -1.0,
            max_x: 2.0,
            max_y: 3.0,
        };

        let matrix = fit_model_matrix(bounds, PhysicalSize::new(100, 100));

        assert_close(matrix.transform_x(bounds.center_x()), 0.0);
        assert_close(matrix.transform_y(bounds.center_y()), 0.0);
        assert!(matrix.transform_x(bounds.min_x) >= -1.0);
        assert!(matrix.transform_x(bounds.max_x) <= 1.0);
        assert!(matrix.transform_y(bounds.min_y) >= -1.0);
        assert!(matrix.transform_y(bounds.max_y) <= 1.0);
    }

    #[test]
    fn fit_model_matrix_preserves_pixels_on_wide_surface() {
        let bounds = ModelBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        };

        let matrix = fit_model_matrix(bounds, PhysicalSize::new(200, 100));

        assert_close(matrix.scale_x() * 200.0, matrix.scale_y() * 100.0);
    }

    #[test]
    fn model_specs_point_at_bundled_models() {
        let names = MODEL_SPECS
            .iter()
            .map(|model| model.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "Wanko", "Hiyori", "Haru", "Ren", "Mark", "Rice", "Natori", "Mao"
            ]
        );
        for model in MODEL_SPECS {
            assert!(
                std::path::Path::new(model.path).exists(),
                "missing model asset: {}",
                model.path
            );
        }
    }

    #[test]
    fn model_specs_load_drawable_meshes() {
        for spec in MODEL_SPECS {
            let model = load_model_runtime(spec.path).expect(spec.path);

            assert!(
                !model.runtime().meshes().is_empty(),
                "model has no drawable meshes: {}",
                spec.path
            );
            assert!(
                !model.textures().is_empty(),
                "model has no textures: {}",
                spec.path
            );
        }
    }

    #[test]
    fn next_model_index_advances_and_wraps() {
        assert_eq!(next_model_index(0, 3), Some(1));
        assert_eq!(next_model_index(2, 3), Some(0));
        assert_eq!(next_model_index(0, 0), None);
    }

    #[test]
    fn button_hit_test_uses_top_left_rect() {
        let rect = button_rect(SWITCH_BUTTON_Y);

        assert!(rect.contains(20.0, 20.0));
        assert!(rect.contains(rect.x + rect.width, rect.y + rect.height));
        assert!(!rect.contains(rect.x + rect.width + 1.0, rect.y));
        assert!(!rect.contains(rect.x, rect.y + rect.height + 1.0));
    }

    #[test]
    fn motion_button_sits_below_switch_button() {
        let switch = button_rect(SWITCH_BUTTON_Y);
        let motion = button_rect(MOTION_BUTTON_Y);

        assert!(motion.y > switch.y + switch.height - 0.0001);
        assert_eq!(switch.x, motion.x);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.0001);
    }
}
