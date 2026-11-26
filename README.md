# RTCFLE

RTCFLE is a browser-based SWF asset corruption tool powered by a custom-built Ruffle web player and published with GitHub Pages.

## Features

- Load local SWF files by browsing or drag-and-drop.
- Play the selected SWF in the self-hosted Ruffle build.
- Corrupt supported internal vector assets before playback with either Classic Mutation or the separate Vector Stretch mode.
- Corrupt supported bitmap assets with an independent strength control; Vector Stretch can smear selected bitmap scanlines.
- Choose vector-only, image-only, or both asset targets.
- Export the pre-playback-mutated SWF as a separate copy; the original file is not overwritten.
- Mutate vector shapes inside the currently loaded Ruffle runtime while the movie keeps playing.
- Apply Sprite Aspect Warp to live display-list instances, scaling local X and Y matrix axes independently to create wide/tall sprite proportions, with optional flips, shear, offsets, and near-collapse glitches. It changes current instances rather than shared character definitions and never calls `load()` or `reload()`.\n- Choose ten additional live vector-only effects: Shatter Lines, Melt Geometry, Ripple Distortion, Mirror Flip, Pixel Grid, Spiral Twist, Outline Explosion, Gradient Storm, Ghost Contours, and Vector Chaos. The `ALL VECTOR EFFECTS` option runs all ten in one mutation pass.\n- Expand the SWF preview to browser fullscreen and exit fullscreen without leaving the page.

## Custom Ruffle runtime

RTCFLE builds a pinned Ruffle source revision in GitHub Actions and applies the RTCFLE patch set in `scripts/` before compiling the WebAssembly player. The source revision is pinned to `9c6ab49454c96222a2d55db2c361cbf3faa44785`.

The patch adds:

- In-place vector mutation methods to Ruffle's `Graphic` and `MorphShape` display objects.
- Geometry, line, fill, gradient, and bitmap-fill transform perturbations for SWF vector definitions.
- Recalculation and expansion of SWF geometry bounds after vector changes, so culling and hit testing do not keep stale rectangles.
- Invalidation of Graphic tessellation handles and MorphShape's cached interpolated frames whenever their source vectors change.
- Invalidation of active display-object bitmap caches and their ancestors.
- WASM methods and public JavaScript methods: `player.ruffle().mutateVectorShapes(intensity, seed)`, `player.ruffle().mutateVectorStretch(intensity, seed)`, and `player.ruffle().mutateSpriteAspectWarp(intensity, glitchChance, seed)`.

The mutation walks all character definitions registered in the loaded movie libraries, including symbols used by later timeline frames, rather than only changing whatever is visible at the instant the button is pressed. All active Stage levels are traversed to invalidate display-object bitmap caches and their ancestors, so the next normal render draws the changed vectors. The timeline is not reloaded or restarted. Sprite Aspect Warp traverses the active display list and changes each selected instance's local matrix, preserving its translation/rotation basis while scaling the local X and Y axes independently. Display-object bitmap caches and their ancestors are invalidated after each selected instance changes.

The UI maps its 1–1000 vector slider to the runtime's 1–100 live-mutation intensity range. Each click uses a new random seed, so successive passes do not intentionally repeat the same mutation sequence. Vector Stretch amplifies selected straight and curved edge deltas, shifts selected contour starts, stretches bitmap-fill mappings, widens or collapses selected strokes, alters color/gradient stops, and can set selected alpha-bearing fills to transparent. Its pre-playback path also smears selected bitmap scanlines. The mutations are randomized and local to vector segments; the engine does not semantically identify named body parts such as arms or faces.

## Limitations

- Classic Mutation, Vector Stretch, and all ten dedicated vector effects target vector `Graphic` and `MorphShape` definitions. Sprite Aspect Warp targets the current display-list instances (including nested movie clips, vector shapes, bitmaps, and text objects) by changing their transform matrices. It does not rewrite standalone bitmap pixel buffers, embedded font glyphs, text runs, or script-created `Drawing` content; pre-playback processing handles supported bitmap pixels separately.
- The runtime mutation affects the in-memory Ruffle movie. The SWF export button exports the separate pre-playback SWF copy; it does not serialize live in-memory Ruffle changes back into a SWF file.
- The custom Ruffle build must complete successfully in GitHub Actions before the Pages deployment contains the updated player.
- The mutation changes geometry and styles while preserving the SWF movie's timeline and script execution. It does not guarantee that every unusual SWF feature will remain visually coherent after heavy corruption.

## Safety and file handling

The original local file remains untouched. Pre-playback mutation happens on a temporary in-memory copy, and Ruffle's networking and script-access restrictions remain configured by RTCFLE.
