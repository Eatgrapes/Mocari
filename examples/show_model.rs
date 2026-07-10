use std::{collections::BTreeMap, error::Error, fmt, path::PathBuf, sync::Arc, time::Instant};

use ab_glyph::{Font, FontArc, Glyph, ScaleFont, point};
use mocari::{
    ExpressionManager, ModelRuntime, MotionPlayer,
    assets::{DecodedTexture, load_model_runtime},
    core::Matrix44,
    expression::load_expression,
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
const EXPRESSION_BUTTON_Y: f64 = 124.0;
const PREV_PARAMETER_BUTTON_Y: f64 = 178.0;
const NEXT_PARAMETER_BUTTON_Y: f64 = 232.0;
const RESET_PARAMETER_BUTTON_Y: f64 = 286.0;
const PARAMETER_LABEL_Y: f64 = 346.0;
const PARAMETER_SLIDER_Y: f64 = 386.0;
const MODEL_SCALE_LABEL_Y: f64 = 426.0;
const MODEL_SCALE_SLIDER_Y: f64 = 466.0;
const BUTTON_WIDTH: f64 = 168.0;
const BUTTON_HEIGHT: f64 = 42.0;
const BUTTON_RGBA: &[u8] = &[46, 65, 78, 235];
const SLIDER_X: f64 = 16.0;
const SLIDER_WIDTH: f64 = 260.0;
const SLIDER_HEIGHT: f64 = 18.0;
const SLIDER_TRACK_RGBA: &[u8] = &[78, 90, 98, 235];
const SLIDER_FILL_RGBA: &[u8] = &[76, 149, 208, 245];
const TEXT_HEIGHT_PX: f32 = 22.0;
const TEXT_RGBA: [u8; 4] = [232, 238, 242, 255];
const MODEL_SCALE_MIN: f32 = 0.5;
const MODEL_SCALE_MAX: f32 = 2.0;
const MODEL_SCALE_DEFAULT: f32 = 1.0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ButtonAction {
    SwitchModel,
    PlayMotion,
    PlayExpression,
    PreviousParameter,
    NextParameter,
    ResetParameter,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct ButtonSpec {
    action: ButtonAction,
    label: &'static str,
    top: f64,
}

const BUTTON_SPECS: &[ButtonSpec] = &[
    ButtonSpec {
        action: ButtonAction::SwitchModel,
        label: "Switch Model",
        top: SWITCH_BUTTON_Y,
    },
    ButtonSpec {
        action: ButtonAction::PlayMotion,
        label: "Play Motion",
        top: MOTION_BUTTON_Y,
    },
    ButtonSpec {
        action: ButtonAction::PlayExpression,
        label: "Play Expression",
        top: EXPRESSION_BUTTON_Y,
    },
    ButtonSpec {
        action: ButtonAction::PreviousParameter,
        label: "Prev Param",
        top: PREV_PARAMETER_BUTTON_Y,
    },
    ButtonSpec {
        action: ButtonAction::NextParameter,
        label: "Next Param",
        top: NEXT_PARAMETER_BUTTON_Y,
    },
    ButtonSpec {
        action: ButtonAction::ResetParameter,
        label: "Reset Param",
        top: RESET_PARAMETER_BUTTON_Y,
    },
];

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
    event_loop.set_control_flow(ControlFlow::Wait);

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
            Ok(state) => {
                state.window.request_redraw();
                self.state = Some(state);
            }
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
            WindowEvent::CursorMoved { position, .. } => {
                state.cursor_position = Some(position);
                if state.dragging_parameter_slider
                    && let Err(error) = state.update_parameter_slider(position.x)
                {
                    eprintln!("parameter slider failed: {error}");
                    event_loop.exit();
                }
                if state.dragging_model_scale_slider
                    && let Err(error) = state.update_model_scale_slider(position.x)
                {
                    eprintln!("model scale slider failed: {error}");
                    event_loop.exit();
                }
            }
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                if button == MouseButton::Left {
                    let result = match button_state {
                        ElementState::Pressed => state.handle_left_press(),
                        ElementState::Released => {
                            state.dragging_parameter_slider = false;
                            state.dragging_model_scale_slider = false;
                            Ok(())
                        }
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
        if let Some(state) = self.state.as_ref()
            && state.needs_continuous_redraw()
        {
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
    button_uis: Vec<ButtonUi>,
    ui_transform: WgpuTransform,
    parameter_label: LabelQuad,
    slider_track_buffers: WgpuMeshBuffers,
    slider_fill_buffers: WgpuMeshBuffers,
    model_scale_label: LabelQuad,
    model_scale_slider_track_buffers: WgpuMeshBuffers,
    model_scale_slider_fill_buffers: WgpuMeshBuffers,
    slider_track_texture: WgpuTexture,
    slider_fill_texture: WgpuTexture,
    fps_label: LabelQuad,
    fps_meter: FpsMeter,
    selected_parameter_index: Option<usize>,
    model_scale: f32,
    dragging_parameter_slider: bool,
    dragging_model_scale_slider: bool,
    cursor_position: Option<PhysicalPosition<f64>>,
    last_frame: Instant,
    rng: u64,
}

struct LoadedModel {
    runtime: ModelRuntime,
    motions: Vec<PathBuf>,
    motion_groups: BTreeMap<String, Vec<PathBuf>>,
    expressions: Vec<PathBuf>,
    player: Option<MotionPlayer>,
    expression_manager: ExpressionManager,
    dirty: bool,
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

struct ButtonUi {
    transform: WgpuTransform,
    label: LabelQuad,
}

struct FpsMeter {
    sample_frames: u32,
    sample_elapsed: f32,
    total_frames: u64,
    total_elapsed: f32,
    last_presented_at: Option<Instant>,
    label: String,
}

impl FpsMeter {
    fn new() -> Self {
        Self {
            sample_frames: 0,
            sample_elapsed: 0.0,
            total_frames: 0,
            total_elapsed: 0.0,
            last_presented_at: None,
            label: "FPS -- AVG --".to_owned(),
        }
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn record_present(&mut self, now: Instant) -> Option<String> {
        let previous = self.last_presented_at.replace(now)?;
        let delta = now.duration_since(previous).as_secs_f32();
        if delta <= 0.0 {
            return None;
        }
        if delta > 0.25 {
            self.sample_frames = 0;
            self.sample_elapsed = 0.0;
            return None;
        }

        self.sample_frames += 1;
        self.sample_elapsed += delta;
        self.total_frames += 1;
        self.total_elapsed += delta;
        if self.sample_elapsed < 0.5 {
            return None;
        }

        let fps = self.sample_frames as f32 / self.sample_elapsed;
        let average = self.total_frames as f32 / self.total_elapsed;
        self.sample_frames = 0;
        self.sample_elapsed = 0.0;
        let label = format!("FPS {:.0} AVG {:.0}", fps, average);
        if label == self.label {
            None
        } else {
            self.label = label.clone();
            Some(label)
        }
    }
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
                apply_limit_buckets: false,
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
        config.present_mode = preferred_present_mode(&capabilities.present_modes);
        config.desired_maximum_frame_latency = 3;
        surface.configure(&device, &config);

        let renderer = WgpuLive2dRenderer::new(&device, config.format);
        let font = load_font()?;
        let model_index = 0;
        let model_scale = MODEL_SCALE_DEFAULT;
        let model = load_rendered_model(
            &renderer,
            &device,
            &queue,
            MODEL_SPECS[model_index],
            size,
            model_scale,
        )?;
        let button_buffers = create_button_quad_buffers(&device, size, SWITCH_BUTTON_Y)?;
        let button_texture = renderer.create_rgba8_texture(&device, &queue, 1, 1, BUTTON_RGBA)?;
        let button_uis = create_button_uis(&renderer, &device, &queue, &font, size)?;
        let ui_transform = renderer.create_transform(&device, &Matrix44::identity());
        let selected_parameter_index = initial_parameter_selection(&model.runtime);
        let parameter_label = create_parameter_label_quad(
            &renderer,
            &device,
            &queue,
            &font,
            &model.runtime,
            selected_parameter_index,
            size,
        )?;
        let slider_track_buffers = create_slider_track_buffers(&device, size, PARAMETER_SLIDER_Y)?;
        let slider_fill_buffers = create_slider_fill_buffers(
            &device,
            size,
            PARAMETER_SLIDER_Y,
            selected_parameter_normalized(&model.runtime, selected_parameter_index),
        )?;
        let model_scale_label =
            create_model_scale_label_quad(&renderer, &device, &queue, &font, model_scale, size)?;
        let model_scale_slider_track_buffers =
            create_slider_track_buffers(&device, size, MODEL_SCALE_SLIDER_Y)?;
        let model_scale_slider_fill_buffers = create_slider_fill_buffers(
            &device,
            size,
            MODEL_SCALE_SLIDER_Y,
            normalized_model_scale(model_scale),
        )?;
        let slider_track_texture =
            renderer.create_rgba8_texture(&device, &queue, 1, 1, SLIDER_TRACK_RGBA)?;
        let slider_fill_texture =
            renderer.create_rgba8_texture(&device, &queue, 1, 1, SLIDER_FILL_RGBA)?;
        let fps_meter = FpsMeter::new();
        let fps_label =
            create_fps_label_quad(&renderer, &device, &queue, &font, fps_meter.label(), size)?;
        window.set_title(&window_title(MODEL_SPECS[model_index]));

        let now = Instant::now();
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
            button_uis,
            ui_transform,
            parameter_label,
            slider_track_buffers,
            slider_fill_buffers,
            model_scale_label,
            model_scale_slider_track_buffers,
            model_scale_slider_fill_buffers,
            slider_track_texture,
            slider_fill_texture,
            fps_label,
            fps_meter,
            selected_parameter_index,
            model_scale,
            dragging_parameter_slider: false,
            dragging_model_scale_slider: false,
            cursor_position: None,
            last_frame: now,
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
        self.update_model_transform();
        self.button_buffers = create_button_quad_buffers(&self.device, size, SWITCH_BUTTON_Y)?;
        self.button_uis =
            create_button_uis(&self.renderer, &self.device, &self.queue, &self.font, size)?;
        self.slider_track_buffers =
            create_slider_track_buffers(&self.device, size, PARAMETER_SLIDER_Y)?;
        self.model_scale_slider_track_buffers =
            create_slider_track_buffers(&self.device, size, MODEL_SCALE_SLIDER_Y)?;
        self.fps_label = create_fps_label_quad(
            &self.renderer,
            &self.device,
            &self.queue,
            &self.font,
            self.fps_meter.label(),
            size,
        )?;
        self.refresh_parameter_controls()?;
        self.refresh_model_scale_controls()?;
        Ok(())
    }

    fn button_action_at_cursor(&self) -> Option<ButtonAction> {
        let position = self.cursor_position?;
        BUTTON_SPECS
            .iter()
            .find(|spec| button_rect(spec.top).contains(position.x, position.y))
            .map(|spec| spec.action)
    }

    fn parameter_slider_hit(&self) -> bool {
        self.cursor_position
            .map(|position| slider_rect(PARAMETER_SLIDER_Y).contains(position.x, position.y))
            .unwrap_or(false)
    }

    fn model_scale_slider_hit(&self) -> bool {
        self.cursor_position
            .map(|position| slider_rect(MODEL_SCALE_SLIDER_Y).contains(position.x, position.y))
            .unwrap_or(false)
    }

    fn handle_left_press(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(action) = self.button_action_at_cursor() {
            match action {
                ButtonAction::SwitchModel => self.switch_to_next_model(),
                ButtonAction::PlayMotion => self.play_random_motion(),
                ButtonAction::PlayExpression => self.play_random_expression(),
                ButtonAction::PreviousParameter => self.select_previous_parameter(),
                ButtonAction::NextParameter => self.select_next_parameter(),
                ButtonAction::ResetParameter => self.reset_selected_parameter(),
            }
        } else if self.parameter_slider_hit() {
            self.dragging_parameter_slider = true;
            if let Some(position) = self.cursor_position {
                self.update_parameter_slider(position.x)?;
            }
            Ok(())
        } else if self.model_scale_slider_hit() {
            self.dragging_model_scale_slider = true;
            if let Some(position) = self.cursor_position {
                self.update_model_scale_slider(position.x)?;
            }
            Ok(())
        } else {
            self.handle_model_tap()
        }
    }

    fn handle_model_tap(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(position) = self.cursor_position else {
            return Ok(());
        };
        let Some((model_x, model_y)) = cursor_to_model_position(
            position,
            self.window.inner_size(),
            fit_model_matrix_with_scale(
                self.model.model_bounds,
                self.window.inner_size(),
                self.model_scale,
            ),
        ) else {
            return Ok(());
        };
        let Some(hit_area) = self.model.runtime.hit_test(model_x, model_y) else {
            return Ok(());
        };
        let motion_group = format!("Tap{}", hit_area.name());
        self.play_random_motion_group(&motion_group)
    }

    fn play_random_motion(&mut self) -> Result<(), Box<dyn Error>> {
        if self.model.motions.is_empty() {
            return Ok(());
        }
        let pick = (self.next_rng() % self.model.motions.len() as u64) as usize;
        let motion = load_motion(&self.model.motions[pick])?;
        self.model.player = Some(MotionPlayer::new(motion));
        self.model.dirty = true;
        self.reset_fps_label()?;
        self.last_frame = Instant::now();
        self.window.request_redraw();
        Ok(())
    }

    fn play_random_motion_group(&mut self, group: &str) -> Result<(), Box<dyn Error>> {
        let motion_count = match self.model.motion_groups.get(group) {
            Some(motions) if !motions.is_empty() => motions.len(),
            _ => return Ok(()),
        };
        let pick = (self.next_rng() % motion_count as u64) as usize;
        let Some(motion_path) = self
            .model
            .motion_groups
            .get(group)
            .and_then(|motions| motions.get(pick))
        else {
            return Ok(());
        };
        let motion = load_motion(motion_path)?;
        self.model.player = Some(MotionPlayer::new_once(motion));
        self.model.dirty = true;
        self.reset_fps_label()?;
        self.last_frame = Instant::now();
        self.window.request_redraw();
        Ok(())
    }

    fn play_random_expression(&mut self) -> Result<(), Box<dyn Error>> {
        if self.model.expressions.is_empty() {
            return Ok(());
        }
        let pick = (self.next_rng() % self.model.expressions.len() as u64) as usize;
        let expression = load_expression(&self.model.expressions[pick])?;
        self.model.expression_manager.play(expression);
        self.model.dirty = true;
        self.reset_fps_label()?;
        self.last_frame = Instant::now();
        self.window.request_redraw();
        Ok(())
    }

    fn select_previous_parameter(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(current) = self.selected_parameter_index {
            self.selected_parameter_index =
                previous_parameter_index(current, self.model.runtime.parameter_ids().len());
            self.refresh_parameter_controls()?;
            self.window.request_redraw();
        }
        Ok(())
    }

    fn select_next_parameter(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(current) = self.selected_parameter_index {
            self.selected_parameter_index =
                next_parameter_index(current, self.model.runtime.parameter_ids().len());
            self.refresh_parameter_controls()?;
            self.window.request_redraw();
        }
        Ok(())
    }

    fn reset_selected_parameter(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(index) = self.selected_parameter_index else {
            return Ok(());
        };
        self.model.runtime.clear_parameter_override_by_index(index);
        if let Some(default) = self.model.runtime.parameter_default_by_index(index) {
            self.model.runtime.set_parameter_by_index(index, default);
        }
        self.model.dirty = true;
        self.refresh_parameter_controls()?;
        self.window.request_redraw();
        Ok(())
    }

    fn update_parameter_slider(&mut self, x: f64) -> Result<(), Box<dyn Error>> {
        let Some(index) = self.selected_parameter_index else {
            return Ok(());
        };
        let value = slider_rect(PARAMETER_SLIDER_Y).normalized_value(x);
        self.model
            .runtime
            .set_parameter_override_normalized_by_index(index, value);
        self.model.dirty = true;
        self.refresh_parameter_controls()?;
        self.window.request_redraw();
        Ok(())
    }

    fn update_model_scale_slider(&mut self, x: f64) -> Result<(), Box<dyn Error>> {
        let normalized = slider_rect(MODEL_SCALE_SLIDER_Y).normalized_value(x);
        self.model_scale = model_scale_from_normalized(normalized);
        self.update_model_transform();
        self.refresh_model_scale_controls()?;
        self.window.request_redraw();
        Ok(())
    }

    fn update_model_transform(&mut self) {
        self.model.transform.update_matrix(
            &self.queue,
            &fit_model_matrix_with_scale(
                self.model.model_bounds,
                self.window.inner_size(),
                self.model_scale,
            ),
        );
    }

    fn refresh_parameter_controls(&mut self) -> Result<(), Box<dyn Error>> {
        let size = self.window.inner_size();
        self.parameter_label = create_parameter_label_quad(
            &self.renderer,
            &self.device,
            &self.queue,
            &self.font,
            &self.model.runtime,
            self.selected_parameter_index,
            size,
        )?;
        self.slider_fill_buffers = create_slider_fill_buffers(
            &self.device,
            size,
            PARAMETER_SLIDER_Y,
            selected_parameter_normalized(&self.model.runtime, self.selected_parameter_index),
        )?;
        Ok(())
    }

    fn refresh_model_scale_controls(&mut self) -> Result<(), Box<dyn Error>> {
        let size = self.window.inner_size();
        self.model_scale_label = create_model_scale_label_quad(
            &self.renderer,
            &self.device,
            &self.queue,
            &self.font,
            self.model_scale,
            size,
        )?;
        self.model_scale_slider_fill_buffers = create_slider_fill_buffers(
            &self.device,
            size,
            MODEL_SCALE_SLIDER_Y,
            normalized_model_scale(self.model_scale),
        )?;
        Ok(())
    }

    fn next_rng(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn needs_continuous_redraw(&self) -> bool {
        self.model.player.is_some() || self.model.expression_manager.active_expression_count() > 0
    }

    fn reset_fps_label(&mut self) -> Result<(), Box<dyn Error>> {
        self.fps_meter.reset();
        self.fps_label = create_fps_label_quad(
            &self.renderer,
            &self.device,
            &self.queue,
            &self.font,
            self.fps_meter.label(),
            self.window.inner_size(),
        )?;
        Ok(())
    }

    fn advance_motion(&mut self) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        advance_model_frame(
            &self.renderer,
            &self.device,
            &self.queue,
            &mut self.model,
            delta,
        )?;
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
            self.model_scale,
        )?;

        self.model_index = next_index;
        self.model = model;
        self.fps_meter.reset();
        self.fps_label = create_fps_label_quad(
            &self.renderer,
            &self.device,
            &self.queue,
            &self.font,
            self.fps_meter.label(),
            self.window.inner_size(),
        )?;
        let now = Instant::now();
        self.last_frame = now;
        self.selected_parameter_index = initial_parameter_selection(&self.model.runtime);
        self.refresh_parameter_controls()?;
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
            for button_ui in &self.button_uis {
                self.renderer.draw_with_textures_and_transform(
                    &mut pass,
                    &self.button_buffers,
                    std::slice::from_ref(&self.button_texture),
                    &button_ui.transform,
                )?;
            }
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.slider_track_buffers,
                std::slice::from_ref(&self.slider_track_texture),
                &self.ui_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.slider_fill_buffers,
                std::slice::from_ref(&self.slider_fill_texture),
                &self.ui_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.model_scale_slider_track_buffers,
                std::slice::from_ref(&self.slider_track_texture),
                &self.ui_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.model_scale_slider_fill_buffers,
                std::slice::from_ref(&self.slider_fill_texture),
                &self.ui_transform,
            )?;
            for button_ui in &self.button_uis {
                self.renderer.draw_with_textures_and_transform(
                    &mut pass,
                    &button_ui.label.buffers,
                    std::slice::from_ref(&button_ui.label.texture),
                    &self.ui_transform,
                )?;
            }
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.parameter_label.buffers,
                std::slice::from_ref(&self.parameter_label.texture),
                &self.ui_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.model_scale_label.buffers,
                std::slice::from_ref(&self.model_scale_label.texture),
                &self.ui_transform,
            )?;
            self.renderer.draw_with_textures_and_transform(
                &mut pass,
                &self.fps_label.buffers,
                std::slice::from_ref(&self.fps_label.texture),
                &self.ui_transform,
            )?;
        }

        self.window.pre_present_notify();
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        if let Some(label) = self.fps_meter.record_present(Instant::now()) {
            self.fps_label = create_fps_label_quad(
                &self.renderer,
                &self.device,
                &self.queue,
                &self.font,
                &label,
                self.window.inner_size(),
            )?;
            self.window.request_redraw();
        }
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

fn preferred_present_mode(supported_modes: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox]
        .into_iter()
        .find(|mode| supported_modes.contains(mode))
        .unwrap_or(wgpu::PresentMode::AutoNoVsync)
}

fn initial_parameter_selection(runtime: &ModelRuntime) -> Option<usize> {
    const PREFERRED_PARAMETERS: &[&str] = &[
        "ParamEyeLOpen",
        "ParamEyeROpen",
        "ParamEyeOpen",
        "ParamMouthOpenY",
    ];
    PREFERRED_PARAMETERS
        .iter()
        .find_map(|id| runtime.parameter_index(id))
        .or_else(|| (!runtime.parameter_ids().is_empty()).then_some(0))
}

fn previous_parameter_index(current: usize, count: usize) -> Option<usize> {
    if count == 0 {
        None
    } else {
        Some((current + count - 1) % count)
    }
}

fn next_parameter_index(current: usize, count: usize) -> Option<usize> {
    if count == 0 {
        None
    } else {
        Some((current + 1) % count)
    }
}

fn selected_parameter_normalized(runtime: &ModelRuntime, index: Option<usize>) -> f32 {
    let Some(index) = index else {
        return 0.0;
    };
    runtime
        .parameter_override_normalized_value_by_index(index)
        .or_else(|| runtime.parameter_normalized_value_by_index(index))
        .unwrap_or(0.0)
}

fn parameter_label_text(runtime: &ModelRuntime, index: Option<usize>) -> String {
    let Some(index) = index else {
        return "No parameters".to_owned();
    };
    let Some(info) = runtime.parameter_info_by_index(index) else {
        return "No parameters".to_owned();
    };
    let value = runtime
        .parameter_override_value_by_index(index)
        .unwrap_or_else(|| info.value());
    format!(
        "{} {:.2} [{:.2}..{:.2}]",
        info.id(),
        value,
        info.minimum(),
        info.maximum()
    )
}

fn normalized_model_scale(scale: f32) -> f32 {
    ((scale - MODEL_SCALE_MIN) / (MODEL_SCALE_MAX - MODEL_SCALE_MIN)).clamp(0.0, 1.0)
}

fn model_scale_from_normalized(normalized: f32) -> f32 {
    MODEL_SCALE_MIN + normalized.clamp(0.0, 1.0) * (MODEL_SCALE_MAX - MODEL_SCALE_MIN)
}

fn model_scale_label_text(scale: f32) -> String {
    format!("Model Size {:.0}%", scale * 100.0)
}

fn load_rendered_model(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    spec: ModelSpec,
    surface_size: PhysicalSize<u32>,
    model_scale: f32,
) -> Result<LoadedModel, Box<dyn Error>> {
    let loaded = load_model_runtime(spec.path)?;
    let runtime = loaded.runtime().clone();
    let model_bounds = ModelBounds::from_drawables(runtime.meshes())
        .ok_or(ExampleError("model has no drawable bounds"))?;
    let textures = create_textures(renderer, device, queue, loaded.textures())?;
    let motion_groups = motion_paths_by_group(&runtime, loaded.model_dir());
    let motions = motion_groups.values().flatten().cloned().collect();
    let expressions = expression_paths(&runtime, loaded.model_dir());

    let mut model = LoadedModel {
        runtime,
        motions,
        motion_groups,
        expressions,
        player: None,
        expression_manager: ExpressionManager::new(),
        dirty: false,
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
        transform: renderer.create_transform(
            device,
            &fit_model_matrix_with_scale(model_bounds, surface_size, model_scale),
        ),
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
    model.mesh_buffers = mesh_buffers;
    rebuild_model_clipping(renderer, device, model)?;
    Ok(())
}

fn update_model_gpu(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &mut LoadedModel,
) -> Result<(), Box<dyn Error>> {
    let update = match model
        .mesh_buffers
        .update_drawables(queue, model.runtime.meshes())
    {
        Ok(update) => update,
        Err(_) => return rebuild_model_gpu(renderer, device, model),
    };

    if update.bounds_changed() || update.visibility_changed() {
        update_model_clipping(renderer, device, queue, model)?;
    }
    Ok(())
}

fn advance_model_frame(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &mut LoadedModel,
    delta: f32,
) -> Result<(), Box<dyn Error>> {
    let animating =
        model.player.is_some() || model.expression_manager.active_expression_count() > 0;
    if !model.dirty && !animating {
        return Ok(());
    }

    model.runtime.reset_parameters();
    model.runtime.reset_part_opacities();
    if let Some(player) = model.player.as_mut() {
        player.tick(delta);
        player.apply(&mut model.runtime);
        if player.is_finished() {
            model.player = None;
        }
    }
    model.expression_manager.tick(delta);
    model.expression_manager.apply(&mut model.runtime);
    model.runtime.apply_parameter_overrides();
    model.runtime.apply_pose(delta);
    if model.runtime.update_meshes().is_none() {
        return Err(Box::new(ExampleError("failed to rebuild model meshes")));
    }
    update_model_gpu(renderer, device, queue, model)?;
    model.dirty = false;
    Ok(())
}

fn rebuild_model_clipping(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    model: &mut LoadedModel,
) -> Result<(), Box<dyn Error>> {
    let mesh_buffers = &model.mesh_buffers;
    let mut clipping_plan = WgpuClippingPlan::from_mesh_buffers(mesh_buffers);
    clipping_plan.prepare_single_texture_masks(mesh_buffers)?;
    model.clipping_resources = renderer.create_clipping_resources(device, &clipping_plan)?;
    Ok(())
}

fn update_model_clipping(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &mut LoadedModel,
) -> Result<(), Box<dyn Error>> {
    let mesh_buffers = &model.mesh_buffers;
    let mut clipping_plan = WgpuClippingPlan::from_mesh_buffers(mesh_buffers);
    clipping_plan.prepare_single_texture_masks(mesh_buffers)?;
    if !renderer.update_clipping_resources(queue, &mut model.clipping_resources, &clipping_plan)? {
        model.clipping_resources = renderer.create_clipping_resources(device, &clipping_plan)?;
    }
    Ok(())
}

fn motion_paths_by_group(
    runtime: &ModelRuntime,
    model_dir: Option<&std::path::Path>,
) -> BTreeMap<String, Vec<PathBuf>> {
    let Some(model_dir) = model_dir else {
        return BTreeMap::new();
    };
    runtime
        .model()
        .motions()
        .iter()
        .map(|(group, references)| {
            (
                group.clone(),
                references
                    .iter()
                    .map(|reference| model_dir.join(reference.file()))
                    .collect(),
            )
        })
        .collect()
}

fn expression_paths(runtime: &ModelRuntime, model_dir: Option<&std::path::Path>) -> Vec<PathBuf> {
    let Some(model_dir) = model_dir else {
        return Vec::new();
    };
    runtime
        .model()
        .expressions()
        .iter()
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

fn create_button_uis(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &FontArc,
    surface_size: PhysicalSize<u32>,
) -> Result<Vec<ButtonUi>, Box<dyn Error>> {
    BUTTON_SPECS
        .iter()
        .map(|spec| {
            let transform =
                renderer.create_transform(device, &button_offset_matrix(surface_size, spec.top));
            let label = create_label_quad(
                renderer,
                device,
                queue,
                font,
                spec.label,
                surface_size,
                spec.top,
            )?;
            Ok(ButtonUi { transform, label })
        })
        .collect()
}

fn create_parameter_label_quad(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &FontArc,
    runtime: &ModelRuntime,
    index: Option<usize>,
    surface_size: PhysicalSize<u32>,
) -> Result<LabelQuad, Box<dyn Error>> {
    create_text_label_quad(
        renderer,
        device,
        queue,
        font,
        &parameter_label_text(runtime, index),
        surface_size,
        [BUTTON_X, PARAMETER_LABEL_Y],
    )
}

fn create_model_scale_label_quad(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &FontArc,
    scale: f32,
    surface_size: PhysicalSize<u32>,
) -> Result<LabelQuad, Box<dyn Error>> {
    create_text_label_quad(
        renderer,
        device,
        queue,
        font,
        &model_scale_label_text(scale),
        surface_size,
        [BUTTON_X, MODEL_SCALE_LABEL_Y],
    )
}

fn create_fps_label_quad(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &FontArc,
    text: &str,
    surface_size: PhysicalSize<u32>,
) -> Result<LabelQuad, Box<dyn Error>> {
    let (width, height, rgba) = rasterize_text(font, text);
    let texture = renderer.create_rgba8_texture(device, queue, width, height, &rgba)?;
    let x = (f64::from(surface_size.width) - f64::from(width) - 16.0).max(16.0);
    let mesh = textured_quad_mesh(surface_size, x, 16.0, f64::from(width), f64::from(height))
        .ok_or(ExampleError("invalid fps label size"))?;
    let buffers = WgpuMeshBuffers::from_drawables(device, &[mesh])
        .ok_or(ExampleError("failed to create fps label buffers"))?;

    Ok(LabelQuad { buffers, texture })
}

fn create_text_label_quad(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &FontArc,
    text: &str,
    surface_size: PhysicalSize<u32>,
    position: [f64; 2],
) -> Result<LabelQuad, Box<dyn Error>> {
    let (width, height, rgba) = rasterize_text(font, text);
    let texture = renderer.create_rgba8_texture(device, queue, width, height, &rgba)?;
    let mesh = textured_quad_mesh(
        surface_size,
        position[0],
        position[1],
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

#[derive(Debug, Copy, Clone, PartialEq)]
struct SliderRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl SliderRect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    fn normalized_value(self, x: f64) -> f32 {
        ((x - self.x) / self.width).clamp(0.0, 1.0) as f32
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

fn slider_rect(top: f64) -> SliderRect {
    SliderRect {
        x: SLIDER_X,
        y: top,
        width: SLIDER_WIDTH,
        height: SLIDER_HEIGHT,
    }
}

fn create_button_quad_buffers(
    device: &wgpu::Device,
    surface_size: PhysicalSize<u32>,
    top: f64,
) -> Result<WgpuMeshBuffers, Box<dyn Error>> {
    create_rect_quad_buffers(
        device,
        surface_size,
        BUTTON_X,
        top,
        BUTTON_WIDTH,
        BUTTON_HEIGHT,
    )
}

fn create_slider_track_buffers(
    device: &wgpu::Device,
    surface_size: PhysicalSize<u32>,
    top: f64,
) -> Result<WgpuMeshBuffers, Box<dyn Error>> {
    let slider = slider_rect(top);
    create_rect_quad_buffers(
        device,
        surface_size,
        slider.x,
        slider.y,
        slider.width,
        slider.height,
    )
}

fn create_slider_fill_buffers(
    device: &wgpu::Device,
    surface_size: PhysicalSize<u32>,
    top: f64,
    normalized: f32,
) -> Result<WgpuMeshBuffers, Box<dyn Error>> {
    let slider = slider_rect(top);
    let width = (slider.width * f64::from(normalized.clamp(0.0, 1.0))).max(1.0);
    create_rect_quad_buffers(
        device,
        surface_size,
        slider.x,
        slider.y,
        width,
        slider.height,
    )
}

fn create_rect_quad_buffers(
    device: &wgpu::Device,
    surface_size: PhysicalSize<u32>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<WgpuMeshBuffers, Box<dyn Error>> {
    let mesh = textured_quad_mesh(surface_size, x, y, width, height)
        .ok_or(ExampleError("invalid rectangle size"))?;
    WgpuMeshBuffers::from_drawables(device, &[mesh])
        .ok_or_else(|| Box::new(ExampleError("failed to create rectangle buffers")).into())
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

fn fit_model_matrix_with_scale(
    bounds: ModelBounds,
    surface_size: PhysicalSize<u32>,
    model_scale: f32,
) -> Matrix44 {
    let aspect = surface_size.width as f32 / surface_size.height as f32;
    let view_fill = MODEL_VIEW_FILL * model_scale.clamp(MODEL_SCALE_MIN, MODEL_SCALE_MAX);
    let fit_x = view_fill / (bounds.width() * aspect);
    let fit_y = view_fill / bounds.height();
    let scale_y = fit_x.min(fit_y);
    let scale_x = scale_y / aspect;

    let mut matrix = Matrix44::identity();
    matrix.scale(scale_x, scale_y);
    matrix.translate(-bounds.center_x() * scale_x, -bounds.center_y() * scale_y);
    matrix
}

fn cursor_to_model_position(
    position: PhysicalPosition<f64>,
    surface_size: PhysicalSize<u32>,
    transform: Matrix44,
) -> Option<(f32, f32)> {
    if surface_size.width == 0 || surface_size.height == 0 {
        return None;
    }

    let clip_x = (position.x as f32 / surface_size.width as f32) * 2.0 - 1.0;
    let clip_y = 1.0 - (position.y as f32 / surface_size.height as f32) * 2.0;
    Some((
        transform.invert_transform_x(clip_x),
        transform.invert_transform_y(clip_y),
    ))
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

        let matrix =
            fit_model_matrix_with_scale(bounds, PhysicalSize::new(100, 100), MODEL_SCALE_DEFAULT);

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

        let matrix =
            fit_model_matrix_with_scale(bounds, PhysicalSize::new(200, 100), MODEL_SCALE_DEFAULT);

        assert_close(matrix.scale_x() * 200.0, matrix.scale_y() * 100.0);
    }

    #[test]
    fn model_scale_normalization_round_trips() {
        assert_close(normalized_model_scale(MODEL_SCALE_MIN), 0.0);
        assert_close(normalized_model_scale(MODEL_SCALE_DEFAULT), 1.0 / 3.0);
        assert_close(normalized_model_scale(MODEL_SCALE_MAX), 1.0);
        assert_close(model_scale_from_normalized(0.0), MODEL_SCALE_MIN);
        assert_close(model_scale_from_normalized(1.0), MODEL_SCALE_MAX);
    }

    #[test]
    fn fit_model_matrix_applies_model_scale() {
        let bounds = ModelBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        };

        let normal = fit_model_matrix_with_scale(bounds, PhysicalSize::new(100, 100), 1.0);
        let large = fit_model_matrix_with_scale(bounds, PhysicalSize::new(100, 100), 2.0);

        assert_close(large.scale_x(), normal.scale_x() * 2.0);
        assert_close(large.scale_y(), normal.scale_y() * 2.0);
    }

    #[test]
    fn cursor_to_model_position_inverts_fit_matrix() {
        let bounds = ModelBounds {
            min_x: -2.0,
            min_y: -1.0,
            max_x: 2.0,
            max_y: 3.0,
        };
        let size = PhysicalSize::new(100, 100);
        let matrix = fit_model_matrix_with_scale(bounds, size, MODEL_SCALE_DEFAULT);
        let center = cursor_to_model_position(PhysicalPosition::new(50.0, 50.0), size, matrix)
            .expect("valid surface size");

        assert_close(center.0, bounds.center_x());
        assert_close(center.1, bounds.center_y());
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

    #[test]
    fn expression_button_sits_below_motion_button() {
        let motion = button_rect(MOTION_BUTTON_Y);
        let expression = button_rect(EXPRESSION_BUTTON_Y);

        assert!(expression.y > motion.y + motion.height - 0.0001);
        assert_eq!(motion.x, expression.x);
    }

    #[test]
    fn expression_paths_follow_model3_references() {
        let loaded = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
        let paths = expression_paths(loaded.runtime(), loaded.model_dir());

        assert_eq!(paths.len(), 8);
        assert_eq!(
            paths[0],
            std::path::PathBuf::from("assets/models/Mao/expressions/exp_01.exp3.json")
        );
        for path in paths {
            assert!(
                path.exists(),
                "missing expression asset: {}",
                path.display()
            );
        }
    }

    #[test]
    fn parameter_buttons_stack_below_expression_button() {
        let expression = button_rect(EXPRESSION_BUTTON_Y);
        let previous = button_rect(PREV_PARAMETER_BUTTON_Y);
        let next = button_rect(NEXT_PARAMETER_BUTTON_Y);
        let reset = button_rect(RESET_PARAMETER_BUTTON_Y);

        assert!(previous.y > expression.y + expression.height - 0.0001);
        assert!(next.y > previous.y + previous.height - 0.0001);
        assert!(reset.y > next.y + next.height - 0.0001);
        assert_eq!(expression.x, previous.x);
    }

    #[test]
    fn initial_parameter_selection_prefers_eye_open_parameter() {
        let loaded = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
        let index = initial_parameter_selection(loaded.runtime()).unwrap();

        assert_eq!(loaded.runtime().parameter_ids()[index], "ParamEyeLOpen");
    }

    #[test]
    fn parameter_index_navigation_wraps() {
        assert_eq!(next_parameter_index(0, 3), Some(1));
        assert_eq!(next_parameter_index(2, 3), Some(0));
        assert_eq!(previous_parameter_index(0, 3), Some(2));
        assert_eq!(previous_parameter_index(1, 3), Some(0));
        assert_eq!(next_parameter_index(0, 0), None);
        assert_eq!(previous_parameter_index(0, 0), None);
    }

    #[test]
    fn slider_position_maps_to_clamped_normalized_value() {
        let slider = slider_rect(PARAMETER_SLIDER_Y);

        assert_close(slider.normalized_value(slider.x - 20.0), 0.0);
        assert_close(
            slider.normalized_value(slider.x + slider.width * 0.25),
            0.25,
        );
        assert_close(slider.normalized_value(slider.x + slider.width + 20.0), 1.0);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.0001);
    }
}
