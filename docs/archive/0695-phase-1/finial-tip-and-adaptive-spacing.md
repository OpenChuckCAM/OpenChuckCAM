# Finial nub (#779) and OI-2198 (#826): investigation notes

main commit: d1674b7c5ba33e15227dff542814eeff1722a23e (release build of openchuck-cli at that commit).
Nothing in the repository was edited; `git status` clean. Scratch tools: tools/nub.py (ball-envelope radius left
per X from the LinuxCNC G-code, tool T204, R 1.5875), tools/variant.sh (baluster variants), tools/stepped.py
(first-cut stepped part STL, port of testkit `revolve`), tools/fcvar.sh (first-cut variants).
An instrumented copy (env OC_DEBUG_PLACES printing every adaptive Place and helix x0/x1/keep_out) was built from
`git archive HEAD` in scratch with its own target dir, then deleted (patch: eprintln after `let first =
places.first()?;` in adaptive.rs `warning`, and after `let (x0, x1) = ...` in helical.rs).

## Part 1: baluster finial (#779)

Reproduce: copy testdata/examples/baluster/baluster-project; edit Finish op [parameters]; then
`openchuck-cli regenerate P --post linuxcnc --tool-numbers keep --acknowledge-collision-warnings --out P/out`.
G-code X = part X (work X = part offset 17.990); Z = tip height above the axis (no Y words). Notes report setup X
(free end "X 78.990" = part X 61.0). Stock is a 24 mm cylinder ending at part X 62.0.

Radius left by the finishing tool envelope (min over all A) per part X; true sphere: X60 3.00, X60.5 2.18,
X60.75 1.56, X61 0:

| variant (Finish params) | 60 | 60.5 | 60.75 | 61 | 61.25 | 61.5 | 61.75 | 62 | 62.5 | 63 | 64 | time |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| def (as saved: adaptive off, free ends on by default) | 3.015 | 2.741 | 2.680 | 2.660 | 0.733 | 0.431 | 0.239 | 0.113 | 0.002 | 0.054 | 0.862 | 13 s |
| adaptive on | identical | | | | | | | | | | | 57 s (77 passes added, 502490 queries) |
| face_free_ends=false | 3.015 | 2.741 | 2.680 | 2.660 | 2.680 | 2.741 | 2.848 | 3.015 | 3.728 | - | - | 13 s |
| extend="far end" | as def to 64; 65: 3.115 | | | | | | | | | | | 14 s, 6.4 turns, 106 mm |
| extend far, free ends off | 3.015 | 2.741 | 2.680 | 2.660 | 2.660 ... 2.660 to X 63, 2.664 at 64 | | | | | | | 15 s |

- The helix's last point is X 61.000 Z 2.660 (tip). The free-end rings are at tip X 62.588 (= 61 + R), Z 4,3,2,1,0.
- Adaptive spacing does refine near the tip (moves in X 58-61 rise 2.6x) but changes nothing past X 61: its family
  only spans the helix's X range.

Causes:
1. helical.rs:371-372 the helix spans the target's bounds (tip X up to 61.0): at the end the ball's side rides the
   sphere's flank, tip 2.66 above the axis.
2. extend.rs:382 (turning, helical) and :466 (line, axial/indexed): run-out height = max(drop, end height); None
   (nothing below) holds the end height. It can never descend past the end.
3. faces.rs:458 free end at x = outer + clear, i.e. tip one grown radius past the last X where the tool touches;
   finishing.rs free_ends cuts rings there to tip Z 0: the ball's round side leaves the cusp between X 61 and 62.6.
- True point needs tip X 61+R with ball centre on the axis (tip at -R), i.e. cutting the end free (slug past X 61).

## Part 2: OI-2198 (#826)

Reproduce: fc/ built with the CLI as e2e.rs build_first_cut (new, import STL from tools/stepped.py, machine
makera-z1, setup with the same flags -> setup TOML identical to testdata/projects/first_cut except ids, tool
"Z1 O-flute 3.175x25" from crates/openchuck-project/defaults/z1). Operations copied from
testdata/projects/first_cut/operations with setup id replaced.
- Finish as committed (adaptive off): 21 s, no warning.
- Finish with adaptive on (exact_contact, face_free_ends=false, max_angle_step 5) and the pure defaults (empty
  [parameters]): exact warning reproduced: 23 places, first near X 52.900 A 5653.3, 26 passes added, 999940
  queries (44 s / 66 s).
