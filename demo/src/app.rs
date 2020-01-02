//! Contains the main types a user needs to interact with to configure and run a skulpin app
use skulpin::AppControl;
use skulpin::InputState;
use skulpin::TimeState;
use skulpin::PeriodicEvent;

use skulpin::RendererBuilder;

use skulpin::ImguiManager;
use skulpin::LogicalSize;

use skulpin::CreateRendererError;
use skulpin::CoordinateSystemHelper;

use skulpin::skia_safe;
use skulpin::ash;
use skulpin::winit;
use skulpin::imgui;
use skulpin::imgui_winit_support;

use legion::prelude::*;

/// Represents an error from creating the renderer
#[derive(Debug)]
pub enum AppError {
    CreateRendererError(CreateRendererError),
    VkError(ash::vk::Result),
    WinitError(winit::error::OsError),
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            AppError::CreateRendererError(ref e) => Some(e),
            AppError::VkError(ref e) => Some(e),
            AppError::WinitError(ref e) => Some(e),
        }
    }
}

impl core::fmt::Display for AppError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            AppError::CreateRendererError(ref e) => e.fmt(fmt),
            AppError::VkError(ref e) => e.fmt(fmt),
            AppError::WinitError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<CreateRendererError> for AppError {
    fn from(result: CreateRendererError) -> Self {
        AppError::CreateRendererError(result)
    }
}

impl From<ash::vk::Result> for AppError {
    fn from(result: ash::vk::Result) -> Self {
        AppError::VkError(result)
    }
}

impl From<winit::error::OsError> for AppError {
    fn from(result: winit::error::OsError) -> Self {
        AppError::WinitError(result)
    }
}

pub trait AppHandler {
    /// Called once at start, put one-time init code here
    fn init(
        &mut self,
        world: &mut World,
    );

    /// Called frequently, this is the intended place to put non-rendering logic
    fn update(
        &mut self,
        world: &mut World,
    );

    /// Called frequently, this is the intended place to put drawing code
    fn draw(
        &mut self,
        world: &mut World,
    );

    fn fatal_error(
        &mut self,
        error: &AppError,
    );
}

struct DrawContextInner {
    canvas: *mut skia_safe::Canvas,
    coordinate_system_helper: CoordinateSystemHelper,
}

#[derive(Default)]
pub struct DrawContext {
    inner: std::sync::Mutex<Option<DrawContextInner>>,
}

unsafe impl Send for DrawContext {}
unsafe impl Sync for DrawContext {}

impl DrawContext {
    pub fn begin_draw_context(
        &mut self,
        canvas: &mut skia_safe::Canvas,
        coordinate_system_helper: skulpin::CoordinateSystemHelper,
    ) {
        let mut lock = self.inner.lock().unwrap();
        *lock = Some(DrawContextInner {
            canvas: canvas as *mut skia_safe::Canvas,
            coordinate_system_helper,
        });
    }

    pub fn end_draw_context(&mut self) {
        let mut lock = self.inner.lock().unwrap();
        *lock = None;
    }

    pub fn with_canvas<F>(
        &mut self,
        f: F,
    ) where
        F: FnOnce(&mut skia_safe::Canvas, &CoordinateSystemHelper),
    {
        let lock = self.inner.lock().unwrap();
        let lock_ref = (*lock).as_ref().unwrap();
        //let x = lock_ref.as_ref().unwrap();
        let canvas = unsafe { &mut *lock_ref.canvas };
        (f)(canvas, &lock_ref.coordinate_system_helper);
    }
}

pub struct App {}

