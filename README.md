# Icarus

Icarus is a compact repository that keeps the core sources and build outputs for the project. It is set up to publish a simple project page through GitHub Pages.

## Repository layout

The repository intentionally tracks only these folders:

- `src`
- `dist`
- `pkg`
- `js`
- `vm`
- `wasm`

Everything else is ignored by design.

## GitHub Pages

This repo includes a static `index.html` at the root. To enable GitHub Pages:

1. Go to the repository Settings.
2. Open Pages.
3. Set the source to the `master` branch and the root folder (`/`).

After that, the page will be available at:

`https://Ishan-Sreejith.github.io/Icarus/`

## Notes

If you want to include additional files in the repo, update `.gitignore` to unignore them explicitly.
