#![allow(unused)]

#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate cargo_emit;

use std::env;

fn main() {
    cfg_if! {
        if #[cfg(all(feature = "sdl2-static", feature = "sdl2-dynamic"))] {
            compile_error!("Features `sdl2-static` and `sdl2-dynamic` are mutually exclusive!");
        }
    }
    // The version in Cargo.toml is the single source of truth: `git describe`
    // produced `-desync`/commit-hash strings that no package manager accepts.
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| String::from("0.0.0"));
    rustc_env!("MFEK_VERSION", "{}", version);
    rustc_env!(
        "MFEK_PKG_NAME",
        "{}",
        env::var("CARGO_PKG_NAME").unwrap_or_else(|_| String::from("nglif"))
    );
    rustc_env!("MFEK_COMPILED_AT", "{}", chrono::Local::now().timestamp());

    // A build script runs on the machine doing the building, so `#[cfg(target_os)]` in here
    // describes the build host, not the program being built. Ask Cargo about the target instead;
    // otherwise a cross-compiled nglif (x86_64 macOS built on an Apple Silicon Mac, Windows built
    // on Linux, ...) is configured for the wrong operating system.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    let sdl2_static = env::var_os("CARGO_FEATURE_SDL2_STATIC").is_some();
    let sdl2_dynamic = env::var_os("CARGO_FEATURE_SDL2_DYNAMIC").is_some();

    if target_os == "macos" {
        // Cargo.toml always builds SDL2 from source and links it statically on macOS (see the
        // comment there). SDL 2.0.18+ drives game controller rumble through CoreHaptics, which
        // sdl2-sys 0.35 forgets to link, so without this the final link fails with undefined
        // `_OBJC_CLASS_$_CHHapticPattern` and friends.
        println!("cargo:rustc-link-lib=framework=CoreHaptics");
        if sdl2_dynamic {
            warning!("`sdl2-dynamic` is ignored on macOS: SDL2 is always built from source and linked statically there, so the app runs on Macs without Homebrew.");
        }
    }

    if cfg!(feature = "reproducible-build") {
        rustc_cfg!("reproducible");
    } else if sdl2_static && target_os == "linux" {
        warning!(
            "Trying to statically link SDL2 is known to be buggy on Linux. Proceed with caution."
        );
    } else if sdl2_dynamic && target_family == "windows" {
        warning!(
            "It is neither recommended nor supported to compile nglif on Windows w/dynamic SDL2."
        );
    }

    println!("cargo::rustc-check-cfg=cfg(is_free_software_os)");
    if target_family == "unix" && !matches!(target_os.as_str(), "macos" | "ios" | "android") {
        rustc_cfg!("is_free_software_os");
        rustc_env!("MFEK_FREE_OS_DETECTED", "{}", version);
    } else {
        warning!("nglif works best on free software operating systems such as GNU/Linux and FreeBSD. Close Windows, open doors: <https://www.fsf.org/windows>");
    }
}
