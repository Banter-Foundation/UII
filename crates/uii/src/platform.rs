//! GLFW, OpenGL 3.3, and Banter's dear-imgui-rs 0.22. Docking is enabled
//! before the first frame. The window thread owns the editor.

use crate::bridge::{self, Job};
use crate::view::View;
use dear_imgui_glow::GlowRenderer;
use dear_imgui_rs::{ConfigFlags, Context, Key};
use glfw::{Action, Context as _, GlfwReceiver, Key as GkKey, Modifiers, WindowEvent};
use glow::HasContext;
use std::rc::Rc;
use std::time::Instant;
use uii_core::Editor;

pub fn run() -> Result<(), String> {
    let mut glfw = glfw::init(glfw::fail_on_errors).map_err(|err| err.to_string())?;
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    glfw.window_hint(glfw::WindowHint::ScaleToMonitor(true));

    let (mut window, events) = glfw
        .create_window(1440, 900, "UII", glfw::WindowMode::Windowed)
        .ok_or("could not open a window")?;
    window.make_current();
    window.set_all_polling(true);
    glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

    let gl = Rc::new(unsafe {
        glow::Context::from_loader_function(|name| match window.get_proc_address(name) {
            Some(address) => std::mem::transmute(address),
            None => std::ptr::null(),
        })
    });

    let mut imgui = Context::create();
    {
        let io = imgui.io_mut();
        io.set_config_flags(io.config_flags() | ConfigFlags::DOCKING_ENABLE);
    }
    let mut renderer = GlowRenderer::with_shared_context(Rc::clone(&gl), &mut imgui)
        .map_err(|err| format!("imgui renderer: {err}"))?;

    let jobs = bridge::start();
    let mut editor = Editor::new();
    let mut view = View::new();
    let plug = uii_cursor::LayoutPlug::bind().ok();
    let snapshot_path = uii_cursor::default_snapshot_path();
    let mut published = u64::MAX;
    let started = Instant::now();
    let mut last = started;

    while !window.should_close() {
        glfw.poll_events();
        feed(&mut imgui, &events);
        while let Ok(job) = jobs.incoming.try_recv() {
            answer(&mut editor, job);
        }

        let now = Instant::now();
        let dt = (now - last).as_secs_f32().clamp(1.0 / 240.0, 0.1);
        last = now;
        let (width, height) = window.get_size();
        let (fb_w, fb_h) = window.get_framebuffer_size();
        let scale_x = if width > 0 {
            fb_w as f32 / width as f32
        } else {
            1.0
        };
        let scale_y = if height > 0 {
            fb_h as f32 / height as f32
        } else {
            1.0
        };
        {
            let io = imgui.io_mut();
            io.set_display_size([width as f32, height as f32]);
            io.set_display_framebuffer_scale([scale_x, scale_y]);
            io.set_delta_time(dt);
        }

        unsafe {
            gl.clear_color(0.12, 0.12, 0.13, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        let ui = imgui.frame();
        view.draw(&ui, &mut editor, dt);
        if editor.revision() != published {
            if let Some(plug) = &plug {
                if let Ok(snapshot) = crate::publish::snapshot(&editor) {
                    let _ = plug.publish_to(snapshot, &snapshot_path);
                }
            }
            published = editor.revision();
        }
        let consumer = renderer
            .renderer_consumer()
            .map_err(|err| format!("imgui consumer: {err}"))?;
        let pending = imgui.render(consumer);
        renderer
            .render(pending)
            .map_err(|err| format!("imgui render: {err}"))?;
        window.swap_buffers();
    }
    view.save_function();
    Ok(())
}

fn answer(editor: &mut Editor, job: Job) {
    let response = editor.cursor(job.request.trim());
    let _ = job.reply.send(response);
}

fn feed(imgui: &mut Context, events: &GlfwReceiver<(f64, WindowEvent)>) {
    let io = imgui.io_mut();
    for (_, event) in glfw::flush_messages(events) {
        match event {
            WindowEvent::CursorPos(x, y) => io.add_mouse_pos_event([x as f32, y as f32]),
            WindowEvent::MouseButton(button, action, _) => {
                let imgui_button = match button {
                    glfw::MouseButtonLeft => dear_imgui_rs::MouseButton::Left,
                    glfw::MouseButtonRight => dear_imgui_rs::MouseButton::Right,
                    glfw::MouseButtonMiddle => dear_imgui_rs::MouseButton::Middle,
                    _ => continue,
                };
                io.add_mouse_button_event(imgui_button, action != Action::Release);
            }
            WindowEvent::Scroll(x, y) => io.add_mouse_wheel_event([x as f32, y as f32]),
            WindowEvent::Char(ch) => io.add_input_character(ch),
            WindowEvent::Key(key, _, action, mods) => {
                let down = action != Action::Release;
                if let Some(mapped) = map_key(key) {
                    io.add_key_event(mapped, down);
                }
                io.add_key_event(Key::ModCtrl, mods.contains(Modifiers::Control));
                io.add_key_event(Key::ModShift, mods.contains(Modifiers::Shift));
                io.add_key_event(Key::ModAlt, mods.contains(Modifiers::Alt));
                io.add_key_event(Key::ModSuper, mods.contains(Modifiers::Super));
            }
            _ => {}
        }
    }
}

fn map_key(key: GkKey) -> Option<Key> {
    Some(match key {
        GkKey::Tab => Key::Tab,
        GkKey::Left => Key::LeftArrow,
        GkKey::Right => Key::RightArrow,
        GkKey::Up => Key::UpArrow,
        GkKey::Down => Key::DownArrow,
        GkKey::PageUp => Key::PageUp,
        GkKey::PageDown => Key::PageDown,
        GkKey::Home => Key::Home,
        GkKey::End => Key::End,
        GkKey::Insert => Key::Insert,
        GkKey::Delete => Key::Delete,
        GkKey::Backspace => Key::Backspace,
        GkKey::Space => Key::Space,
        GkKey::Enter => Key::Enter,
        GkKey::Escape => Key::Escape,
        GkKey::LeftControl | GkKey::RightControl => Key::LeftCtrl,
        GkKey::LeftShift | GkKey::RightShift => Key::LeftShift,
        GkKey::LeftAlt | GkKey::RightAlt => Key::LeftAlt,
        GkKey::LeftSuper | GkKey::RightSuper => Key::LeftSuper,
        GkKey::A => Key::A,
        GkKey::B => Key::B,
        GkKey::C => Key::C,
        GkKey::D => Key::D,
        GkKey::E => Key::E,
        GkKey::F => Key::F,
        GkKey::G => Key::G,
        GkKey::H => Key::H,
        GkKey::I => Key::I,
        GkKey::J => Key::J,
        GkKey::K => Key::K,
        GkKey::L => Key::L,
        GkKey::M => Key::M,
        GkKey::N => Key::N,
        GkKey::O => Key::O,
        GkKey::P => Key::P,
        GkKey::Q => Key::Q,
        GkKey::R => Key::R,
        GkKey::S => Key::S,
        GkKey::T => Key::T,
        GkKey::U => Key::U,
        GkKey::V => Key::V,
        GkKey::W => Key::W,
        GkKey::X => Key::X,
        GkKey::Y => Key::Y,
        GkKey::Z => Key::Z,
        GkKey::Num0 => Key::Key0,
        GkKey::Num1 => Key::Key1,
        GkKey::Num2 => Key::Key2,
        GkKey::Num3 => Key::Key3,
        GkKey::Num4 => Key::Key4,
        GkKey::Num5 => Key::Key5,
        GkKey::Num6 => Key::Key6,
        GkKey::Num7 => Key::Key7,
        GkKey::Num8 => Key::Key8,
        GkKey::Num9 => Key::Key9,
        GkKey::Minus => Key::Minus,
        GkKey::Equal => Key::Equal,
        GkKey::LeftBracket => Key::LeftBracket,
        GkKey::RightBracket => Key::RightBracket,
        GkKey::Backslash => Key::Backslash,
        GkKey::Semicolon => Key::Semicolon,
        GkKey::Apostrophe => Key::Apostrophe,
        GkKey::Comma => Key::Comma,
        GkKey::Period => Key::Period,
        GkKey::Slash => Key::Slash,
        GkKey::GraveAccent => Key::GraveAccent,
        _ => return None,
    })
}
