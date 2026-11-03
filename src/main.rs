//! MFEKglif - A cross-platform .glif renderer and editor.
//! (c) 2020–2021 Fredrick R. Brennan, Matthew Blanchard & MFEK Authors
//! Apache 2.0 licensed. See AUTHORS.
#![allow(non_snake_case)] // for our name MFEKglif
#![allow(incomplete_features)]
#![feature(
    const_trait_impl,
    generic_const_exprs,
    adt_const_params,
    unsized_const_params
)]

use crate::command::{Command, CommandInfo, CommandMod};
use crate::editor::{
    events::{EditorEvent, IOEventType, MouseEventType},
    Editor,
};
use crate::tools::zoom::{zoom_in_factor, zoom_out_factor};
use crate::tools::ToolEnum;
use crate::user_interface::mouse_input::MouseInfo;
use crate::user_interface::Interface;

use enum_iterator::IntoEnumIterator as _;
use glifrenderer::toggles::{PointLabels, PreviewMode};
use sdl2::event::{Event, WindowEvent};
use sdl2::mouse::{MouseButton, MouseWheelDirection};
use tool_behaviors::pan::PanBehavior;
use user_interface::egui_manager::EguiManager;
use user_interface::gui::window::WindowManager;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pub_mod;

use std::cell::RefCell;
use std::path::PathBuf;

pub mod args;
mod command;
pub mod constants;
mod contour_operations;
pub mod editor;
mod filedialog;
mod ipc;
mod render;
pub mod settings;
mod system_fonts;
mod tool_behaviors;
mod tools;
mod user_interface;
pub mod util;

