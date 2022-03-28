use xcb::{x, Xid};

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

    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();

    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    let window: x::Window = conn.generate_id();

    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: window,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: 1280,
        height: 720,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: screen.root_visual(),
        // this list must be in same order than `Cw` enum order
        value_list: &[
            x::Cw::BackPixel(screen.black_pixel()),
            x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS),
        ],
    });

    conn.check_request(cookie).unwrap();

    conn.send_request(&x::MapWindow { window });

    let surface =
        egl::create_window_surface(display, config, window.resource_id() as _, &[]).unwrap();

    assert!(egl::make_current(display, surface, surface, context));

    gl::load_with(|s| egl::get_proc_address(s) as _);

    loop {
        unsafe {
            gl::ClearColor(1.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        egl::swap_buffers(display, surface);
    }

    assert!(egl::terminate(display));
}
