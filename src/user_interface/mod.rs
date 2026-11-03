pub mod egui_manager;
pub mod follow;
pub mod gui;
#[cfg(target_os = "macos")]
mod macos;
pub mod mouse_input;
pub mod popout;
pub mod sdl;

use std::rc::Rc;

use crate::render;
//use crate::user_interface::gui::build_imgui_ui;
use gl;
use glifrenderer::grid::Grid;
use glifrenderer::viewport::Viewport;
use sdl2::video::GLContext;
use skia_bindings::GrDirectContext;
use skia_bindings::GrSurfaceOrigin;
use skia_safe::gpu;
use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::Color;
use skia_safe::ColorType;
use skia_safe::RCHandle;

use crate::editor::Editor;
pub use crate::render::measure::Measure;
pub use crate::user_interface::mouse_input::MouseInfo;

use sdl2::{video::Window as SdlWindow, Sdl, VideoSubsystem};

use self::egui_manager::EguiManager;
use self::gui::build_ui;
use self::gui::window::WindowManager;
pub use self::popout::PopoutWindow;

/* Window */
pub const PAPER_DRAW_GUIDELINES: bool = false;

pub struct Interface {
    prompts: Vec<InputPrompt>,
    sdl_context: Sdl,
    sdl_dpi: f32,

    pub context: Option<(f32, f32)>,
    pub grid: Grid,
    pub measure: Measure,
    pub curvature_vis: bool,
    pub mouse_info: MouseInfo,
    pub viewport: Viewport,

    // OpenGL and Skia
    gl_ctx: GLContext,
    child_gl_ctxs: Vec<GLContext>,
    pub gr_context: RCHandle<GrDirectContext>,
    pub fb_info: FramebufferInfo,

    // Window *must* be dropped at end to prevent a segmentation fault.
    // See <https://github.com/rust-skia/rust-skia/issues/476>.
    sdl_window: SdlWindow,
    child_sdl_windows: Vec<SdlWindow>,
}

fn fb_info() -> FramebufferInfo {
    let mut fboid = 0;
    unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

    FramebufferInfo {
        fboid: fboid.try_into().unwrap(),
        format: skia_safe::gpu::gl::Format::RGBA8.into(),
        ..Default::default()
    }
}

impl Interface {
    pub fn new(sdl: Sdl, video_subsystem: VideoSubsystem, filename: &str) -> Self {
        let mut viewport = Viewport::default();

        let (window, gr_context, gl_ctx) =
            Self::initialize_window(video_subsystem, filename, &mut viewport);

        let fb_info = fb_info();

        #[allow(unused_mut)]
        let mut iself = Interface {
            prompts: vec![],
            sdl_context: sdl,
            sdl_window: window,
            sdl_dpi: f32::NAN,

            context: None,
            grid: Grid::default(),
            measure: Measure {
                start_point: None,
                end_point: None,
                enabled: true,
            },
            curvature_vis: true,
            mouse_info: MouseInfo::default(),
            viewport: Viewport::default(),

            gl_ctx,
            gr_context,
            fb_info,
            child_gl_ctxs: vec![],
            child_sdl_windows: vec![],
        };

        // The window was created 800×800 in points, but the canvas is in pixels: twice that on
        // a Retina display. Measure it now rather than at the first resize, or the glyph starts
        // out drawn around the top-left quarter of the window.
        iself.refresh_window_metrics();

        iself
    }

    pub fn create_surface(&mut self) -> skia_safe::Surface {
        create_surface(&self.sdl_window, &self.fb_info, &mut self.gr_context)
    }

    /// Re-reads the window's pixel density and its size in pixels. Call it whenever SDL reports
    /// a resize; on macOS that includes moving the window to a display with another scale
    /// factor (Retina laptop screen ↔ standard external monitor).
    pub fn refresh_window_metrics(&mut self) {
        let dpi = self.set_dpi_from_os();
        let (width, height) = self.sdl_window.drawable_size();
        self.viewport.winsize = (width as f32, height as f32);
        let (w, h) = self.sdl_window.size();
        log::debug!(
            "Window is {}×{} points, drawn at {}×{} pixels ({} px/pt)",
            w,
            h,
            width,
            height,
            1. / dpi
        );
    }

    pub fn active_prompts(&self) -> bool {
        !self.prompts.is_empty()
    }

    pub fn peek_prompt(&self) -> &InputPrompt {
        &self.prompts.first().unwrap()
    }