fn main() {
    ipc::header();
    util::init_env_logger();
    util::set_panic_hook();

    let args = args::parse_args();
    let filename = args.filename.clone().map(PathBuf::from);
    let mut editor = Editor::new(args);

    // On macOS, start SDL before deciding what to open: a document opened from Finder only
    // arrives once Cocoa is up. Elsewhere the file dialog still runs first, as it always has.
    #[cfg(target_os = "macos")]
    let (sdl, filename) = {
        let sdl = Interface::init_sdl();
        let filename = filename.or_else(|| Interface::take_launch_document(&sdl.0));
        (sdl, filename)
    };
    let filename = match filename {
        Some(filename) => filename,
        None => filedialog::filename_or_panic(&None, Some("glif"), None),
    };
    #[cfg(not(target_os = "macos"))]
    let sdl = Interface::init_sdl();
    let mut interface = Interface::new(sdl.0, sdl.1, filename.to_str().unwrap());
    let mut sk_surface = interface.create_surface();
    let mut window_manager = WindowManager::new();
    let mut egui_manager = EguiManager::new(&mut interface);

    // Makes glyph available to on_load_glif events
    editor.load_glif(&mut interface, &filename);

    ctrlc::set_handler(util::quit_next_frame).expect("Could not set SIGTERM handler.");

    let _notify_thread = ipc::launch_fs_watcher(&mut editor);

    command::initialize_keybinds();
    // TODO: Replace console! tools::console::initialize_console_commands();

    let mut event_pump = interface.get_event_pump();
    'main_loop: loop {
        // Quit from console
        if editor.quit_requested {
            break 'main_loop;
        }

        let keys_down = interface.get_pressed_keys(&event_pump);
        let keymod = CommandMod::from_keys_down(&keys_down);

        // sdl event handling
        for event in event_pump.poll_iter() {
            util::log_sdl_event(&event);

            if let Event::Quit { .. } = &event {
                editor.quit(&mut interface);
            }

            if egui_manager.wants_event(
                interface.windows().into_iter().nth(0).unwrap().sdl_window,
                &event,
            ) {
                continue;
            }

            if interface.active_prompts() {
                continue;
            }

            match event {
                Event::KeyDown { keycode, .. } => {
                    // we don't care about keydown events that have no keycode
                    if keycode.is_none() {
                        continue;
                    }
                    let keycode = keycode.unwrap();

                    // check if we've got a command
                    let command_info: CommandInfo =
                        match command::keycode_to_command(&keycode, &keys_down) {
                            Some(command) => command,
                            None => continue,
                        };

                    let delete_after = RefCell::new(false);
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::ToolCommand {
                            command: command_info.command,
                            command_mod: command_info.command_mod,
                            stop_after: delete_after.clone(),
                        },
                    );
                    if *delete_after.borrow() {
                        continue;
                    }

                    log::trace!("Received command: {:?}", command_info.command);

                    match command_info.command {
                        Command::ResetScale => {
                            interface.update_viewport(None, Some(1.));
                        }
                        Command::ZoomIn => {
                            let scale = zoom_in_factor(&mut interface);
                            interface.update_viewport(None, Some(scale));
                        }
                        Command::ZoomOut => {
                            let scale = zoom_out_factor(&mut interface);
                            interface.update_viewport(None, Some(scale));
                        }
                        Command::ToolPan => {
                            editor.set_tool(ToolEnum::Pan);
                        }
                        Command::ToolPen => {
                            editor.set_tool(ToolEnum::Pen);
                        }
                        Command::ToolSelect => {
                            editor.set_tool(ToolEnum::Select);
                        }
                        Command::ToolZoom => {
                            editor.set_tool(ToolEnum::Zoom);
                        }
                        Command::ToolDash => {
                            editor.set_tool(ToolEnum::Dash);
                        }
                        Command::ToolPAP => {
                            editor.set_tool(ToolEnum::PAP);
                        }
                        Command::ToolVWS => {
                            editor.set_tool(ToolEnum::VWS);
                        }
                        Command::ToolMeasure => {
                            editor.set_tool(ToolEnum::Measure);
                        }
                        Command::ToolAnchors => {
                            editor.set_tool(ToolEnum::Anchors);
                        }
                        Command::ToolShapes => {
                            editor.set_tool(ToolEnum::Shapes);
                        }
                        Command::ToolGuidelines => {
                            editor.set_tool(ToolEnum::Guidelines);
                        }
                        Command::ToolImages => {
                            editor.set_tool(ToolEnum::Image);
                        }
                        Command::TogglePointLabels => {
                            trigger_toggle_on!(
                                interface,
                                point_labels,
                                PointLabels,
                                !command_info.command_mod.shift
                            );
                        }
                        Command::TogglePreviewMode => {
                            trigger_toggle_on!(
                                interface,
                                preview_mode,
                                PreviewMode,
                                !command_info.command_mod.shift
                            );
                        }
                        /* TODO: Replace console!
                        Command::ToggleConsole => {
                            CONSOLE.with(|c| {
                                c.borrow_mut().active = true;
                            });
                        }*/
                        Command::DeleteSelection => {
                            if editor.selected.is_empty() && editor.point_idx.is_some() {
                                editor.simplify_cubic_selection();
                            } else {
                                editor.delete_selection();
                            }
                        }
                        Command::SelectAll => {} // handled by select tool, only when select active
                        Command::CopySelection => {
                            editor.copy_selection();
                        }
                        Command::PasteSelection => {
                            editor.paste_selection(Some(interface.mouse_info.position));
                        }
                        Command::PasteSelectionInPlace => {
                            editor.paste_selection(None);
                        }
                        Command::CutSelection => {
                            editor.copy_selection();
                            editor.delete_selection();
                        }
                        Command::HistoryUndo => {
                            editor.undo();
                        }
                        Command::HistoryRedo => {
                            editor.redo();
                        }
                        Command::IOOpen => {
                            let filename =
                                match filedialog::open_filename(Some("glif,glifjson"), None) {
                                    Some(f) => f,
                                    None => continue,
                                };
                            editor.load_glif(&mut interface, &filename);
                        }
                        Command::IOSave => {
                            drop(editor.save_glif(false));
                            editor.dispatch_editor_event(
                                &mut interface,
                                EditorEvent::IOEvent {
                                    event_type: IOEventType::FileSaved,
                                    path: filename.clone(),
                                },
                            );
                        }
                        Command::IOSaveAs => match editor.save_glif(true) {
                            Ok(pb) => {
                                editor.dispatch_editor_event(
                                    &mut interface,
                                    EditorEvent::IOEvent {
                                        event_type: IOEventType::FileSavedAs,
                                        path: pb.clone(),
                                    },
                                );
                                editor.load_glif(&mut interface, &pb);
                            }
                            Err(()) => {}
                        },
                        Command::IOFlatten | Command::IOFlattenAs => {
                            let rename = command_info.command == Command::IOFlattenAs;
                            let event_type = if rename {
                                IOEventType::FileFlattenedAs
                            } else {
                                IOEventType::FileFlattened
                            };
                            match editor.flatten_glif(Some(&mut interface), rename) {
                                Ok(filename) => editor.dispatch_editor_event(
                                    &mut interface,
                                    EditorEvent::IOEvent {
                                        event_type,
                                        path: filename,
                                    },
                                ),
                                Err(()) => {}
                            }
                        }
                        Command::IOExport => {
                            if let Ok(()) = editor.export_glif(Some(&mut interface)) {
                                editor.dispatch_editor_event(
                                    &mut interface,
                                    EditorEvent::IOEvent {
                                        event_type: IOEventType::FileExported,
                                        path: filename.clone(),
                                    },
                                );
                            }
                        }
                        Command::Quit => {
                            editor.quit(&mut interface);
                        }
                        // TODO: More elegantly deal with Command's meant for consumption by a
                        // single tool?
                        Command::ReverseContour => {
                            log::debug!("Tried to reverse contour outside Select tool");
                        }
                        Command::SkiaDump => {
                            editor.skia_dump();
                        }
                        #[allow(unreachable_patterns)]
                        // This failsafe is here if you add a Command.
                        cmd => log::error!("Command unimplemented: {:?}", cmd),
                    }
                }

                Event::MouseMotion { x, y, .. } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(&mut interface, None, position, None, keymod);
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::Moved,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseButtonDown {
                    mouse_btn,
                    x,
                    y,
                    clicks: 2,
                    ..
                } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(
                        &mut interface,
                        Some(mouse_btn),
                        position,
                        Some(true),
                        keymod,
                    );
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::DoubleClick,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(
                        &mut interface,
                        Some(mouse_btn),
                        position,
                        Some(true),
                        keymod,
                    );

                    if mouse_btn == MouseButton::Middle {
                        editor.push_behavior(Box::new(PanBehavior::new(
                            interface.viewport.clone(),
                            mouse_info,
                        )));
                        continue;
                    }

                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::Pressed,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    let position = (x as f32, y as f32);
                    let mouse_info = MouseInfo::new(
                        &mut interface,
                        Some(mouse_btn),
                        position,
                        Some(false),
                        keymod,
                    );
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::MouseEvent {
                            event_type: MouseEventType::Released,
                            mouse_info,
                        },
                    );

                    interface.mouse_info = mouse_info;
                }

                Event::MouseWheel {
                    x, y, direction, ..
                } => {
                    let (horizontal, vertical) = wheel_steps(x, y, direction);
                    editor.dispatch_editor_event(
                        &mut interface,
                        EditorEvent::ScrollEvent {
                            horizontal,
                            vertical,
                        },
                    );
                }

                // A .glif dragged onto the window (or, on macOS, onto the Dock icon, or opened
                // from Finder while nglif is already running).
                Event::DropFile { filename, .. } => {
                    editor.open_dropped_glif(&mut interface, PathBuf::from(filename));
                }

                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::SizeChanged(..) | WindowEvent::Resized(..) => {
                        interface.viewport.set_broken_flag();
                        interface.refresh_window_metrics();
                        sk_surface = interface.create_surface();
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        editor.rebuild(&mut interface);
        interface.render(
            &mut editor,
            &mut window_manager,
            &mut egui_manager,
            &mut sk_surface,
        );
    }
}

