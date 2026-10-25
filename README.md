# Termynter

Termynter is a functional native TUI client for [Lyntr](https://lyntr.gizmowizard.tech) written in Rust!

Termynter was created as part of the 2026 Lyntr Hackathon that ran from the 10th of October to the 25th of October, 2026.

> [!IMPORTANT]
> Please note that while Termynter does, of course, work (hopefully), it was made by a single person (me, ItsThatOneJack) in a period of about two weeks on a cost of £0 and amongst other projects, so do not expect too much polish.

## Controls

Termynter has some pretty simple and (hopefully) intuitive controls!

You can use `UpArrow` and `DownArrow`, alongside `J` and `K` if you wish, to scroll up and down in lists and between items vertically in a pane. Similarly, you can use `RightArrow` and `LeftArrow` alongside `H` and `L` to scroll side to side when needed.

You can use `Enter`/`Return` to select a text box or to click a button, and `Escape`/`Esc` to stop editing. You can also use `Enter`/`Return` to stop editing in most cases.

To move between panes, you can use `Tab` to cycle through them, or `Shift+[any of the movement keys mentioned above]` to move between them exactly. In some cases `Shift+[a movement key]` is needed to navigate within one pane, such as when moving from a search box to the results, or from the top bar of a list to the list itself.

Intuitively, items you are hovering over but are not selecting are lit up yellow, so is the pane you are currently in. If you are focused into something, it will be lit up green, this does not happen for buttons as you can't focus them, only hover and press.

If you ever get stuck, you will likely be okay again quite quickly thanks to the hint bar at the bottom of Termynter, it shows you the available keys for you at any moment, though keep in mind it is not perfect.

## Building

Pre-built binaries are provided for x86 Linux and x86 Windows, however you can build Termynter from the project root on a Windows machine using:

```shell
rustup target add x86_64-unknown-linux-gnu # To get the required target for compiling for Linux.
cargo build --target x86_64-unknown-linux-gnu --release # To build for Linux.
cargo build --release # To build for Windows.
```

Or if building for both from a Linux machine:

```shell
rustup target add x86_64-pc-windows-gnu # To get the required target for compiling for Windows.
cargo build --target x86_64-pc-windows-gnu --release # To build for Windows.
cargo build --release # To build for Linux.
```

Keep in mind you may need to install platform-dependent linkers for the other OS, and so in your particular case it may be easier to use a system such as [cross](https://github.com/cross-rs/cross).

## License

Termynter is licensed under the Mozilla Public License version 2.0, if a copy of the MPL was not distributed with Termynter, you can obtain a copy [from Mozilla](https://mozilla.org/MPL/2.0/) or [from SPDX](https://spdx.org/licenses/MPL-2.0.html).
