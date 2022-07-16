use wgpu::util::DeviceExt;
use winit::{
	event::*,
	event_loop::{ControlFlow, EventLoop},
	window::WindowBuilder,
	window::Window,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
	color: [f32; 3],
	position: [f32; 3],
}

impl Vertex {
	fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
			wgpu::VertexBufferLayout {
					array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
					step_mode: wgpu::VertexStepMode::Vertex,
					attributes: &[
							wgpu::VertexAttribute {
									offset: 0,
									shader_location: 0,
									format: wgpu::VertexFormat::Float32x3,
							},
							wgpu::VertexAttribute {
									offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
									shader_location: 1,
									format: wgpu::VertexFormat::Float32x3,
							}
					]
			}
	}
}

const VERTICES: &[Vertex] = &[
		// Counter clockwise so that they dont get culled! Top, bottom left, bottom right.    
		Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
		Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
		Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
];

struct State {
	surface: wgpu::Surface,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	size: winit::dpi::PhysicalSize<u32>,
	render_pipeline: wgpu::RenderPipeline,
	vertex_buffer: wgpu::Buffer,
	num_vertices: u32,
}

impl State {
	// Creating some of the wgpu types requires async code
	async fn new(window: &Window) -> Self {
		let size = window.inner_size();
		let num_vertices = VERTICES.len() as u32;

		// The instance is a handle to our GPU
		// Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
		let instance = wgpu::Instance::new(wgpu::Backends::all());
		let surface = unsafe { instance.create_surface(window) };
		let adapter = instance.request_adapter(
			&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			},
		).await.unwrap();

		let (device, queue) = adapter.request_device(
			&wgpu::DeviceDescriptor {
				features: wgpu::Features::empty(),
				limits: wgpu::Limits::default(),
				label: None,
			},
			None, // Trace path
		).await.unwrap();

		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface.get_supported_formats(&adapter)[0],
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::Fifo, // vsync
		};
		surface.configure(&device, &config);

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("Shader"),
			source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
		});

		let render_pipeline_layout =
		device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts: &[],
				push_constant_ranges: &[],
		});

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main", 
				buffers: &[
						Vertex::desc(),
				],
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: "fs_main",
				targets: &[Some(wgpu::ColorTargetState {
					format: config.format,
					blend: Some(wgpu::BlendState {
						color: wgpu::BlendComponent::REPLACE,
						alpha: wgpu::BlendComponent::REPLACE,
					}),
					write_mask: wgpu::ColorWrites::ALL,
				})],
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw, // Counter clockwise
				cull_mode: Some(wgpu::Face::Back),
				// Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
				polygon_mode: wgpu::PolygonMode::Fill,
				// Requires Features::DEPTH_CLIP_CONTROL
				unclipped_depth: false,
				// Requires Features::CONSERVATIVE_RASTERIZATION
				conservative: false,
			},
			depth_stencil: None, 
			multisample: wgpu::MultisampleState {
				count: 1, 
				mask: !0, 
				alpha_to_coverage_enabled: false, 
			},
			multiview: None, 
		});

		let vertex_buffer = device.create_buffer_init(
			&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(VERTICES),
				usage: wgpu::BufferUsages::VERTEX,
			}
		);

		Self {
			surface,
			device,
			queue,
			config,
			size,
			render_pipeline,
			vertex_buffer,
			num_vertices,
		}

	}

	fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.size = new_size;
			self.config.width = new_size.width;
			self.config.height = new_size.height;
			self.surface.configure(&self.device, &self.config);
		}
	}

	fn input(&mut self, _event: &WindowEvent) -> bool {
		false
	}

	fn update(&mut self) {
		//pass
	}

	fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		let output = self.surface.get_current_texture()?;
		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color {
							r: 0.1,
							g: 0.2,
							b: 0.3,
							a: 1.0,
						}),
						store: true,
					},
				})],
				depth_stencil_attachment: None,
			});
			
			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
			render_pass.draw(0..self.num_vertices, 0..1);
		}


		// submit will accept anything that implements IntoIter
		self.queue.submit(std::iter::once(encoder.finish()));
		output.present();

		Ok(())
	}
}

pub async fn run() {
	env_logger::init();
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new().build(&event_loop).unwrap();

	let mut state = State::new(&window).await;

	event_loop.run(move |event, _, control_flow| {
		match event {
				Event::WindowEvent {
						ref event,
						window_id,
				} if window_id == window.id() => if !state.input(event) {
						match event {
								WindowEvent::CloseRequested
								| WindowEvent::KeyboardInput {
										input:
												KeyboardInput {
														state: ElementState::Pressed,
														virtual_keycode: Some(VirtualKeyCode::Escape),
														..
												},
										..
								} => *control_flow = ControlFlow::Exit,
								WindowEvent::Resized(physical_size) => {
										state.resize(*physical_size);
								}
								WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
										state.resize(**new_inner_size);
								}
								WindowEvent::CursorMoved { position, .. } => {
									println!("{}, {}", position.x, position.y)
								}
								_ => {}
						}
				}
				Event::RedrawRequested(window_id) if window_id == window.id() => {
						state.update();
						match state.render() {
								Ok(_) => {}
								// Reconfigure the surface if lost
								Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
								// The system is out of memory, we should probably quit
								Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
								// All other errors (Outdated, Timeout) should be resolved by the next frame
								Err(e) => eprintln!("{:?}", e),
						}
				}
				Event::MainEventsCleared => {
						// RedrawRequested will only trigger once, unless we manually
						// request it.
						window.request_redraw();
				}
				_ => {}
		}
});
}