/// Every tool takes one unit of wheel motion as one zoom (or shape-edit) step, as a mouse
/// wheel notch gives on Linux. A Mac trackpad instead reports two-finger scrolling in points,
/// often dozens per event, and under "natural scrolling" (the macOS default) with the sign
/// reversed. So on macOS, keep the physical direction and take at most one step per event.
fn wheel_steps(x: i32, y: i32, direction: MouseWheelDirection) -> (i32, i32) {
    if cfg!(target_os = "macos") {
        let sign = if direction == MouseWheelDirection::Flipped {
            -1
        } else {
            1
        };
        ((x * sign).signum(), (y * sign).signum())
    } else {
        (x, y)
    }
}

#[test]
fn wheel_steps_per_platform() {
    use MouseWheelDirection::{Flipped, Normal};
    if cfg!(target_os = "macos") {
        // A fast two-finger swipe: one zoom step, not forty.
        assert_eq!(wheel_steps(0, 40, Normal), (0, 1));
        assert_eq!(wheel_steps(0, -40, Normal), (0, -1));
        // Natural scrolling reports a swipe up as negative y; it should still zoom in.
        assert_eq!(wheel_steps(0, -40, Flipped), (0, 1));
        assert_eq!(wheel_steps(3, 0, Flipped), (-1, 0));
        assert_eq!(wheel_steps(0, 0, Normal), (0, 0));
    } else {
        assert_eq!(wheel_steps(0, 3, Normal), (0, 3));
        assert_eq!(wheel_steps(-2, 0, Flipped), (-2, 0));
    }
}
