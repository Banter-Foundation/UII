//! GLFW and Dear ImGui host, matched to Banter: OpenGL 3.3 core, glow 0.18,
//! dear-imgui-rs 0.22 docking branch, docking enabled before the first frame.

use std::path::PathBuf;
use std::rc::Rc;

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::{ClipboardBackend, ConfigFlags, Context, Key, MouseButton};
use glfw::Context as _;
use glow::HasContext as _;

use crate::default_graph_path;
use crate::function_tab::{draw_function_tab, Session};

pub fn run() -> Result<(), String> {
    let path = graph_path();
    let mut session = Session::open(path);
    let mut glfw = glfw::init(glfw_error).map_err(|err| format!("GLFW init failed: {err}"))?;
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    let (mut window, events) = glfw
        .create_window(1440, 900, "UII", glfw::WindowMode::Windowed)
        .ok_or_else(|| "GLFW could not open a window. UII needs a display.".to_string())?;
    window.make_current();
    window.set_all_polling(true);
    glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

    let gl = Rc::new(unsafe {
        glow::Context::from_loader_function(|name| {
            window
                .get_proc_address(name)
                .map(|address| address as *const _)
                .unwrap_or(std::ptr::null())
        })
    });
    let mut imgui = Context::create();
    let flags = imgui.io().config_flags() | ConfigFlags::DOCKING_ENABLE;
    imgui.io_mut().set_config_flags(flags);
    imgui.set_clipboard_backend(GlfwClipboard);
    let ini = session
        .path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("uii-dock.ini");
    imgui
        .set_ini_filename(Some(ini))
        .map_err(|err| err.to_string())?;

    let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui)
        .map_err(|err| err.to_string())?;
    let mut last_time = glfw.get_time();

    while !window.should_close() {
        glfw.poll_events();
        let frame_events: Vec<_> = glfw::flush_messages(&events)
            .map(|(_, event)| event)
            .collect();
        let (width, height) = window.get_size();
        let (framebuffer_width, framebuffer_height) = window.get_framebuffer_size();
        if width <= 0 || height <= 0 {
            window.swap_buffers();
            continue;
        }
        let now = glfw.get_time();
        let delta = (now - last_time).max(1.0 / 1000.0);
        last_time = now;
        feed_frame_io(
            &mut imgui,
            &window,
            &frame_events,
            width,
            height,
            framebuffer_width,
            framebuffer_height,
            delta,
        );
        unsafe {
            gl.viewport(0, 0, framebuffer_width, framebuffer_height);
            gl.clear_color(0.08, 0.08, 0.10, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        let save = {
            let ui = imgui.frame();
            draw_function_tab(ui, &mut session)
        };
        if save {
            if let Err(err) = session.save() {
                session.notice = err;
            }
        }
        let pending = imgui.render(
            renderer
                .renderer_consumer()
                .map_err(|err| err.to_string())?,
        );
        renderer.render(pending).map_err(|err| err.to_string())?;
        window.swap_buffers();
    }
    session.save_if_changed()
}

fn graph_path() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        return PathBuf::from(arg);
    }
    let data_home = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    default_graph_path(data_home.as_deref(), home.as_deref())
}

fn glfw_error(error: glfw::Error, description: String) {
    if error == glfw::Error::FormatUnavailable
        || description.contains("does not provide the window position")
    {
        return;
    }
    eprintln!("GLFW: {description}");
}

fn feed_frame_io(
    imgui: &mut Context,
    window: &glfw::Window,
    events: &[glfw::WindowEvent],
    width: i32,
    height: i32,
    framebuffer_width: i32,
    framebuffer_height: i32,
    delta: f64,
) {
    let io = imgui.io_mut();
    io.set_display_size([width as f32, height as f32]);
    io.set_delta_time(delta as f32);
    let scale_x = framebuffer_width as f32 / width as f32;
    let scale_y = framebuffer_height as f32 / height as f32;
    io.set_display_framebuffer_scale([scale_x, scale_y]);
    io.add_focus_event(window.is_focused());
    if window.is_focused() {
        let (cursor_x, cursor_y) = window.get_cursor_pos();
        io.add_mouse_pos_event([cursor_x as f32, cursor_y as f32]);
    } else {
        io.add_mouse_pos_event([f32::MIN, f32::MIN]);
    }
    for event in events {
        if window.is_focused() {
            feed_event(io, event.clone());
        }
    }
    let press = |button| window.get_mouse_button(button) == glfw::Action::Press;
    if window.is_focused() {
        io.add_mouse_button_event(MouseButton::Left, press(glfw::MouseButton::Button1));
        io.add_mouse_button_event(MouseButton::Right, press(glfw::MouseButton::Button2));
        io.add_mouse_button_event(MouseButton::Middle, press(glfw::MouseButton::Button3));
    }
    let held = |key| window.get_key(key) == glfw::Action::Press;
    if window.is_focused() {
        io.add_key_event(
            Key::ModCtrl,
            held(glfw::Key::LeftControl) || held(glfw::Key::RightControl),
        );
        io.add_key_event(
            Key::ModShift,
            held(glfw::Key::LeftShift) || held(glfw::Key::RightShift),
        );
        io.add_key_event(
            Key::ModAlt,
            held(glfw::Key::LeftAlt) || held(glfw::Key::RightAlt),
        );
        io.add_key_event(
            Key::ModSuper,
            held(glfw::Key::LeftSuper) || held(glfw::Key::RightSuper),
        );
    }
}

