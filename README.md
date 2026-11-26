# RTCFLE

RTCFLE is a browser-based, standalone-style visual corruption playground for SWF files, powered by [Ruffle](https://ruffle.rs/).

## Features

- Load a local SWF by browsing or drag-and-drop.
- Play Flash content through Ruffle.
- Apply real-time visual effects: hue shift, contrast, saturation, CRT scanlines, noise, jitter, and glitch bars.
- Randomize effects manually or enable automatic randomization.
- Reset the effect stack at any time.

## Important

RTCFLE changes only the displayed image. It does not patch, rewrite, or upload the original SWF. The selected file stays in the browser session. Ruffle is loaded from its official CDN, so an internet connection is required.

## GitHub Pages

The workflow in `.github/workflows/deploy.yml` automatically publishes the site when changes are pushed to `main`. In the repository settings, make sure **Settings → Pages → Build and deployment → Source** is set to **GitHub Actions**.

Site URL: https://10574084-cmd.github.io/RTCFLE/
