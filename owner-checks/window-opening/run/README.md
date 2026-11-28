# Run record for the window-opening check

Release builds on Linux x86_64, on Xvfb at 1920 × 1080 with the openbox window manager. `wmctrl` resized, moved, maximised and closed the window as a user would; `wmctrl -lG` read where it was (x, y, width, height; y includes openbox's title bar). Fresh settings folder for each build. 25 Nov 2026.

- **After:** PR #906 at `d2a5b546` (branch `claude/lane-display-target-a`).
- **Before:** `main` at `361613e` (0.63.0).

| Step | PR #906 | `main` (before) |
| --- | --- | --- |
| Fresh settings, first start | 800 × 600 at (561, 280), **not maximised**: openbox on Xvfb does not honour a maximise request at start (`main` asks for it too and is not maximised either) | 800 × 600 at (561, 267), not maximised (same reason) |
| Window set to 1400 × 900 in the top left corner, closed | settings: `maximised = false`, `window = [1400.0, 900.0]` | settings: `window = [1400.0, 900.0]` |
| Start again | 1400 × 900 at (262, 130): **centred** | 1400 × 900 at (261, 117): openbox placed it near the centre |
| Maximised (1920 × 1061), closed | settings: **`maximised = true`, `window = [1400.0, 900.0]`** (the normal size kept) | settings: **`window = [1920.0, 1061.0]`** (the maximised size saved as the normal one) |
| Start again | 1400 × 900 at (262, 130): centred at the kept normal size; **not maximised here**, for the reason in the first row | **1920 × 1061 at (2, 40)**: a screen-sized window, not maximised, running 21 px past the bottom of the screen. This is the bug |

The two `.toml` files here are the settings files at the end of each run (after the last start and close), not snapshots of each row. The PR's file reads `maximised = false` at the end because the last start was not maximised on openbox.