- pass_shape="rings" with adaptive on: no passes added, no warning (55 s).

Instrumented places (setup X): HELIX x0=17.990 x1=117.990 keep_out=[(15.903, 88.078), (113.903, 120.078)].
- 10 places X 52.83-54.57, A 5637-5655: inside the start held region's keep-out (Spacing x4, Queries x6).
- 6 places X 99.57-100.27 (face 97.990 + R 1.5875 = 99.578): Queries x5, Unverified x1.
- 6 places X 107.56-108.26 (face 105.990 + R = 107.578): Queries.
- 52.900 = x0 17.990 + pitch 2.2225 * 5653.3/360 (helical.rs:961-968 pose). Setup frame, not a frame error.
- Added passes land on the part at the step walls (G1 moves X 98-100 +259, 106-108 +243, 88-90 +40).

Causes:
1. CompositeSurface::bounds (composite.rs:43-66) includes held-region and tab solids; helical spiral takes
   x0,x1 from it (helical.rs:371-372) and adaptive measures over the held solid; keep_clear (strategy.rs:579) only
   removes those passes later. Rings use ring_span (helical.rs:749-768) and simultaneous uses outside_held
   (simultaneous.rs:558), so they skip it. Queries burned there; first place reported there.
2. Step walls: a flat end's height is a step function across the wraps at X = wall + R (wrap on the upper step vs
   on the lower). Halving never removes a jump: Spacing/Queries. The shoulder turn (decision 0297) at exactly
   wall + R cuts that material, but it is a separate pass added after the helix and not a track of the measure.
   Not a scallop-formula error for flat ends: on cylinders nothing is added (rings run).

## Options

### #779
- A: run-out follows the drop down past the part's end (extend.rs:382/466: height = drop where found; where none,
  either hold (A1) or go to the axis / ball centre on axis with a "part cut free" warning (A2)). Only with Extend on
  (default "no"), so no committed golden changes (no testdata uses extend). Saved ops with Extend on change G-code
  (amends 0212 B item 3 / As built 4, 0546 item 3). Must still rise over tabs (OP-21), respect keep-out (SET-09),
  interact with free-end turn (0344: same X band; turn becomes redundant at a pointed end), cut-free warning reuse
  (finishing.rs). Effort M; risk medium (descending into a parted slug, gouge near tabs); API: none
  (Extension crate-internal), minor version bump only for behaviour/changelog.
  - A' (needed for default settings to get a point): end the helix past the part (x1 + grown R, or to where the
    drop finds the part) without Extend: changes every helical finishing golden (cone, sphere, groove,
    cone_held_tabs, first_cut, blisk/reference if helical) unless gated by a new parameter/schema migration. L.
- B: A plus adaptive spacing over the run-out (extend the adaptive family's c range past x1). Adaptive already
  refines curvature inside the range (2.6x moves X58-61) but cannot reach past it. Default-on adaptive means new
  ops get it; goldens all adaptive off -> unchanged. Effort L; risk: query budget (already met on first_cut).
- C: note only (Past the ends topic): "past X 61 the end keeps up to 2.66 mm ... turn on Extend / face". S, no
  golden change (notes are not in G-code; check setup sheet/notes goldens).

### OI-2198
- F1: helical spiral: run adaptive measure (and/or the helix fit) only over outside_held/ring_span range.
  F1a mask stations in keep-out bands (helix phase kept: adaptive-off G-code identical): S, low risk, no golden change
  (all goldens adaptive off). F1b start the helix at the band edge: changes phase -> golden change for helical
  with held regions (cone_held_tabs, first_cut): M.
- F2: count the shoulder/face turns (0297) as tracks of the measure, or exclude stations within the grown radius
  band of a turn's X: removes the step-wall non-convergence. M, medium risk (proving coverage), no golden change.
- F3: report places with the frame named ("setup X") and skip places in keep-out; S.
