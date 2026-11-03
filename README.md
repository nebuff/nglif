# nglif

Stand-alone UFO `.glif` glyph viewer and editor — the Nebuff continuation of
[MFEKglif](https://github.com/MFEK/glif), maintained to build and run on current
Linux distributions.

## Install

```sh
curl -sSL https://install.nebuff.xyz/nglif | bash   # add the Nebuff Repo, once
sudo dnf install nglif                              # Fedora / RHEL
sudo apt install nglif                              # Debian / Ubuntu
sudo pacman -S nglif                                # Arch
```

Every distribution's package is built inside that distribution, so it links its
own GTK 3, SDL2 and glibc and updates with the system. Versions, sizes,
checksums and the signing key: <https://repo.nebuff.xyz/nglif/>.

On a Mac, nglif is an ordinary app: one download runs natively on both Apple
silicon and Intel Macs (macOS 10.15 or later), and Finder opens `.glif` and
`.glifjson` files in it. See [§ Mac users](#mac-users) to build it yourself.

![](doc/screenshot_main.png)

<img src="https://raw.githubusercontent.com/MFEK/glif/master/doc/screenshot.png" width="300"><img src="https://raw.githubusercontent.com/MFEK/glif/master/doc/screenshot2.png" width="300">

## Table of Contents
<!-- https://github.com/alexharv074/markdown_toc -->

1. [Overview](#overview)
2. [Keys](#keys)
    * [I/O](#io)
    * [Tools](#tools)
    * [Selection](#selection)
3. [Installing](#installing)
4. [Building](#building)
    * [Mac users](#mac-users)
    * [Linux users](#linux-users)
        - [Adding `.desktop` shortcut](#adding-desktop-shortcut)
    * [For everyone](#for-everyone)
    * [Note on system SDL2](#note-on-system-sdl2)
        - [Mac users](#mac-users-2)
    * [Errors?](#errors)
5. [I/O Help](#io-help)
6. [Contributing](#contributing)
    * [More debug output](#more-debug-output)
    * [Backtrace logging](#backtrace-logging)
    * [Adding icons](#adding-icons)
    * [Goals](#goals)
    * [«Issues» vs. «Discussions»](#issues-vs-discussions)
7. [License](#license)

## Overview

MFEKglif mixes three technologies: Skia, a powerful path rasterizer and manipulation library; Dear ImGui, an immediate mode GUI toolkit; and Rust, a modern high-performance systems language.

I wrote it after, hopefully, learning from the mistakes made by George Williams in FontForge, after being a user of FontForge for six years and a co-maintainer for one and a half years.

MFEKglif is the flagship program of the Modular Font Editor K project, which aims to create a full font editor by making many small programs that all work together, fulfilling the Unix adage that each program should have one task and do that task well. MFEKglif aims to do the task of creating and editing glyphs well.

To make this as easy as possible to build, and cross-platform without hassle, resources are compiled into the binary via the Rust `include_str!` macro, and MFEKglif is statically compiled by default to its C dependencies.

## Keys

Note: This is a basic list to get you started. A complete list can be found in `resources/default_keymap.xml`. You may copy this file to e.g. `$HOME/.config/MFEK/glif/keybindings.xml` on Linux, or `~/Library/Application Support/MFEK/glif/keybindings.xml` on macOS, and modify it.

On macOS, <kbd>⌘</kbd> Command does everything this list (and the keymap file) calls <kbd>Ctrl</kbd>: <kbd>⌘</kbd><kbd>S</kbd> saves, <kbd>⌘</kbd><kbd>Z</kbd> undoes, <kbd>⌘</kbd>-drag rotates a selection. <kbd>Ctrl</kbd>-click is a right click there, as in every other Mac app.

### I/O
<sup><sub>For more information, see § “I/O Help”.</sub></sup>

* <kbd>Ctrl</kbd><kbd>O</kbd> &mdash; Open user-specified .glif or .glifjson file
* <kbd>Ctrl</kbd><kbd>S</kbd> &mdash; Save current glyph in a multi-layered .glifjson file
* <kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>S</kbd> &mdash; Save current glyph in a multi-layered user-specified .glifjson file
* <kbd>Ctrl</kbd><kbd>U</kbd> &mdash; Flatten the topmost layer, and overwrite current .glif with it
* <kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>U</kbd> &mdash; Flatten the topmost layer, and write it to a user-specified .glif file
* <kbd>Ctrl</kbd><kbd>E</kbd> &mdash; Export the multi-layered .glif to different `glyphs/` directories for each layer, with `layerinfo.plist` and update `layercontents.plist` for each.

### Tools
* <kbd>A</kbd> &mdash; Select &laquo;Pan&raquo; tool
* <kbd>P</kbd> &mdash; Select &laquo;Pen&raquo; tool
* <kbd>V</kbd> &mdash; Select &laquo;Select&raquo; tool
* <kbd>Z</kbd> &mdash; Select &laquo;Zoom&raquo; tool
* <kbd>Alt</kbd><kbd>D</kbd> &mdash; Select &laquo;Dash Along Path&raquo; tool
* <kbd>Alt</kbd><kbd>P</kbd> &mdash; Select &laquo;Pattern Along Path&raquo; tool
* <kbd>W</kbd> &mdash; Select &laquo;Variable Width Stroke&raquo; tool
* <kbd>M</kbd> &mdash; Select &laquo;Measure&raquo; tool
* <kbd>N</kbd> &mdash; Select &laquo;Anchors&raquo; tool
* <kbd>S</kbd> &mdash; Select &laquo;Shapes&raquo; tool

### Selection
* <kbd>Ctrl</kbd><kbd>A</kbd> &mdash; Select all points in current layer
* <kbd>Backspace</kbd> &mdash; Delete currently selected points

## Installing

The Nebuff Repo packages are the supported way to install nglif: add the
repository once, then install the package by name (§ Install above). Every
release also publishes a portable tarball for machines without a package
manager; the current one is linked from <https://repo.nebuff.xyz/nglif/> and
extracts to a plain `usr/` tree.

## Building

### Mac users

You need the Xcode command line tools (`xcode-select --install`), [`rustup`](https://rustup.rs/)
and CMake (`brew install cmake`, or from [cmake.org](https://cmake.org/download/)). Then:

```sh
make macos-app        # or: util/macos/build-app.sh
make macos-dmg        # ... plus a disk image to share it
```

That builds `target/macos/nglif.app`: one universal binary (Apple silicon from
macOS 11, Intel from macOS 10.15) with SDL2 and Skia linked in, so it runs on
any Mac without Homebrew. The script also checks what it built: the binary links
nothing but macOS itself, and both architectures open, save and flatten a glyph
to identical files (on Apple silicon the Intel half runs under Rosetta 2). Add
`--arch arm64` for a quicker single-architecture build while developing; for a
quick run, `cargo run -- examples/Q_.glif` works as on Linux.

The app is signed ad hoc, not notarized, so the first time a downloaded copy is
opened macOS refuses it. Right-click it and choose **Open** (macOS 14 and
earlier), or click **Open Anyway** under System Settings › Privacy & Security
(macOS 15 and later); or run `xattr -dr com.apple.quarantine /Applications/nglif.app`
once. Copies you build yourself are not affected. To sign with a Developer ID
instead, set `CODESIGN_IDENTITY` when running the script; the app is then ready
for `xcrun notarytool submit`.

Every push is built and tested on Apple silicon and Intel Macs by
`.github/workflows/macos.yml`, which also opens a glyph on a virtual Retina
display and keeps screenshots of it.

### Linux users

nglif depends on GTK 3 (for the open/save dialogs) and SDL2 (for windowing), and
at build time on a C/C++ toolchain and CMake (Skia and fontconfig). The pinned
nightly Rust from `rust-toolchain` is installed automatically by `rustup`.

| Distribution | Build dependencies |
| --- | --- |
| Fedora | `gtk3-devel SDL2-devel cmake gcc-c++ pkgconf-pkg-config` |
| Debian / Ubuntu | `libgtk-3-dev libsdl2-dev cmake build-essential pkg-config` |
| Arch | `gtk3 sdl2 cmake base-devel` |

If using X11 and not Wayland, SDL2 also needs the X11 C shape extension
(`libxcb-shape.so.0`) and the xfixes extension (`libxcb-xfixes.so.0`).

#### Adding `.desktop` shortcut

A `.desktop` shortcut lives in `resources/` as `nglif.desktop` (the packages
install it to `/usr/share/applications/`). To install it, assuming nglif is in
your `$PATH`:

1. Copy the icon to `$XDG_HOME`: `mkdir -p ~/.local/share/icons/hicolor/512x512/apps/ && cp resources/nglif.png ~/.local/share/icons/hicolor/512x512/apps/nglif.png`.
2. Run `desktop-file-install --dir=~/.local/share/applications ~/MFEKglif.desktop` to install locally (also depends on `$XDG_HOME`, or run it as root without `--dir` to install the `.desktop` icon globally.
3. Run `update-desktop-database` on the directory used in step 2.

### For everyone

* Download and install [`rustup`](https://rustup.rs/) — `rust-toolchain` pins the
  exact nightly nglif needs, and rustup installs it on the first build.
* Pull this repository, and finally…
* Compile the project. An example command is in § Contributing; you may also find the provided `Makefile` helpful.

### Note on system SDL2

By default, MFEKglif compiles and statically links to SDL2 if not on Linux. If you have SDL2 installed, or find compiling it difficult for some reason and wish to link to a binary SDL2, you should provide the flag `--features=sdl2-dynamic` to `cargo build`. This will disable the features `sdl2/bundled` and `sdl2/static-link`, and your system will attempt to link to a dynamic [libSDL2](https://www.libsdl.org/).

#### Mac users

On macOS, SDL2 is always built from source and linked statically, whatever the
features: Homebrew's `sdl2` is now sdl2-compat, a shim over SDL3, and a binary
linked against it would only run on Macs with Homebrew.

### Errors?

If you previously pulled the repository and get errors related to `glifparser`, `mfek-ipc`, or another local unstable dependency, try running `cargo update` to force Cargo to pull the latest versions from GitHub.

## I/O Help

It's worth taking a moment to explain MFEKglif's I/O. Before November 6, 2021, a complex system was used to save private MFEKglif UFO .glif extensions to the standard .glif format. This became too confusing, both for users and developers, and was removed.

MFEKglif considers UFO .glif as an **import** format, and its own native format is **.glifjson**, which is just a JSON file with keys and values it understands. This is because MFEKglif supports multi-layered glyphs, variable-width stroking, pattern-along-path, and many other features that are of course not in standard .glif; others, such as Spiro and Hyperbézier curves, are planned.

So, when you save (Ctrl+S), MFEKglif will write a file named `a.glifjson` if you had open `a.glif`. To get back out UFO .glif output, you have to do one of the several export abilities MFEKglif has. If you instead save with Ctrl+U, you'll be given a dialog asking you a name for your output .glif file. If you save with Ctrl+Shift+U, MFEKglif will overwrite whatever the current filename is as a `.glif`, so if you've opened `a.glif`, it'll overwrite that; if you've opened `a.glifjson`, it'll write to `a.glif`. This flattens all layers, so you may instead want MFEKglif's most complex (and therefore potentially buggy! please open any issue you find) mode of saving: exporting—Ctrl+E. This will create a new directory for every layer in your glyph and save the layer into it, flattening layer groups.

## Contributing

I typically build and run MFEKglif like this:

```
RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- examples/Q_.glif
```

We welcome all contributions! Please open an issue first so we can discuss before you make big changes so no effort is wasted.

### More debug output

It is possible to get even more debug output out of MFEKglif for figuring out where problems lie. To ask MFEKglif to dump the parsed .glif file on runtime, pass `MFEK_DEBUG_DUMP_GLYPH=Y`. To see every single `sdl2` event (warning: this will flood your stdout) pass `MFEK_DEBUG_EVENTS=Y`.

### Backtrace logging

By default, MFEKglif writes its backtraces to your temporary directory (it outputs the filename on crash as well). You should share these when reporting a crash; you may stop them from being written with the environment variable `MFEK_BACKTRACE_NO_WRITE=Y`.

To quiet all potentially useful information from a crash, set `MFEK_QUIET_CRASH=Y`.

### Adding icons

Icons are themselves `.glif` files, see `resources/fonts/MFEKglifIconFont.ufo/glyphs`. Once you add a glyph to the UFO, you can rebuild the font (provided you have `fontmake`) with `make iconfont`.

### Goals

Contributions which do not work on at least GNU/Linux and Windows will be rejected; we want to be able to build MFEKglif on as many platforms as possible. Both Skia and Dear ImGui are cross-platform; we use Vulkan and not OpenGL so we are future-proof even on OS X.

Contibutions will also be judged on how well they fit into the MFEK project as a whole. It's possible that your idea fits better into another module and not MFEKglif; we can help you figure out where it should go.

### «Issues» vs. «Discussions»

If you find a bug in MFEKglif, please open the bug against this repository in «[Issues](https://github.com/MFEK/glif/issues)». Your bug may be moved by a maintainer (as of 4 January 2022 Fred Brennan/@ctrlcctrlv is the only MFEK maintainer) to the bug tracker of another MFEK module or library, considering only where _the code resides_ that your issue concerns, not the module you're using. When in doubt as to which repository is correct, just use this one—let Fred figure it out.

To discuss the MFEK project/offer ideas/report your experience, (good or bad!) and not report a bug, please open a thread in «[Discussions](https://github.com/MFEK/glif/discussions)». «Discussions» is not at all strict about repository hygeine, so you may discuss anything related to MFEK there.

## License

```
Copyright 2020–2021 Fredrick R. Brennan, Matthew Blanchard & MFEK Authors

Licensed under the Apache License, Version 2.0 (the "License"); you may not use
this software or any of the provided source code files except in compliance
with the License.  You may obtain a copy of the License at

  http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied.  See the License for the
specific language governing permissions and limitations under the License.
```

**By contributing you release your contribution under the terms of the license.**