impl App {
    /// Runs the app. This is called by `AppBuilder::run`. This does not return because winit does
    /// not return. For consistency, we use the fatal_error() callback on the passed in AppHandler.
    pub fn run<T: 'static + AppHandler>(
        mut app_handler: T,
        logical_size: LogicalSize,
        renderer_builder: &RendererBuilder,
    ) -> ! {
        // Create the event loop
        let event_loop = winit::event_loop::EventLoop::<()>::with_user_event();

        // Create a single window
        let window_result = winit::window::WindowBuilder::new()
            .with_title("Skulpin")
            .with_inner_size(logical_size)
            .build(&event_loop);

        let window = match window_result {
            Ok(window) => window,
            Err(e) => {
                log::warn!("Passing WindowBuilder::build() error to app {}", e);

                let app_error = e.into();
                app_handler.fatal_error(&app_error);

                // Exiting in this way is consistent with how we will exit if we fail within the
                // input loop
                std::process::exit(0);
            }
        };

        let imgui_manager = init_imgui_manager(&window);
        imgui_manager.begin_frame(&window);

        let renderer_result = renderer_builder.build(&window, imgui_manager.clone());
        let mut renderer = match renderer_result {
            Ok(renderer) => renderer,
            Err(e) => {
                log::warn!("Passing RendererBuilder::build() error to app {}", e);

                let app_error = e.into();
                app_handler.fatal_error(&app_error);

                // Exiting in this way is consistent with how we will exit if we fail within the
                // input loop
                std::process::exit(0);
            }
        };

        // To print fps once per second
        let mut print_fps_event = PeriodicEvent::default();

        let universe = Universe::new();
        let mut world = universe.create_world();

        world.resources.insert(imgui_manager);
        world.resources.insert(AppControl::default());
        world.resources.insert(TimeState::new());
        world.resources.insert(InputState::new(&window));
        world.resources.insert(DrawContext::default());

        app_handler.init(&mut world);

        // Pass control of this thread to winit until the app terminates. If this app wants to quit,
        // the update loop should send the appropriate event via the channel
        event_loop.run(move |event, window_target, control_flow| {
            {
                let mut input_state = world.resources.get_mut::<InputState>().unwrap();
                let mut app_control = world.resources.get_mut::<AppControl>().unwrap();
                input_state.handle_winit_event(&mut app_control, &event, window_target);
            }

            {
                let imgui_manager = world.resources.get_mut::<ImguiManager>().unwrap();
                imgui_manager.handle_event(&window, &event);
            }

            match event {
                winit::event::Event::EventsCleared => {
                    {
                        let mut time_state = world.resources.get_mut::<TimeState>().unwrap();
                        time_state.update();

                        if print_fps_event.try_take_event(
                            time_state.current_instant(),
                            std::time::Duration::from_secs(1),
                        ) {
                            log::debug!("fps: {}", time_state.updates_per_second());
                        }
                    }

                    app_handler.update(&mut world);

                    // Call this to mark the start of the next frame (i.e. "key just down" will return false)
                    {
                        let mut input_state = world.resources.get_mut::<InputState>().unwrap();
                        input_state.end_frame();
                    }

                    // Queue a RedrawRequested event.
                    window.request_redraw();
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::RedrawRequested,
                    ..
                } => {
                    let imgui_manager = world.resources.get::<ImguiManager>().unwrap().clone();
                    if let Err(e) = renderer.draw(
                        &window,
                        imgui_manager,
                        |canvas, coordinate_system_helper, _imgui_manager| {
                            world
                                .resources
                                .get_mut::<DrawContext>()
                                .unwrap()
                                .begin_draw_context(canvas, coordinate_system_helper);
                            app_handler.draw(&mut world);
                            world
                                .resources
                                .get_mut::<DrawContext>()
                                .unwrap()
                                .end_draw_context();
                        },
                    ) {
                        log::warn!("Passing Renderer::draw() error to app {}", e);
                        app_handler.fatal_error(&e.into());
                        {
                            let mut app_control = world.resources.get_mut::<AppControl>().unwrap();
                            app_control.enqueue_terminate_process();
                        }
                    }
                }
                _ => {}
            }

            {
                let app_control = world.resources.get::<AppControl>().unwrap();
                if app_control.should_terminate_process() {
                    *control_flow = winit::event_loop::ControlFlow::Exit
                }
            }
        });
    }
}

fn init_imgui(window: &winit::window::Window) -> imgui::Context {
    use imgui::Context;

    let mut imgui = Context::create();
    {
        // Fix incorrect colors with sRGB framebuffer
        fn imgui_gamma_to_linear(col: [f32; 4]) -> [f32; 4] {
            let x = col[0].powf(2.2);
            let y = col[1].powf(2.2);
            let z = col[2].powf(2.2);
            let w = 1.0 - (1.0 - col[3]).powf(2.2);
            [x, y, z, w]
        }

        let style = imgui.style_mut();
        for col in 0..style.colors.len() {
            style.colors[col] = imgui_gamma_to_linear(style.colors[col]);
        }
    }

    imgui.set_ini_filename(None);

    // In the examples we only use integer DPI factors, because the UI can get very blurry
    // otherwise. This might or might not be what you want in a real application.
    let hidpi_factor = window.hidpi_factor().round();
    let font_size = (16.0 * hidpi_factor) as f32;
    imgui.fonts().add_font(&[imgui::FontSource::TtfData {
        data: include_bytes!("../../fonts/mplus-1p-regular.ttf"),
        size_pixels: font_size,
        config: None,
    }]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    return imgui;
}

pub fn init_imgui_manager(window: &winit::window::Window) -> ImguiManager {
    let mut imgui_context = init_imgui(&window);
    let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui_context);

    imgui_platform.attach_window(
        imgui_context.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Rounded,
    );

    ImguiManager::new(imgui_context, imgui_platform)
}
