# RTCFLE

RTCFLE is a browser-based SWF asset corruption tool powered by a custom-built Ruffle web player and published with GitHub Pages.

## Features

- Load local SWF files by browsing or drag-and-drop.
- Play the selected SWF in the self-hosted Ruffle build.
- Corrupt supported internal vector assets before playback.
- Corrupt supported bitmap assets with an independent strength control.
- Choose vector-only, image-only, or both asset targets.
- Export the pre-playback-mutated SWF as a separate copy; the original file is not overwritten.
- Mutate vector shapes inside the currently loaded Ruffle runtime while the movie keeps playing. This does not call `load()` or `reload()`.

## Custom Ruffle runtime

RTCFLE builds a pinned Ruffle source revision in GitHub Actions and applies the RTCFLE patch set in `scripts/` before compiling the WebAssembly player. The source revision is pinned to `9c6ab49454c96222a2d55db2c361cbf3faa44785`.

The patch adds:

- In-place vector mutation methods to Ruffle's `Graphic` and `MorphShape` display objects.
- Geometry, line, fill, gradient, and bitmap-fill transform perturbations for SWF vector definitions.
- Invalidation of Graphic tessellation handles and MorphShape's cached interpolated frames whenever their source vectors change.
- Invalidation of active display-object bitmap caches and their ancestors.
- A WASM method and public JavaScript method: `player.ruffle().mutateVectorShapes(intensity, seed)`.

The mutation walks all character definitions registered in the loaded movie libraries, including symbols used by later timeline frames, rather than only changing whatever is visible at the instant the button is pressed. Active instances are then invalidated so the next normal render draws the changed vectors. The timeline is not reloaded or restarted.

The UI maps its 1–1000 vector slider to the runtime's 1–100 live-mutation intensity range. Each click uses a new random seed, so successive passes do not intentionally repeat the same mutation sequence.

## Limitations

- The runtime mutation currently targets vector `Graphic` and `MorphShape` definitions. It does not rewrite embedded font glyphs, text runs, bitmap pixel data, or script-created `Drawing` content.
- The runtime mutation affects the in-memory Ruffle movie. The SWF export button exports the separate pre-playback SWF copy; it does not serialize live in-memory Ruffle changes back into a SWF file.
- The custom Ruffle build must complete successfully in GitHub Actions before the Pages deployment contains the updated player.
- The mutation changes geometry and styles while preserving the SWF movie's timeline and script execution. It does not guarantee that every unusual SWF feature will remain visually coherent after heavy corruption.

## Safety and file handling

The original local file remains untouched. Pre-playback mutation happens on a temporary in-memory copy, and Ruffle's networking and script-access restrictions remain configured by RTCFLE.
