//! macOS only: tell "opened to edit a document" apart from "opened on its own".
//!
//! LaunchServices starts an app bundle with one Apple Event: `odoc` (open documents) when the
//! user opened files with it from Finder, `oapp` (open application) otherwise. SDL turns `odoc`
//! into SDL_DROPFILE events but drops `oapp`, so without this nglif could only guess how long to
//! wait for a document that may never come before showing its Open dialog.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

type OSErr = i16;
type Handler =
    extern "C" fn(event: *const c_void, reply: *mut c_void, refcon: *mut c_void) -> OSErr;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn AEGetEventHandler(
        class: u32,
        id: u32,
        handler: *mut Option<Handler>,
        refcon: *mut *mut c_void,
        is_sys_handler: u8,
    ) -> OSErr;
    fn AEInstallEventHandler(
        class: u32,
        id: u32,
        handler: Handler,
        refcon: *mut c_void,
        is_sys_handler: u8,
    ) -> OSErr;
}

const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
const OPEN_APPLICATION: u32 = u32::from_be_bytes(*b"oapp");

static OPENED_WITHOUT_DOCUMENTS: AtomicBool = AtomicBool::new(false);
// Cocoa's own handler, which ours hands the event on to so AppKit sees it as before.
static COCOA_HANDLER: AtomicUsize = AtomicUsize::new(0);
static COCOA_REFCON: AtomicUsize = AtomicUsize::new(0);

extern "C" fn open_application(event: *const c_void, reply: *mut c_void, _: *mut c_void) -> OSErr {
    OPENED_WITHOUT_DOCUMENTS.store(true, Ordering::SeqCst);
    match COCOA_HANDLER.load(Ordering::SeqCst) {
        0 => 0, // noErr
        cocoa => {
            let cocoa: Handler = unsafe { std::mem::transmute(cocoa) };
            cocoa(
                event,
                reply,
                COCOA_REFCON.load(Ordering::SeqCst) as *mut c_void,
            )
        }
    }
}

/// Starts listening for `oapp`. Call once SDL's video subsystem is up (AppKit has installed its
/// handlers by then) and before pumping events, which is what delivers the launch event.
pub fn watch_for_open_application() {
    unsafe {
        let mut cocoa: Option<Handler> = None;
        let mut refcon: *mut c_void = std::ptr::null_mut();
        if AEGetEventHandler(
            CORE_EVENT_CLASS,
            OPEN_APPLICATION,
            &mut cocoa,
            &mut refcon,
            0,
        ) == 0
        {
            if let Some(cocoa) = cocoa {
                COCOA_HANDLER.store(cocoa as usize, Ordering::SeqCst);
                COCOA_REFCON.store(refcon as usize, Ordering::SeqCst);
            }
        }
        let err = AEInstallEventHandler(
            CORE_EVENT_CLASS,
            OPEN_APPLICATION,
            open_application,
            std::ptr::null_mut(),
            0,
        );
        if err != 0 {
            log::warn!(
                "Could not watch for the open-application Apple Event (OSErr {})",
                err
            );
        }
    }
}

/// True once macOS has said this launch came with no documents to open.
pub fn opened_without_documents() -> bool {
    OPENED_WITHOUT_DOCUMENTS.load(Ordering::SeqCst)
}
