use wgpu::{util::DeviceExt, BufferUsages};
use winit::{
    dpi::PhysicalPosition, event::{Event, KeyEvent, WindowEvent}, event_loop::EventLoop, keyboard::{KeyCode, PhysicalKey}, window::{Window, WindowBuilder}
};

// Represents a vertex (point in 3D space)
// needs to derive Copy so it can be copied into the buffer
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    // The XYZ position array
    position: [f32; 3],

    // The RGB color array
    color: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            // Defines how "wide" the vertex is in memory
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,

            // Each element represents vertex data
            step_mode: wgpu::VertexStepMode::Vertex,

            attributes: &[
                wgpu::VertexAttribute {
                    // Offset (in bytes) before the attributes start
                    offset: 0,

                    // location to store the attribute at (location(0) in this case)
                    shader_location: 0,

                    // Shape of the attribute (corresponds to a 3 element 32-bit float vector)
                    format: wgpu::VertexFormat::Float32x3
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3
                },
            ],
        }
    }
}

// A front facing triangle (to avoid being culled)
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E
];

// Indices to access repeated vertex data efficiently
const INDICES: &[u16] = &[
    0,1,4,
    1,2,4,
    2,3,4,
];

// Represents the application state
struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    clear_color: wgpu::Color,
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl State<'_> {
    async fn new(window: &Window) -> State<'_> {
        let size = window.inner_size();

        // WGPU instance that manages adapters (gpus) and surfaces (to draw on windows)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Rendering surface that will be displayed on a window
        let surface: wgpu::Surface = instance.create_surface(window).expect("failed to create wgpu surface on winit window");

        // GPU handle that allows us to retrieve information related to the GPU
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false
            }
        ).await.expect("failed to create wgpu adapter");

        // Device: actual GPU used for rendering resources
        // Queue: accepts "commands" to the GPU for rendering
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None
        ).await.expect("failed to create device and queue");

        // Cnfigures the rendering surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2
        };
        surface.configure(&device, &config);

        // Configure the render pipeline
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),

            // Configure the vertex stage
            vertex: wgpu::VertexState {
                // The shader module containing the shader source code
                module: &shader,

                // The entry point function to run
                entry_point: "vs_main",

                // Describes the layout of the buffer
                buffers: &[
                    Vertex::desc(),
                ],
            },

            // Configure the fragment stage (this is an Option<> type since it's optional)
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",

                // The color outputs
                targets: &[Some(wgpu::ColorTargetState {
                    // The first output which is just using the surface texture view format
                    format: config.format,

                    // Replace old pixel data with new data each frame
                    blend: Some(wgpu::BlendState::REPLACE),

                    // Write all colors (RGBA)
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),

            // Configures the interpretation of the vertices when converting them into a primitive (such as a triangle)
            primitive: wgpu::PrimitiveState {
                // This means every 3 vertices corresponds to a single traingle
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,

                // These two fields configure the culling (whether or not the primitives show up in the final rendered output)
                // in this case CCW means the triangle is "front facing" if the vertices are arranged in counter clockwise direction
                // and "Back" means to cull triangles that are facing "back"
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),


                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
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

        // Use the device to create a vertex buffer to store vertex data
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX
        });

        // use the device to create a index buffer to store index data, which will be used to access repeated vertex data
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX
        });

        let num_vertices = VERTICES.len() as u32;
        let num_indices = INDICES.len() as u32;

        State {
            surface,
            device,
            queue,
            config,
            size,
            window,

            // Define the default color to use when clearing the texture view
            clear_color: wgpu::Color {
                a: 1.0,
                r: 0.1,
                g: 0.2,
                b: 0.3,
            },

            // The render pipeline
            render_pipeline,

            vertex_buffer,
            num_vertices,
            index_buffer,
            num_indices,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>){
        // if the size changed, update the state
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, position: PhysicalPosition<f64>) -> bool {
        self.clear_color = wgpu::Color {
            a: 1.0,
            r: position.x as f64 / self.size.width as f64,
            g: position.y as f64 / self.size.height as f64,
            b: 0.3,
        };

        true
    }

    fn update(&mut self){
        
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError>{
        // Get the surface texture to render to
        let output = self.surface.get_current_texture().unwrap();

        // Describes the texture we are rendering to
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Creates+encodes commands that are sent to the queue for processing
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // defining an extra block because we want the borrowing of "encoder" to be short lived
        {
            // The render pass (basically runs the graphics pipeline)
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                // Name for the render pass
                label: Some("Render Pass"),

                // Describes where we are going to draw color to (in this case it's the defined texture view)
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // Which texture view to save the color to
                    view: &view,

                    resolve_target: None,

                    // Which color operations to run
                    ops: wgpu::Operations { 
                        // Defines the load operation (in this case it's clearing the screen's color)
                        load: wgpu::LoadOp::Clear(self.clear_color),

                        // Defines the storage operation (in this case we are storing the color render result)
                        store: wgpu::StoreOp::Store
                    }
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None
            });
            
            // Sets the render pipeline for the render pass
            render_pass.set_pipeline(&self.render_pipeline);

            // Set the vertex buffer before drawing
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            // Set the index buffer before drawing
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // Draws primitives
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Publish and process the command
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let window = WindowBuilder::new().build(&event_loop).expect("failed to create window");

    let mut state = State::new(&window).await;
    
    let _ = event_loop.run(move |event, event_loop_window_target|{
        match event {
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested | WindowEvent::KeyboardInput { 
                    event: KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Backspace),
                        ..
                    },
                    ..
                },
                ..
            } => {
                event_loop_window_target.exit();    
            }
            Event::WindowEvent { 
                event: WindowEvent::CursorMoved {
                    position,
                    ..
                },
                ..
            } => {
                state.input(position);
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                state.resize(physical_size);
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                window_id
            } => {
                if window_id == state.window().id() {
                    state.update();
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop_window_target.exit(),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
            }
            Event::AboutToWait =>{
                state.window.request_redraw()
            }
            _ => ()
        }
    }).unwrap();
}