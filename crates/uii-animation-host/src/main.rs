//! Standalone window for the Animation tab.
//!
//! The editor in `uii` is the host: it calls `draw_animation_tab` and uses the
//! same clip file. This binary opens that tab on its own when you want the
//! timeline without the rest of the editor.

use std::rc::Rc;

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::{
    ClipboardBackend, Condition, ConfigFlags, Context, Key, MouseButton, WindowFlags,
};
use glfw::Context as _;
use glow::HasContext;
use uii_animation::AnimationTab;

fn main() {
    if let Err(err) = run() {
        eprintln!("uii-animation: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = uii_animation::animation_path();
    let loaded = uii_animation::load_animation(&path)?;
    let mut tab = AnimationTab::new();
    tab.project = loaded.project;
    if !loaded.status.is_empty() {
        tab.set_status(loaded.status);
    }

    let frame_limit = std::env::var("UII_ANIMATION_FRAMES")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());

    let mut glfw = glfw::init(glfw_error).map_err(|err| format!("GLFW init failed: {err}"))?;
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    let (mut window, events) = glfw
        .create_window(1440, 900, "UII — Animation", glfw::WindowMode::Windowed)
        .ok_or_else(|| {
            "GLFW could not open a window. The Animation tab needs a display.".to_string()
        })?;
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
    imgui
        .set_ini_filename::<std::path::PathBuf>(None)
        .map_err(|err| err.to_string())?;
    imgui.set_clipboard_backend(GlfwClipboard);
    let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui)
        .map_err(|err| err.to_string())?;

    let mut last_time = glfw.get_time();
    let mut frames = 0u32;
    while !window.should_close() {
        glfw.poll_events();
        let pending: Vec<_> = glfw::flush_messages(&events)
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
        feed_frame(
            &mut imgui,
            &window,
            &pending,
            width,
            height,
            framebuffer_width,
            framebuffer_height,
            delta,
        );
        tab.tick(delta as f32);
        unsafe {
            gl.viewport(0, 0, framebuffer_width, framebuffer_height);
            gl.clear_color(0.08, 0.08, 0.10, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        {
            let ui = imgui.frame();
            ui.window("UII Animation")
                .flags(
                    WindowFlags::NO_DECORATION
                        | WindowFlags::NO_MOVE
                        | WindowFlags::NO_RESIZE
                        | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS,
                )
                .position([0.0, 0.0], Condition::Always)
                .size([width as f32, height as f32], Condition::Always)
                .build(|| uii_animation::draw_animation_tab(ui, &mut tab));
        }
        let pending = imgui.render(
            renderer
                .renderer_consumer()
                .map_err(|err| err.to_string())?,
        );
        renderer.render(pending).map_err(|err| err.to_string())?;
        window.swap_buffers();
        if tab.is_dirty() {
            uii_animation::save_animation(&path, &tab.project)?;
            let _ = tab.take_dirty();
        }
        frames += 1;
        if frame_limit.is_some_and(|limit| frames >= limit) {
            break;
        }
    }
    if tab.is_dirty() {
        uii_animation::save_animation(&path, &tab.project)?;
        let _ = tab.take_dirty();
    }
    Ok(())
}

fn glfw_error(code: glfw::Error, description: String) {
    eprintln!("glfw {code:?}: {description}");
}

fn feed_frame(
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
    io.set_display_framebuffer_scale([
        framebuffer_width as f32 / width as f32,
        framebuffer_height as f32 / height as f32,
    ]);
    io.add_focus_event(window.is_focused());
    if window.is_focused() {
        let (x, y) = window.get_cursor_pos();
        io.add_mouse_pos_event([x as f32, y as f32]);
    } else {
        io.add_mouse_pos_event([f32::MIN, f32::MIN]);
    }
    for event in events {
        if window.is_focused() {
            feed_event(io, event.clone());
        }
    }
    let pressed = |button| window.get_mouse_button(button) == glfw::Action::Press;
    if window.is_focused() {
        io.add_mouse_button_event(MouseButton::Left, pressed(glfw::MouseButton::Button1));
        io.add_mouse_button_event(MouseButton::Right, pressed(glfw::MouseButton::Button2));
        io.add_mouse_button_event(MouseButton::Middle, pressed(glfw::MouseButton::Button3));
        let held = |key| window.get_key(key) == glfw::Action::Press;
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
        glfw::Key::C => Key::C,
        glfw::Key::V => Key::V,
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
