# Fotry

**Fotry** is the image editor of the [Arch Quilly](https://github.com/Sapyst/quilly) suite: a native editor (Rust, drawn on the GPU through wgpu/Vulkan) with layers and groups, masks, adjustment layers, layer styles, live type, vectors, brushes (`.abr` import), smart objects, camera RAW and real PSD files.

It is a fork of **[PhotoCraft](https://github.com/storytold/photocraft) by the ArtCraft team** (MIT OR Apache-2.0), which provides the engine. Fotry adds:

- the **Aero** look (`crates/ui-egui/src/aero.rs`): Aero Dark and Aero Light, the Arch Aero desktop's greys, rounder corners and the desktop's accent colour, following the system's light/dark setting;
- **Image › Upscale…** (`crates/engine/src/upscale_cmds.rs`, `Ctrl+Alt+U`): 2×, 3× or 4× of the whole document as one undo step, with Lanczos, Preserve Details, or AI (Real-ESRGAN on the GPU through Vulkan, downloaded on request);
- the suite's name, icon and links (`com.archquilly.Fotry`, settings in `~/.config/arch-quilly/fotry`).

The ArtCraft name and marks are not used: as `docs/brand/LICENSE-brand.txt` asks of modified versions, the mark files were removed from this branch. PhotoCraft's own README is kept in [`docs/upstream-README.md`](docs/upstream-README.md).

## Build

```sh
git clone https://github.com/storytold/craft-fonts ../craft-fonts
CRAFT_FONTS_DIR="$PWD/../craft-fonts" cargo build --release -p photocraft
```

On Arch Linux, install the suite's package instead (`arch-fotry` or the whole `arch-quilly`, from the [Arch Quilly releases](https://github.com/Sapyst/quilly/releases)); the Quilly hub updates it.

Branch `fotry` follows PhotoCraft's `main` (`git fetch upstream && git rebase upstream/main`).

## License

MIT OR Apache-2.0, as PhotoCraft (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `ATTRIBUTION.md`).
