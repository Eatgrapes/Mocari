use std::{error::Error, fmt, sync::Arc};

use rusty_live2d::{
    assets::{DecodedTexture, load_model},
    core::Matrix44,
    moc3::Moc3DrawableMesh,
    render::wgpu::{
        WgpuClippingPlan, WgpuClippingResources, WgpuLive2dRenderer, WgpuMaskRenderTarget,
        WgpuMeshBuffers, WgpuTexture, WgpuTransform,
    },
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 900;
const MASK_TEXTURE_SIZE: u32 = 256;
const MODEL_VIEW_FILL: f32 = 1.85;
const DEFAULT_MODEL_PATH: &str = "assets/models/hiyori_free_en/runtime/hiyori_free_t08.model3.json";

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
            .with_title("Live2D - Hiyori")
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
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => state.resize(state.window.inner_size()),
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
    mesh_buffers: WgpuMeshBuffers,
    textures: Vec<WgpuTexture>,
    clipping_resources: WgpuClippingResources,
    mask_target: WgpuMaskRenderTarget,
    transform: WgpuTransform,
    model_bounds: ModelBounds,
}

impl WindowState {
    async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let default_model = load_model(DEFAULT_MODEL_PATH)?;
        let model_bounds = ModelBounds::from_drawables(default_model.meshes())
            .ok_or(ExampleError("default model has no drawable bounds"))?;

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

        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(ExampleError("surface is not supported by this adapter"))?;
        surface.configure(&device, &config);

        let renderer = WgpuLive2dRenderer::new(&device, config.format);
        let mesh_buffers = WgpuMeshBuffers::from_drawables(&device, default_model.meshes())
            .ok_or(ExampleError("failed to create mesh buffers"))?;
        let textures = create_textures(&renderer, &device, &queue, default_model.textures())?;

        let mut clipping_plan = WgpuClippingPlan::from_mesh_buffers(&mesh_buffers);
        clipping_plan.prepare_single_texture_masks(&mesh_buffers)?;
        let clipping_resources = renderer.create_clipping_resources(&device, &clipping_plan)?;
        let mask_target = renderer.create_mask_render_target(&device, MASK_TEXTURE_SIZE)?;
        let transform = renderer.create_transform(&device, &fit_model_matrix(model_bounds, size));

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            mesh_buffers,
            textures,
            clipping_resources,
            mask_target,
            transform,
            model_bounds,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.transform = self
            .renderer
            .create_transform(&self.device, &fit_model_matrix(self.model_bounds, size));
    }

    fn render(&mut self) -> Result<(), Box<dyn Error>> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.resize(self.window.inner_size());
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
                    view: self.mask_target.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
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
                &self.mesh_buffers,
                &self.clipping_resources,
                &self.textures,
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
                &self.mesh_buffers,
                &self.textures,
                &self.clipping_resources,
                &self.mask_target,
                &self.transform,
            )?;
        }

        self.window.pre_present_notify();
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }
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

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.0001);
    }
}
