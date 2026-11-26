# RTCFLE

RTCFLE is a browser-based SWF visual corruption playground powered by [Ruffle](https://ruffle.rs/) and served as a GitHub Pages site.

## Features

- Load local SWF files by browsing or drag-and-drop.
- Play SWF content through the official Ruffle web player loaded from unpkg.
- Modify supported SWF assets before playback: vector fill/gradient/line colors, vector edge coordinates, embedded lossless bitmap resources, and decodable JPEG/PNG/GIF bitmap resources.
- Apply display effects in real time: hue shift, contrast, saturation, CRT scanlines, noise, jitter, and glitch bars.
- Randomize the display effects manually or automatically.
- Preserve the original selected file; internal edits are made to a temporary in-memory copy.

## Supported SWF asset edits

- FWS (uncompressed) and CWS (zlib-compressed) file containers.
- DefineShape, DefineShape2, DefineShape3, and DefineShape4 vector tags.
- DefineBitsLossless / DefineBitsLossless2 formats 3, 4 (where defined), and 5.
- Decodable DefineBitsJPEG2 / JPEG3 / JPEG4 bitmap assets. JPEG3/4 assets are converted to PNG resources when their alpha channel is merged.
- Unsupported, malformed, or ZWS/LZMA input is left unchanged rather than forced through the mutator.

## Safety and file handling

The mutator edits only a temporary copy in browser memory. The original file is not uploaded or overwritten. Ruffle keeps its browser sandbox, ActionScript-to-page access disabled, external URL opening denied, and networking APIs restricted to its internal mode. The app intentionally does not disable Ruffle or browser security protections.

An internet connection is required to load the Ruffle player from unpkg. Some SWF files may rely on external resources or unsupported Flash features and may not work in Ruffle.

## GitHub Pages

The workflow in `.github/workflows/deploy.yml` publishes the site when changes are pushed to `main`. In **Settings → Pages → Build and deployment**, set **Source** to **GitHub Actions**.

Site URL: https://10574084-cmd.github.io/RTCFLE/
