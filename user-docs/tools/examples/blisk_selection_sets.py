# SPDX-License-Identifier: AGPL-3.0-or-later
"""Screens of the worked example "Selection sets on a blisk" (user-docs/src/examples/selection-sets.md).

Run with `python3 user-docs/tools/capture_example.py blisk_selection_sets` after building the release
program and the STEP reader (`cargo build --release -p openchuck-ui` and
`cargo build --release -p openchuck-import --features kernel --bins`), with `xclip` installed.
The click positions belong to the layout of the build they were taken with, at 1920 x 1080;
change them and the page together.
"""
import _blisk as b

PROJECT = b.PROJECT


def run(cap, work):
    b.require_xclip(cap)
    b.front_view(cap)
    b.new_set(cap, "faces", [b.HUB_FLOOR])
    cap.shot("01-new-selection")
    b.save_set(cap, "Hub floor")
    cap.shot("02-face-set")
    b.leave_picking(cap)
    b.new_set(cap, "edges", [b.GUIDE_EDGE])
    b.save_set(cap, "Floor guide")
    cap.shot("03-edge-set")
    b.leave_picking(cap)
    b.new_set(cap, "faces", [b.BLADE_WALL])
    b.save_set(cap, "Blade wall")
    b.leave_picking(cap)
    b.new_set(cap, "faces", [b.RIM])
    b.save_set(cap, "Rim")
    cap.shot("04-four-sets")
    b.leave_picking(cap)
    b.add_operation(cap, "face_flow", "ball", "Floor finish")
    cap.click(1820, 524, 1)                    # Cut on: Add a set...
    cap.click(1810, 557, 1.5)                  # Hub floor
    cap.click(1820, 588, 1)                    # Guide edge: Add a set... (under Hub floor's line)
    cap.click(1810, 621, 1.5)                  # Floor guide
    cap.click(1820, 716, 1)                    # Check: Add a set...
    cap.click(1810, 813, 1.5)                  # Blade wall
    cap.scroll(1740, 700, 6)                   # down to Pattern and tolerance
    cap.fill(1815, 888, "0.2")                 # Stock to leave
    cap.scroll(1740, 500, 10, down=False)
    cap.shot("05-faceflow-sets")
    b.generate(cap)
    cap.click(450, 790, 1)                     # Results
    cap.shot("06-faceflow-result")
    b.dump_results(cap, work, "faceflow")
    b.add_operation(cap, "swarf", "flat", "Blade wall swarf")
    cap.click(1810, 429, 1)                    # Cut on: Add a set...
    cap.click(1810, 493, 1.5)                  # Floor guide: the wrong kind of set, on purpose
    cap.shot("07-wrong-kind")
    cap.click(1890, 633, 1.5)                  # the x at the end of Floor guide's line, under the refusal box
    cap.click(1810, 429, 1)                    # Cut on: Add a set...
    cap.click(1810, 525, 1.5)                  # Blade wall
    b.generate(cap)
    cap.click(455, 790, 1)                     # Results
    cap.shot("08-swarf-before")
    b.dump_results(cap, work, "before")
    cap.click(1810, 525, 1)                    # Avoid: Add a set...
    cap.click(1810, 558, 1.5)                  # Hub floor
    cap.click(1810, 525, 1)                    # Avoid: Add a set... again, on the role's line
    cap.click(1810, 622, 1.5)                  # Rim
    cap.click(1798, 717, 1.5)                  # Rim: Warn only, under Rim's name
    cap.shot("09-swarf-sets")
    b.generate(cap)
    cap.click(455, 790, 1)                     # Results
    cap.shot("10-swarf-after")
    b.dump_results(cap, work, "after")
