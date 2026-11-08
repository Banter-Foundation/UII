//! GLFW window, OpenGL 3.3, and the Dear ImGui docking branch.
//!
//! Same window stack as Banter: `glfw` 0.63, OpenGL 3.3 core, `glow` 0.18,
//! `dear-imgui-rs` 0.22 with `ConfigFlags::DOCKING_ENABLE` before the first frame.
//! Docking is enabled so this context matches Banter. This host does not build a
//! dockspace or a tab bar. The only surface is the Inspect tool.

use std::rc::Rc;

use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::{ConfigFlags, Context, Key, MouseButton};
use glfw::Context as _;
use glow::HasContext;

use crate::inspect::InspectState;

pub(crate) fn run() -> Result<(), String> {
    let mut glfw = glfw::init(glfw_error).map_err(|err| format!("GLFW init failed: {err}"))?;
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    let (mut window, events) = glfw
        .create_window(960, 640, "UII", glfw::WindowMode::Windowed)
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
    // No dock layout to persist. Inspect is not a tab.
    imgui
        .set_ini_filename(None::<&std::path::Path>)
        .map_err(|err| err.to_string())?;

    let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui)
        .map_err(|err| err.to_string())?;

    // Selection is empty until another part of UII sets one. This host does not
    // invent items.
    let state = InspectState::new();
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

        window.make_current();
        unsafe {
            gl.viewport(0, 0, framebuffer_width, framebuffer_height);
            gl.clear_color(0.08, 0.08, 0.10, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        feed_frame_io(
            &mut imgui,
            &window,
            &frame_events,
            FrameInput {
                width,
                height,
                framebuffer_width,
                framebuffer_height,
                delta,
            },
        );

        {
            let ui = imgui.frame();
            state.draw(ui);
        }

        let pending = imgui.render(
            renderer
                .renderer_consumer()
                .map_err(|err| err.to_string())?,
        );
        renderer.render(pending).map_err(|err| err.to_string())?;
        window.swap_buffers();
    }

    Ok(())
}

struct FrameInput {
    width: i32,
    height: i32,
    framebuffer_width: i32,
    framebuffer_height: i32,
    delta: f64,
}

fn feed_frame_io(
    imgui: &mut Context,
    window: &glfw::Window,
    events: &[glfw::WindowEvent],
    frame: FrameInput,
) {
    let focused = window.is_focused();
    let io = imgui.io_mut();
    io.set_display_size([frame.width as f32, frame.height as f32]);
    io.set_delta_time(frame.delta as f32);
    let scale_x = frame.framebuffer_width as f32 / frame.width as f32;
    let scale_y = frame.framebuffer_height as f32 / frame.height as f32;
    io.set_display_framebuffer_scale([scale_x, scale_y]);
    io.add_focus_event(focused);

    if focused {
        let (cursor_x, cursor_y) = window.get_cursor_pos();
        io.add_mouse_pos_event([cursor_x as f32, cursor_y as f32]);
    } else {
        io.add_mouse_pos_event([f32::MIN, f32::MIN]);
    }

    for event in events {
        if focused {
            feed_event(io, event.clone());
        }
    }

    let press = |button| window.get_mouse_button(button) == glfw::Action::Press;
    if focused {
        io.add_mouse_button_event(MouseButton::Left, press(glfw::MouseButton::Button1));
        io.add_mouse_button_event(MouseButton::Right, press(glfw::MouseButton::Button2));
        io.add_mouse_button_event(MouseButton::Middle, press(glfw::MouseButton::Button3));
    } else {
        io.add_mouse_button_event(MouseButton::Left, false);
        io.add_mouse_button_event(MouseButton::Right, false);
        io.add_mouse_button_event(MouseButton::Middle, false);
    }

    let held = |key| window.get_key(key) == glfw::Action::Press;
    if focused {
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
        io.add_key_event(Key::Escape, held(glfw::Key::Escape));
    } else {
        io.add_key_event(Key::ModCtrl, false);
        io.add_key_event(Key::ModShift, false);
        io.add_key_event(Key::ModAlt, false);
        io.add_key_event(Key::ModSuper, false);
        io.add_key_event(Key::Escape, false);
    }
}

fn feed_event(io: &mut dear_imgui_rs::Io, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(key, _, action, _) => {
            if let Some(mapped) = map_key(key) {
                io.add_key_event(mapped, action != glfw::Action::Release);
            }
        }
        glfw::WindowEvent::Char(character) => {
            io.add_input_character(character);
        }
        glfw::WindowEvent::MouseButton(button, action, _) => {
            if let Some(mapped) = map_mouse(button) {
                io.add_mouse_button_event(mapped, action != glfw::Action::Release);
            }
        }
        glfw::WindowEvent::Scroll(x, y) => {
            io.add_mouse_wheel_event([x as f32, y as f32]);
        }
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
        glfw::Key::F1 => Key::F1,
        glfw::Key::F2 => Key::F2,
        glfw::Key::F3 => Key::F3,
        glfw::Key::F4 => Key::F4,
        glfw::Key::F5 => Key::F5,
        glfw::Key::F6 => Key::F6,
        glfw::Key::F7 => Key::F7,
        glfw::Key::F8 => Key::F8,
        glfw::Key::F9 => Key::F9,
        glfw::Key::F10 => Key::F10,
        glfw::Key::F11 => Key::F11,
        glfw::Key::F12 => Key::F12,
        _ => return None,
    })
}

fn glfw_error(error: glfw::Error, description: String) {
    eprintln!("GLFW {error:?}: {description}");
}
