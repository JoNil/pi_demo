use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::{run_return::EventLoopExtRunReturn, unix::WindowExtUnix},
    window::WindowBuilder,
};

mod egl;
mod gl;

static CONFIG_ATTRIBS: &[i32] = &[
    egl::EGL_RED_SIZE,
    8,
    egl::EGL_GREEN_SIZE,
    8,
    egl::EGL_BLUE_SIZE,
    8,
    egl::EGL_DEPTH_SIZE,
    8,
    egl::EGL_RENDERABLE_TYPE,
    egl::EGL_OPENGL_ES3_BIT,
    egl::EGL_NONE,
];

static CONTEXT_ATTRIBS: &[i32] = &[egl::EGL_CONTEXT_CLIENT_VERSION, 3, egl::EGL_NONE];

fn main() {
    let display = egl::get_display(egl::EGL_DEFAULT_DISPLAY).unwrap();

    dbg!(display);

    let mut major = 0;
    let mut minor = 0;

    assert!(egl::initialize(display, &mut major, &mut minor));

    dbg!(major, minor);

    assert!(egl::bind_api(egl::EGL_OPENGL_ES_API));

    let config = egl::choose_config(display, CONFIG_ATTRIBS, 1).unwrap();

    dbg!(config);

    let context =
        egl::create_context(display, config, egl::EGL_NO_CONTEXT, CONTEXT_ATTRIBS).unwrap();

    dbg!(context);

    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let surface =
        egl::create_window_surface(display, config, window.xlib_window().unwrap() as _, &[])
            .unwrap();

    assert!(egl::make_current(display, surface, surface, context));

    gl::load_with(|s| egl::get_proc_address(s) as _);

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawRequested(_) => {
                unsafe {
                    gl::ClearColor(1.0, 1.0, 0.0, 1.0);
                    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                }

                egl::swap_buffers(display, surface);
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });

    assert!(egl::destroy_surface(display, surface));
    assert!(egl::destroy_context(display, context));
    assert!(egl::terminate(display));
}
