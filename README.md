# RTCFLE

RTCFLE is a standalone-style SWF asset corruption tool for the browser, powered by [Ruffle](https://ruffle.rs/) and published with GitHub Pages.

## Features

- Load local SWF files by browsing or drag-and-drop.
- Play the selected SWF through the Ruffle web player loaded from unpkg.
- Corrupt supported internal vector assets: fill and gradient colors, line colors, and shape edge coordinates.
- Corrupt supported embedded bitmap assets: lossless bitmap pixel data and decodable JPEG/PNG/GIF image resources.
- Choose vector-only, image-only, or both asset targets.
- Adjust internal corruption strength and algorithm.
- Export the modified SWF as a separate file.
- Keep the source file unchanged; mutation happens on a temporary in-memory copy.

There are no screen filters, CRT overlays, visual jitter, hue/contrast effects, or display randomizers. RTCFLE edits supported resources inside the SWF file itself.

## Supported SWF asset edits

- FWS (uncompressed) and CWS (zlib-compressed) file containers.
- DefineShape, DefineShape2, DefineShape3, and DefineShape4 vector tags.
- DefineBitsLossless / DefineBitsLossless2 formats 3, 4 (where defined), and 5.
- Decodable DefineBitsJPEG2 / JPEG3 / JPEG4 bitmap assets. JPEG3/4 assets are converted to PNG resources when their alpha channel is merged.
- Unsupported, malformed, or ZWS/LZMA input is left unchanged rather than forced through the mutator.

## Safety and file handling

The mutator edits only a temporary copy in browser memory. The original file is not uploaded or overwritten. Ruffle keeps its browser sandbox, ActionScript-to-page access disabled, external URL opening denied, and networking APIs restricted to its internal mode. The app does not disable Ruffle or browser security protections.

An internet connection is required to load the Ruffle player from unpkg. Some SWF files may rely on external resources or unsupported Flash features and may not work in Ruffle.

## GitHub Pages

The workflow in `.github/workflows/deploy.yml` publishes the site when changes are pushed to `main`. In **Settings → Pages → Build and deployment**, set **Source** to **GitHub Actions**.

Site URL: https://10574084-cmd.github.io/RTCFLE/
