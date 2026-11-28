# SPDX-License-Identifier: AGPL-3.0-or-later
"""Screens of the worked example "Tabs and held regions" (user-docs/src/examples/tabs-and-held-regions.md).

Run with `python3 user-docs/tools/capture_example.py tabs_held`. The click positions belong to
the layout of the build they were taken with, at 1920 x 1080; change them and the page together.
"""

GENERATE = 90  # seconds: both operations of the baluster generate in well under a minute


def hover_shot(cap, x, y, name):
    """A shot with the mouse left on (x, y), so its tooltip shows (shot() parks the mouse)."""
    cap.move(x, y)
    cap.pause(2.0)
    cap.grab(cap.OUT / f"{name}.png")
    print("captured", f"{name}.png", flush=True)


def run(cap, work):
    cap.click(165, 54, 2)                      # 2 Setup
    cap.click(125, 342, 2)                     # Setup 1 > Stock
    cap.fill(1810, 255, "78")                  # Stock length
    cap.fill(1810, 287, "4.99")                # Start X: 5 mm of bar between the grip and the part at 17.99
    cap.click(98, 426, 2)                      # Setup 1 > Workholding
    cap.shot("01-zones")
    cap.click(1632, 974, 2)                    # Head tab
    cap.key("Escape")
    cap.scroll(1750, 600, 15)
    cap.click(1620, 974, 2)                    # Tail tab (the Inspector is wider with Tab 1)
    cap.key("Escape")
    cap.scroll(1750, 600, 15)
    cap.shot("02-head-and-tail-tabs")
    cap.click(1878, 1016, 6)                   # Apply
    cap.click(1840, 1057, 3)                   # Continue to operations
    cap.click(1855, 1057, GENERATE)            # Generate
    cap.fill(1810, 802, "2")                   # Tab 2 Length
    cap.scroll(1750, 600, 10)
    cap.shot("03-length-vs-tools")
    cap.click(1878, 1016, 6)                   # Apply: stored, with the warning on the tab's row
    cap.click(1855, 1057, GENERATE)            # Generate
    cap.shot("04-tab-length-warnings")
    hover_shot(cap, 114, 550, "04a-needs-acknowledging")  # the warning mark on 1 Rough (decision 0538)
    cap.scroll(1750, 600, 2, down=False)       # back up past the Kept clear row to Grip length
    cap.fill(1810, 274, "14")                  # Grip length: the held region now reaches 1 mm into the part
    # (13 mm would leave a head zone of 0.0002 mm, from the samples' shift of 17.990234375 mm)
    cap.pause(2)
    cap.shot("05-no-room")
    cap.scroll(1750, 600, 10)
    cap.click(1878, 1016, 6)                   # Apply: stored, with a warning for the tab with no room
    cap.shot("06-apply-warns")