fn feed_event(io: &mut dear_imgui_rs::Io, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(key, _, action, _) => {
            if let Some(mapped) = map_key(key) {
                io.add_key_event(mapped, action != glfw::Action::Release);
            }
        }
        glfw::WindowEvent::Char(character) => io.add_input_character(character),
        glfw::WindowEvent::MouseButton(button, action, _) => {
            if let Some(mapped) = map_mouse(button) {
                io.add_mouse_button_event(mapped, action != glfw::Action::Release);
            }
        }
        glfw::WindowEvent::Scroll(x, y) => io.add_mouse_wheel_event([x as f32, y as f32]),
        glfw::WindowEvent::Focus(focused) => io.add_focus_event(focused),
        _ => {}
    }
}

fn map_mouse(button: glfw::MouseButton) -> Option<MouseButton> {
    match button {
        glfw::MouseButton::Button1 => Some(MouseButton::Left),
        glfw::MouseButton::Button2 => Some(MouseButton::Right),
        glfw::MouseButton::Button3 => Some(MouseButton::Middle),
        _ => None,
    }
}

fn map_key(key: glfw::Key) -> Option<Key> {
    Some(match key {
        glfw::Key::Tab => Key::Tab,
        glfw::Key::Left => Key::LeftArrow,
        glfw::Key::Right => Key::RightArrow,
        glfw::Key::Up => Key::UpArrow,
        glfw::Key::Down => Key::DownArrow,
        glfw::Key::PageUp => Key::PageUp,
        glfw::Key::PageDown => Key::PageDown,
        glfw::Key::Home => Key::Home,
        glfw::Key::End => Key::End,
        glfw::Key::Insert => Key::Insert,
        glfw::Key::Delete => Key::Delete,
        glfw::Key::Backspace => Key::Backspace,
        glfw::Key::Space => Key::Space,
        glfw::Key::Enter | glfw::Key::KpEnter => Key::Enter,
        glfw::Key::Escape => Key::Escape,
        glfw::Key::LeftControl => Key::LeftCtrl,
        glfw::Key::LeftShift => Key::LeftShift,
        glfw::Key::LeftAlt => Key::LeftAlt,
        glfw::Key::LeftSuper => Key::LeftSuper,
        glfw::Key::RightControl => Key::RightCtrl,
        glfw::Key::RightShift => Key::RightShift,
        glfw::Key::RightAlt => Key::RightAlt,
        glfw::Key::RightSuper => Key::RightSuper,
        glfw::Key::A => Key::A,
        glfw::Key::B => Key::B,
        glfw::Key::C => Key::C,
        glfw::Key::D => Key::D,
        glfw::Key::E => Key::E,
        glfw::Key::F => Key::F,
        glfw::Key::G => Key::G,
        glfw::Key::H => Key::H,
        glfw::Key::I => Key::I,
        glfw::Key::J => Key::J,
        glfw::Key::K => Key::K,
        glfw::Key::L => Key::L,
        glfw::Key::M => Key::M,
        glfw::Key::N => Key::N,
        glfw::Key::O => Key::O,
        glfw::Key::P => Key::P,
        glfw::Key::Q => Key::Q,
        glfw::Key::R => Key::R,
        glfw::Key::S => Key::S,
        glfw::Key::T => Key::T,
        glfw::Key::U => Key::U,
        glfw::Key::V => Key::V,
        glfw::Key::W => Key::W,
        glfw::Key::X => Key::X,
        glfw::Key::Y => Key::Y,
        glfw::Key::Z => Key::Z,
        glfw::Key::Num0 => Key::Key0,
        glfw::Key::Num1 => Key::Key1,
        glfw::Key::Num2 => Key::Key2,
        glfw::Key::Num3 => Key::Key3,
        glfw::Key::Num4 => Key::Key4,
        glfw::Key::Num5 => Key::Key5,
        glfw::Key::Num6 => Key::Key6,
        glfw::Key::Num7 => Key::Key7,
        glfw::Key::Num8 => Key::Key8,
        glfw::Key::Num9 => Key::Key9,
        _ => return None,
    })
}

struct GlfwClipboard;

impl ClipboardBackend for GlfwClipboard {
    fn get(&mut self) -> Option<String> {
        unsafe {
            let text = glfw::ffi::glfwGetClipboardString(std::ptr::null_mut());
            if text.is_null() {
                None
            } else {
                std::ffi::CStr::from_ptr(text)
                    .to_str()
                    .ok()
                    .map(str::to_string)
            }
        }
    }

    fn set(&mut self, value: &str) {
        let Ok(text) = std::ffi::CString::new(value) else {
            return;
        };
        unsafe { glfw::ffi::glfwSetClipboardString(std::ptr::null_mut(), text.as_ptr()) }
    }
}
