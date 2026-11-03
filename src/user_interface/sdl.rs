use std::collections::HashSet;
use std::ffi::NulError;

use crate::user_interface::Interface;

use glifrenderer::viewport::Viewport;
use image;
use sdl2::keyboard::Keycode;
use sdl2::video::{GLContext, GLProfile};
use sdl2::EventPump;
use sdl2::{pixels::PixelFormatEnum, surface::Surface, video::Window, Sdl, VideoSubsystem};
use skia_bindings::GrDirectContext;
use skia_safe::RCHandle;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

impl Interface {
    /// Starts SDL and its video subsystem, which on macOS also starts Cocoa (the app menu, the
    /// Dock icon, and the Apple Events that carry documents opened from Finder).
    pub fn init_sdl() -> (Sdl, VideoSubsystem) {
        const WL_ENV: &'static str = "WAYLAND_DISPLAY";
        if let Some(_) = std::env::var_os(WL_ENV) {
            let (k, v) = ("SDL_VIDEODRIVER", "wayland");
            std::env::set_var(k, v);
            log::info!(
                "Setting {k} to {v} as we see in env {}={}. If this fails, set {k} to `x11`!",
                WL_ENV,
                std::env::var(WL_ENV).unwrap()
            );
        }

        // Control-click is the Mac's right click on trackpads and one-button mice. The Select
        // tool's right click sets the rotation pivot, and the Zoom tool's zooms out.
        #[cfg(target_os = "macos")]
        sdl2::hint::set("SDL_MAC_CTRL_CLICK_EMULATE_RIGHT_CLICK", "1");

        let sdl_context = sdl2::init().expect("Failed to initialize sdl2");
        let video_subsystem = sdl_context
            .video()
            .expect("Failed to create sdl video subsystem");
        (sdl_context, video_subsystem)
    }

    /// When macOS opens a document in nglif (double-click in Finder, "Open With", a file dropped
    /// on the Dock icon), the path comes as an Apple Event rather than in argv. SDL turns it into
    /// an SDL_DROPFILE once Cocoa is running, so wait briefly for it before asking the user.
    #[cfg(target_os = "macos")]
    pub fn take_launch_document(sdl_context: &Sdl) -> Option<PathBuf> {
        use sdl2::event::Event;
        use std::time::{Duration, Instant};

        // Only LaunchServices sends those events; started from a shell, nglif gets its file in
        // argv and shouldn't wait for one.
        if !launched_by_launchservices() {
            return None;
        }

        super::macos::watch_for_open_application();
        let mut event_pump = sdl_context.event_pump().ok()?;
        let start = Instant::now();
        // The launch event arrives right after SDL finishes launching Cocoa, about 0.4 s in;
        // the time limit only matters if macOS never sends one.
        while start.elapsed() < Duration::from_secs(3) {
            match event_pump.wait_event_timeout(10) {
                Some(Event::DropFile { filename, .. }) => {
                    log::debug!(
                        "macOS asked us to open {:?} ({:?} after launch)",
                        &filename,
                        start.elapsed()
                    );
                    return Some(PathBuf::from(filename));
                }
                _ if super::macos::opened_without_documents() => {
                    log::debug!(
                        "Opened without a document ({:?} after launch)",
                        start.elapsed()
                    );
                    return None;
                }
                _ => {}
            }
        }
        log::warn!("macOS sent no launch event within {:?}", start.elapsed());
        None
    }

    pub fn initialize_window(
        video_subsystem: VideoSubsystem,
        filename: &str,
        viewport: &mut Viewport,
    ) -> (Window, RCHandle<GrDirectContext>, GLContext) {
        let mut window = video_subsystem
            .window(
                &format!("nglif — {}", filename),
                viewport.winsize.0 as u32,
                viewport.winsize.1 as u32,
            )
            .opengl()
            .position_centered()
            .allow_highdpi()
            .resizable()
            .build()
            .expect("Failed to create SDL Window");

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(3, 3);
        debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
        debug_assert_eq!(gl_attr.context_version(), (3, 3));

        let gl_ctx = window.gl_create_context().unwrap();
        gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);
        let interface = skia_safe::gpu::gl::Interface::new_load_with(|name| {
            if name == "eglGetCurrentDisplay" {
                return std::ptr::null();
            }
            video_subsystem.gl_get_proc_address(name) as *const _
        })
        .expect("Could not create interface");

        let gr_context = skia_safe::gpu::DirectContext::new_gl(Some(interface), None).unwrap();
        video_subsystem.text_input().start();

        let logo = include_bytes!("../../resources/icon.png");

        let mut im = image::load_from_memory_with_format(logo, image::ImageFormat::Png)
            .unwrap()
            .into_rgba8();

        // SDL2's pixel formats are not byte-by-byte, but rather word-by-word, where the words are each
        // 32 bits long. So RGBA8888 means a 32-bit word where 8 bits are R, G, B and A. However,
        // SDL2's words are not big endian, they are little endian, so we need to reverse them.
        im.chunks_exact_mut(4).for_each(|pixel: &mut _| {
            let oldpixel: [u8; 4] = [pixel[0], pixel[1], pixel[2], pixel[3]];
            pixel[0] = oldpixel[3];
            pixel[1] = oldpixel[2];
            pixel[2] = oldpixel[1];
            pixel[3] = oldpixel[0];
        });

        let surface = Surface::from_data(&mut im, 512, 512, 512 * 4, PixelFormatEnum::RGBA8888)
            .expect("Failed to create SDL2 Surface");

        // On macOS the window icon replaces the Dock icon. nglif.app already has a proper one
        // (resources/macos/nglif.icns, shaped the way macOS draws app icons), so only a bare
        // binary (`cargo run`) needs this.
        #[cfg(target_os = "macos")]
        let needs_icon = !running_in_app_bundle();
        #[cfg(not(target_os = "macos"))]
        let needs_icon = true;
        if needs_icon {
            window.set_icon(surface);
        }

        // Started from a terminal (`cargo run`, `make testrun`), a Mac app opens its window behind
        // the terminal, without keyboard focus, unless it asks to be activated. Apps started by
        // LaunchServices (Finder, the Dock, `open`) are activated for us.
        #[cfg(target_os = "macos")]
        if !launched_by_launchservices() {
            window.raise();
        }

        (window, gr_context, gl_ctx)
    }

    pub fn set_window_title(&mut self, title: &str) -> Result<(), NulError> {
        self.sdl_window.set_title(title)
    }

    pub fn get_event_pump(&self) -> EventPump {
        self.sdl_context
            .event_pump()
            .expect("Could not create sdl event pump")
    }

    pub fn get_pressed_keys(&self, event_pump: &EventPump) -> HashSet<Keycode> {
        event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect::<HashSet<Keycode>>()
    }
}

/// Finder, the Dock and `open` start apps through LaunchServices, whose processes are children
/// of launchd; anything started from a shell or an IDE has a different parent.
#[cfg(target_os = "macos")]
fn launched_by_launchservices() -> bool {
    std::os::unix::process::parent_id() == 1
}

#[cfg(target_os = "macos")]
fn running_in_app_bundle() -> bool {
    std::env::current_exe()
        .map(|exe| exe.to_string_lossy().contains(".app/Contents/MacOS/"))
        .unwrap_or(false)
}