    pub fn pop_prompt(&mut self) -> Option<InputPrompt> {
        self.prompts.pop()
    }

    pub fn extents(&mut self) -> (u32, u32) {
        let (window_width, window_height) = self.viewport.winsize;
        let (width, height) = (window_width.ceil() as u32, window_height.ceil() as u32);
        (width, height)
    }

    pub fn render(
        &mut self,
        v: &mut Editor,
        wm: &mut WindowManager,
        egui_manager: &mut EguiManager,
        sk_surface: &mut skia_safe::Surface,
    ) {
        build_ui(egui_manager, v, self, wm);
        let canvas = sk_surface.canvas();
        canvas.clear(Color::BLACK);
        render::render_frame(v, self, canvas);
        canvas.save();
        let scale = 1. / self.os_dpi();
        canvas.scale((scale, scale));
        egui_manager.egui.paint(canvas);
        canvas.restore();
        self.gr_context.flush_and_submit_surface(sk_surface, None);
        sk_surface.notify_content_will_change(skia_bindings::SkSurface_ContentChangeMode::Discard);
        self.sdl_window.gl_swap_window();
    }

    pub fn push_prompt(&mut self, prompt: InputPrompt) {
        self.prompts.push(prompt);
    }

    pub fn nudge_viewport(&mut self, offset: (f32, f32)) {
        let now_offset = self.viewport.offset;
        self.viewport.offset = (now_offset.0 + offset.0, now_offset.1 + offset.1);
    }

    /// Points per pixel: 1 on most screens, 0.5 on a Retina display. SDL sizes windows in points
    /// and drawables in pixels, so ask it for both, every time; deriving this from
    /// `viewport.winsize` went stale as soon as winsize was converted to pixels or the window
    /// changed displays.
    fn set_dpi_from_os(&mut self) -> f32 {
        let (ww, wh) = self.sdl_window.size();
        let (dw, dh) = self.sdl_window.drawable_size();
        if dw == 0 || dh == 0 {
            // No drawable yet (or a minimized window on some systems): keep what we had.
            return if self.sdl_dpi.is_nan() {
                1.
            } else {
                self.sdl_dpi
            };
        }
        let hdpi = ww as f32 / dw as f32;
        let vdpi = wh as f32 / dh as f32;
        if hdpi != vdpi {
            log::warn!("Warning: DPI's not equal? {} != {}", hdpi, vdpi);
        }
        self.sdl_dpi = hdpi;
        hdpi
    }

    pub fn os_dpi(&mut self) -> f32 {
        if self.sdl_dpi.is_nan() {
            self.set_dpi_from_os()
        } else {
            self.sdl_dpi
        }
    }

    pub fn update_viewport(&mut self, offset: Option<(f32, f32)>, scale: Option<f32>) {
        let offset = match offset {
            None => self.viewport.offset,
            Some(offset) => (offset.0, offset.1),
        };
        let scale = match scale {
            None => self.viewport.factor,
            Some(scale) => scale,
        };

        self.viewport.factor = scale;
        self.viewport.offset = offset;
    }

    pub fn windows(&mut self) -> impl IntoIterator<Item = PopoutWindow<'_>> {
        (Box::new([&mut self.sdl_window])
            .into_iter()
            .chain(&mut self.child_sdl_windows)
            .zip(
                Box::new([&mut self.gl_ctx])
                    .into_iter()
                    .chain(&mut self.child_gl_ctxs),
            )
            .map(|(sdl_window, gl_ctx)| PopoutWindow { sdl_window, gl_ctx }))
        .into_iter()
    }
}

fn create_surface(
    window: &SdlWindow,
    fb_info: &FramebufferInfo,
    gr_context: &mut skia_safe::gpu::DirectContext,
) -> skia_safe::Surface {
    let (width, height) = window.drawable_size();

    let backend_render_target =
        gpu::backend_render_targets::make_gl((width as i32, height as i32), 0, 8, *fb_info);
    gpu::surfaces::wrap_backend_render_target(
        gr_context,
        &backend_render_target,
        GrSurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .unwrap()
}

#[derive(Clone)]
pub enum InputPrompt {
    YesNo {
        question: String,
        afterword: String,
        func: Rc<dyn Fn(&mut Editor, &mut Interface, bool)>,
    },
    Text {
        label: String,
        default: String,
        func: Rc<dyn Fn(&mut Editor, String)>,
    },
}
