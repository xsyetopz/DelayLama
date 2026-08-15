use std::{mem::size_of, sync::Arc};

use baseview::{
    Event, EventStatus, MouseButton, MouseEvent, Window, WindowHandler, WindowOpenOptions,
    WindowScalePolicy,
};
use bytemuck::{Pod, Zeroable};
use delaylama_editor::{Artwork, EditorModel, HitTarget, SourceRect, ViewTransform};
use image::RgbaImage;
use truce::core::editor::RawWindowHandle;
use truce::params::FloatParamReadF32;
use truce::prelude::{Editor, Params, PluginContext};
use truce_gui::platform::{ParentWindow, create_wgpu_surface, query_backing_scale};
use wgpu::util::DeviceExt;

use crate::plugin_logic::{PadCommand, PluginParams};

const SIZE: (u32, u32) = (360, 510);
const SHADER: &str = r#"
struct VertexOut { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VertexOut {
 var out: VertexOut; out.position = vec4<f32>(position, 0.0, 1.0); out.uv = uv; return out;
}
@group(0) @binding(0) var texture_image: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@fragment fn fs(input: VertexOut) -> @location(0) vec4<f32> {
 return textureSample(texture_image, texture_sampler, input.uv);
}"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

struct Texture {
    bind_group: wgpu::BindGroup,
}

struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    pipeline: wgpu::RenderPipeline,
    textures: Vec<Texture>,
}

