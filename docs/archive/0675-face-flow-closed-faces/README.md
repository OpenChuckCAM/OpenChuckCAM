# Decision 0675: face-flow finishing on the public blisk, screenshots

Cited by [decision record 0675](../../decisions/0675-face-flow-on-a-face-closed-round-its-axis.md) (issue #791) and by issue #862.

All four are screenshots of the program itself (release build of branch `claude/lane-faceflow` at commit `e4ac7b31`, Linux, Xvfb with Mesa's lavapipe software Vulkan, window 1920 x 1080; the viewport cropped where noted). They were taken on copies of the public reference project `testdata/projects/face_flow` (`testdata/step/valid/blisk.step`, 3 mm ball) with the stepover set to 1 mm and the face set changed as each line says; no private project was used. Colours: orange cuts, green links, lilac entries, hatching unreached.

- [791-blade-face7.png](791-blade-face7.png) (viewport): face 7, a blade, seen from below. The passes run along the blade's tip edge (edge 3, 59.1 mm) and step towards its root; 11 of 13 planned passes cut, 60 mm2 by the root unreached. Blades do not close round on themselves, so this is the guide-edge path of decision 0525, unchanged by 0675.
- [791-rim-face1.png](791-rim-face1.png) (viewport, front view): face 1, the outside cylinder at radius 20 that closes round on itself, cut in 52 rings (the path decision 0675 adds): full rings over the rim from X 10 to 20, then pieces of rings on each blade's tip, with the gaps between the blades. 88,614 moves, 40.5 s.
- [face15-fragments.png](face15-fragments.png) (viewport) and [face15-full-window.png](face15-full-window.png) (whole window, with the Results): face 15, the end face at X 60, on the guide-edge path, unchanged by 0675: 3 of 34 planned passes cut, broken into thousands of pieces, 92,543 moves, 361 s, 125 unreached areas, some reported as "inf mm2". Issue #862.

The images are reduced to 256 colours to keep the files small.
