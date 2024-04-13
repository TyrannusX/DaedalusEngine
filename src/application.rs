use winit::{
    dpi::PhysicalPosition, event::{Event, KeyEvent, WindowEvent}, event_loop::EventLoop, keyboard::{KeyCode, PhysicalKey}, window::{Window, WindowBuilder}
};

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    clear_color: wgpu::Color,
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline
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

                // Types of vertices to pass
                buffers: &[],
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
            render_pipeline
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

            // Draws primitives
            render_pass.draw(0..3, 0..1);
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