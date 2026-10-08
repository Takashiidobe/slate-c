# Upstream

Vendored from the crates.io `slate-parser` 0.1.0 release.
Registry checksum: `bb0a63325ed8857f55bed1adf54882448c017e9447d206e20c087456e7765141`.
Repository: https://github.com/Takashiidobe/slate

Local change: initialize the preprocessor timestamp from JavaScript Date.now()
on wasm32-unknown-unknown, where std::time::SystemTime::now() panics. Native
clock behavior is unchanged. The target-specific js-sys dependency supplies
that clock. Browser date/time macros use Slate's existing UTC fallback.