impl Renderer {
    #[expect(
        unsafe_code,
        reason = "wgpu requires the baseview child window to outlive its surface"
    )]
    unsafe fn new(window: &Window, width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(truce_gui::platform::editor_instance_descriptor());
        let surface = unsafe { create_wgpu_surface(&instance, window) }?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok()?;
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("delay-lama-editor"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .ok()?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .or_else(|| caps.formats.first().copied())?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("delay-lama-texture-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("delay-lama-quads"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("delay-lama-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("delay-lama-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let artwork = Artwork::ORIGINAL;
        let assets = [
            artwork.source_surface,
            artwork.scene_background,
            artwork.monk_sprite_sheet,
            artwork.control_panel,
            artwork.knob_strip_a,
            artwork.knob_strip_b,
            artwork.ui_arrow,
            artwork.help_panel,
            artwork.ui_tile_a,
            artwork.ui_tile_b,
        ];
        let mut textures = Vec::with_capacity(assets.len());
        for bytes in assets {
            textures.push(Self::upload(&device, &queue, &bind_layout, bytes)?);
        }
        Some(Self {
            device,
            queue,
            surface,
            pipeline,
            textures,
        })
    }

    fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        bytes: &[u8],
    ) -> Option<Texture> {
        let mut image = image::load_from_memory(bytes).ok()?.to_rgba8();
        if image.width() == 20 && image.height() == 17 {
            key_white(&mut image);
        }
        let (width, height) = image.dimensions();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("delay-lama-asset"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("delay-lama-asset-bind"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Some(Texture { bind_group })
    }

    fn render(&mut self, state: &UiState, params: &PluginParams, logical_size: (u32, u32)) {
        let (wgpu::CurrentSurfaceTexture::Success(frame)
        | wgpu::CurrentSurfaceTexture::Suboptimal(frame)) = self.surface.get_current_texture()
        else {
            return;
        };
        let view = frame.texture.create_view(&Default::default());
        let transform = ViewTransform::fit(logical_size.0 as f32, logical_size.1 as f32);
        let mut draws = Vec::new();
        draws.push(quad(
            1,
            rect(0.0, 0.0, 360.0, 311.0),
            [0.0, 0.0, 1.0, 1.0],
            transform,
            logical_size,
        ));
        let animation = params.editor.animation_frame().min(29);
        let column = animation / 6;
        let row = animation % 6;
        draws.push(quad(
            2,
            rect(22.0, 5.0, 314.0, 311.0),
            [
                column as f32 / 5.0,
                row as f32 / 6.0,
                (column + 1) as f32 / 5.0,
                (row + 1) as f32 / 6.0,
            ],
            transform,
            logical_size,
        ));
        draws.push(quad(
            3,
            rect(0.0, 290.0, 360.0, 220.0),
            [0.0, 0.0, 1.0, 1.0],
            transform,
            logical_size,
        ));
        let port = (params.port_time.value().clamp(0.0, 1.0) * 59.0).round() as usize;
        let voice = (params.voice.value().clamp(0.0, 1.0) * 59.0).round() as usize;
        draws.push(quad(
            4,
            rect(21.0, 448.0, 50.0, 50.0),
            strip_uv(port),
            transform,
            logical_size,
        ));
        draws.push(quad(
            5,
            rect(293.0, 447.0, 50.0, 50.0),
            strip_uv(voice),
            transform,
            logical_size,
        ));
        let delay_x = 104.0 + params.delay_mix.value().clamp(0.0, 1.0) * 132.0;
        draws.push(quad(
            6,
            rect(delay_x, 483.0, 20.0, 17.0),
            [0.0, 0.0, 1.0, 1.0],
            transform,
            logical_size,
        ));
        draws.push(quad(
            6,
            rect(96.0 + state.marker.0 * 166.0 - 10.0, 345.0, 20.0, 17.0),
            [0.0, 0.0, 1.0, 1.0],
            transform,
            logical_size,
        ));
        draws.push(quad_rotated(
            6,
            rect(79.0, 362.0 + state.marker.1 * 84.0 - 10.0, 17.0, 20.0),
            [0.0, 0.0, 1.0, 1.0],
            transform,
            logical_size,
        ));
        if state.show_help {
            draws.push(quad(
                7,
                rect(53.5, 117.5, 253.0, 275.0),
                [0.0, 0.0, 1.0, 1.0],
                transform,
                logical_size,
            ));
        }
        let vertices: Vec<Vertex> = draws.iter().flat_map(|draw| draw.1).collect();
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("delay-lama-quads"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("delay-lama-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, buffer.slice(..));
            for (index, draw) in draws.iter().enumerate() {
                pass.set_bind_group(0, &self.textures[draw.0].bind_group, &[]);
                pass.draw((index as u32 * 6)..(index as u32 * 6 + 6), 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

fn key_white(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        if pixel[0] == 255 && pixel[1] == 255 && pixel[2] == 255 {
            pixel[3] = 0;
        }
    }
}
fn rect(x: f32, y: f32, width: f32, height: f32) -> [f32; 4] {
    [x, y, width, height]
}
fn strip_uv(frame: usize) -> [f32; 4] {
    [
        0.0,
        frame.min(59) as f32 / 60.0,
        1.0,
        (frame.min(59) + 1) as f32 / 60.0,
    ]
}
fn quad(
    texture: usize,
    r: [f32; 4],
    uv: [f32; 4],
    transform: ViewTransform,
    view_size: (u32, u32),
) -> (usize, [Vertex; 6]) {
    let (x0, y0) = transform.source_to_view((r[0], r[1]));
    let (x1, y1) = transform.source_to_view((r[0] + r[2], r[1] + r[3]));
    let ndc = |x: f32, y: f32| {
        [
            x / view_size.0 as f32 * 2.0 - 1.0,
            1.0 - y / view_size.1 as f32 * 2.0,
        ]
    };
    let a = Vertex {
        position: ndc(x0, y0),
        uv: [uv[0], uv[1]],
    };
    let b = Vertex {
        position: ndc(x1, y0),
        uv: [uv[2], uv[1]],
    };
    let c = Vertex {
        position: ndc(x1, y1),
        uv: [uv[2], uv[3]],
    };
    let d = Vertex {
        position: ndc(x0, y1),
        uv: [uv[0], uv[3]],
    };
    (texture, [a, b, c, a, c, d])
}

fn quad_rotated(
    texture: usize,
    rect: [f32; 4],
    uv: [f32; 4],
    transform: ViewTransform,
    view_size: (u32, u32),
) -> (usize, [Vertex; 6]) {
    let (texture, mut vertices) = quad(texture, rect, uv, transform, view_size);
    let rotated = [
        [uv[2], uv[1]],
        [uv[2], uv[3]],
        [uv[0], uv[3]],
        [uv[2], uv[1]],
        [uv[0], uv[3]],
        [uv[0], uv[1]],
    ];
    for (vertex, mapped) in vertices.iter_mut().zip(rotated) {
        vertex.uv = mapped;
    }
    (texture, vertices)
}

#[derive(Default)]
struct UiState {
    cursor: (f32, f32),
    drag_start: (f32, f32),
    marker: (f32, f32),
    active: Option<usize>,
    origin: f32,
    show_help: bool,
}
struct Handler {
    renderer: Option<Renderer>,
    params: Arc<PluginParams>,
    context: PluginContext<PluginParams>,
    state: UiState,
    logical_size: (u32, u32),
}
impl WindowHandler for Handler {
    fn on_frame(&mut self, _window: &mut Window) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.render(&self.state, &self.params, self.logical_size);
        }
    }
    fn on_event(&mut self, _window: &mut Window, event: Event) -> EventStatus {
        match event {
            Event::Mouse(MouseEvent::CursorMoved { position, .. }) => {
                self.state.cursor = (position.x as f32, position.y as f32);
                if let Some(active) = self.state.active {
                    self.drag(active);
                }
                EventStatus::Captured
            }
            Event::Mouse(MouseEvent::ButtonPressed {
                button: MouseButton::Left,
                ..
            }) => {
                self.press();
                EventStatus::Captured
            }
            Event::Mouse(MouseEvent::ButtonReleased {
                button: MouseButton::Left,
                ..
            }) => {
                self.release();
                EventStatus::Captured
            }
            // Leaving the child window must not terminate a held gesture. The
            // button release (or lifecycle events below) owns its completion.
            Event::Window(baseview::WindowEvent::Unfocused)
            | Event::Window(baseview::WindowEvent::WillClose) => {
                self.release();
                EventStatus::Captured
            }
            Event::Window(baseview::WindowEvent::Resized(info)) => {
                self.logical_size = (
                    info.logical_size().width as u32,
                    info.logical_size().height as u32,
                );
                EventStatus::Captured
            }
            _ => EventStatus::Ignored,
        }
    }
}
impl Handler {
    fn source_point(&self) -> (f32, f32) {
        ViewTransform::fit(self.logical_size.0 as f32, self.logical_size.1 as f32)
            .view_to_source(self.state.cursor)
    }
    fn press(&mut self) {
        if self.state.active.is_some() {
            return;
        }
        self.state.drag_start = self.state.cursor;
        let point = self.source_point();
        if self.state.show_help {
            self.state.show_help = false;
            return;
        }
        let ids = PluginParams::param_infos_static();
        match EditorModel::hit_test(point) {
            Some(HitTarget::Pad) => {
                let (x, y) = EditorModel::pad_position(point);
                self.state.marker = (x, y);
                self.params.editor.push(PadCommand::Down(x, y));
                self.state.active = Some(0);
            }
            Some(HitTarget::Portamento) => {
                self.state.active = Some(1);
                self.state.origin = self.params.port_time.value();
                self.context.begin_edit(ids[1].id);
            }
            Some(HitTarget::Delay) => {
                self.state.active = Some(2);
                self.context.begin_edit(ids[2].id);
                self.drag(2);
            }
            Some(HitTarget::Voice) => {
                self.state.active = Some(3);
                self.state.origin = self.params.voice.value();
                self.context.begin_edit(ids[3].id);
            }
            Some(HitTarget::Help) => self.state.show_help = !self.state.show_help,
            None => {}
        }
    }

    fn drag(&mut self, index: usize) {
        let point = self.source_point();
        let ids = PluginParams::param_infos_static();
        match index {
            0 => {
                let (x, y) = EditorModel::pad_position(point);
                self.state.marker = (x, y);
                self.params.editor.push(PadCommand::Drag(x, y));
            }
            2 => self.context.set_param(
                ids[2].id,
                f64::from(EditorModel::linear_value(point.0, SourceRect::DELAY)),
            ),
            1 | 3 => {
                let delta = (
                    self.state.cursor.0 - self.state.drag_start.0,
                    self.state.cursor.1 - self.state.drag_start.1,
                );
                self.context.set_param(
                    ids[index].id,
                    f64::from(EditorModel::rotary_value(self.state.origin, delta)),
                );
            }
            _ => {}
        }
    }
    fn release(&mut self) {
        if let Some(index) = self.state.active.take() {
            if index == 0 {
                self.params.editor.push(PadCommand::Up);
            } else {
                self.context
                    .end_edit(PluginParams::param_infos_static()[index].id);
            }
        }
    }
}

pub(crate) struct RawEditor {
    params: Arc<PluginParams>,
    window: Option<baseview::WindowHandle>,
}
#[expect(
    unsafe_code,
    reason = "truce Editor is Send but baseview's platform window handle is UI-thread confined"
)]
unsafe impl Send for RawEditor {}
impl RawEditor {
    pub(crate) fn new(params: Arc<PluginParams>) -> Self {
        Self {
            params,
            window: None,
        }
    }
}
impl Editor for RawEditor {
    fn size(&self) -> (u32, u32) {
        SIZE
    }
    #[expect(
        unsafe_code,
        reason = "wgpu surface creation is tied to the live baseview child window"
    )]
    fn open(&mut self, parent: RawWindowHandle, context: PluginContext) {
        let typed = context.with_params(Arc::clone(&self.params));
        let scale = query_backing_scale(&parent);
        let physical = (
            truce_gui::to_physical_px(SIZE.0, scale),
            truce_gui::to_physical_px(SIZE.1, scale),
        );
        let params = Arc::clone(&self.params);
        let options = WindowOpenOptions {
            title: String::from("Delay Lama"),
            size: baseview::Size::new(f64::from(SIZE.0), f64::from(SIZE.1)),
            scale: WindowScalePolicy::SystemScaleFactor,
        };
        self.window = Some(Window::open_parented(
            &ParentWindow(parent),
            options,
            move |window| Handler {
                renderer: unsafe { Renderer::new(window, physical.0, physical.1) },
                params,
                context: typed,
                state: UiState {
                    marker: (0.5, 0.5),
                    ..Default::default()
                },
                logical_size: SIZE,
            },
        ));
    }
    fn close(&mut self) {
        if let Some(mut window) = self.window.take() {
            window.close();
        }
    }
    fn can_resize(&self) -> bool {
        false
    }
}
