//! GLFW window, OpenGL 3.3, and Banter's Dear ImGui docking build.
//!
//! The frame loop follows `crates/banter/src/platform.rs`: one GLFW window, one
//! Dear ImGui context with docking enabled, and `dear-imgui-glow` for the draw.

use std::rc::Rc;

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::{ClipboardBackend, ConfigFlags, Context, Key, MouseButton};
use glfw::Context as _;
use glow::HasContext;

use crate::draw::{draw_frame, FrameInput};
use crate::session::{data_dir, load_session, save_session};
use crate::theme;

pub(crate) fn run() -> Result<(), String> {
    let data_dir = data_dir();
    std::fs::create_dir_all(&data_dir).map_err(|err| err.to_string())?;
    let mut host = load_session(&data_dir).map_err(|err| err.to_string())?;

    let mut glfw = glfw::init(glfw_error).map_err(|err| format!("GLFW init failed: {err}"))?;
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    let (mut window, events) = glfw
        .create_window(1280, 720, "UII", glfw::WindowMode::Windowed)
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
    imgui
        .set_ini_filename(Some(data_dir.join("uii-dock.ini")))
        .map_err(|err| err.to_string())?;
    theme::apply(imgui.style_mut());
    let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui)
        .map_err(|err| err.to_string())?;

    let mut last_time = glfw.get_time();
    let smoke_frames = smoke_frame_limit();
    let mut frame_index = 0u32;
    while !window.should_close() {
        glfw.poll_events();
        let events: Vec<_> = glfw::flush_messages(&events)
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
        let ctrl =
            key_down(&window, glfw::Key::LeftControl) || key_down(&window, glfw::Key::RightControl);
        let space = key_down(&window, glfw::Key::Space);

        window.make_current();
        unsafe {
            gl.viewport(0, 0, framebuffer_width, framebuffer_height);
            gl.clear_color(
                theme::STAGE_BG[0],
                theme::STAGE_BG[1],
                theme::STAGE_BG[2],
                1.0,
            );
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        feed_frame_io(
            &mut imgui,
            &window,
            &events,
            SurfaceSize {
                width,
                height,
                framebuffer_width,
                framebuffer_height,
            },
            delta,
        );
        {
            let ui = imgui.frame();
            draw_frame(
                ui,
                &mut host,
                FrameInput {
                    width: width as f32,
                    height: height as f32,
                    ctrl,
                    space,
                },
            );
        }
        let pending = imgui.render(
            renderer
                .renderer_consumer()
                .map_err(|err| err.to_string())?,
        );
        renderer.render(pending).map_err(|err| err.to_string())?;
        window.swap_buffers();

        frame_index += 1;
        if smoke_frames.is_some_and(|limit| frame_index >= limit) {
            break;
        }
    }
    if host.is_dirty() {
        save_session(host.data_dir(), &host).map_err(|err| err.to_string())?;
    }
    Ok(())
}

/// `UII_SMOKE_FRAMES` closes the window after this many presented frames.
/// Used to exercise the GLFW loop without a person at the keyboard.
fn smoke_frame_limit() -> Option<u32> {
    std::env::var("UII_SMOKE_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
}

fn key_down(window: &glfw::Window, key: glfw::Key) -> bool {
    window.get_key(key) == glfw::Action::Press
}

struct SurfaceSize {
    width: i32,
    height: i32,
    framebuffer_width: i32,
    framebuffer_height: i32,
}

fn feed_frame_io(
    imgui: &mut Context,
    window: &glfw::Window,
    events: &[glfw::WindowEvent],
    size: SurfaceSize,
    delta: f64,
) {
    let io = imgui.io_mut();
    io.set_display_size([size.width as f32, size.height as f32]);
    io.set_delta_time(delta as f32);
    let scale_x = size.framebuffer_width as f32 / size.width as f32;
    let scale_y = size.framebuffer_height as f32 / size.height as f32;
    io.set_display_framebuffer_scale([scale_x, scale_y]);
    io.add_focus_event(window.is_focused());
    let (cursor_x, cursor_y) = window.get_cursor_pos();
    io.add_mouse_pos_event([cursor_x as f32, cursor_y as f32]);
    for event in events {
        feed_event(io, event.clone());
    }
    let press = |button| window.get_mouse_button(button) == glfw::Action::Press;
    io.add_mouse_button_event(MouseButton::Left, press(glfw::MouseButton::Button1));
    io.add_mouse_button_event(MouseButton::Right, press(glfw::MouseButton::Button2));
    io.add_mouse_button_event(MouseButton::Middle, press(glfw::MouseButton::Button3));
    let held = |key| key_down(window, key);
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

fn glfw_error(error: glfw::Error, description: String) {
    if error == glfw::Error::FormatUnavailable
        || description.contains("does not provide the window position")
    {
        return;
    }
    panic!("GLFW Error: {description}");
}
