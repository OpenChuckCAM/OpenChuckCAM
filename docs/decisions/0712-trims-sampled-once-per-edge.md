# 0712: The importer samples each edge once, to 1 µm, for both faces' trims (IMP-02, OP-05, OP-12, NFR-06, NFR-12; issue #917; lane PATH-QUALITY)

- **Status:** Proposed by lane PATH-QUALITY on 28 Nov 2026. It records the owner's answers of 28 Nov 2026 on issue #917, option B (OI-2271), relayed by the coordinator (decision 0049):
  - Q1 F1, the importer fix at the source;
  - Q2 a deflection bound of 1 µm;
  - Q3 the programs of saved STEP projects may change near curved edges;
  - Q4 keep #917 A as a guard, built after this.

  Only the project owner accepts this record (decision 0049).
- **Date:** 28 Nov 2026.
- **Issue:** #917 (STEP blisk: spurious off-axis zones in 0.05 mm strips).
- **Requirements:** IMP-02 (STEP import with the exact B-rep), OP-05 and OP-12 (paths and gouge checks on the exact surface), NFR-06 and GC-07 (determinism), NFR-12.
- **Amends:** the importer's trim rule of slice 11A ([decision 0027](0027-slice-11a-step-import.md); OI-73): straight p-curves sampled at one point and curved ones at 48. OI-73's sampling-density part is settled here; its holes across a seam stay open.
- **Open items:** closes OI-2271. Settles OI-73's sampling-density part (its holes across a seam stay open). Closes OI-1040, which asked for exact trims or a polyline to a stated tolerance (this is the latter, at 1 µm), on the project owner's instruction of 28 Nov 2026, relayed by the coordinator. #917 A, the zone finder's guard, stays as its own change.

Numbering: decision 0712, provisional: the coordinator gives the number at the release push. 0711 is the provisional number of lane PATH-QUALITY's #880 record. No project schema number is needed: the imported model's format is unchanged, and the model is imported again from the project's STEP file each time it is opened.

## Context

The importer sampled every curved p-curve at 48 even steps, separately for each of the two faces on an edge. Each face's trim was then a chord of the true edge, sagging away from it by up to 0.0021 r on a circle (7.5° a step): 0.026 mm at r = 12, 0.043 mm on the blisk. Where a face's chord sags into the face, the strip between the true edge and the chord lies on the face's surface but outside its trim, and the neighbour's surface is not there. A ray into the strip hit nothing, and a point there belonged to neither face.

Measured on the public STEP parts, aiming rays along the normal at points stepped in from every shared edge, the strip was outside both faces on every part with curved edges. At 10 µm in, for example:
- blisk: 168 of 2137 rays missed;
- cylinder: 104 of 288;
- shouldered shaft: 204 of 956.

The widest misses were the sag, 0.02 to 0.04 mm. Parts with only straight edges had none.

The effect on output was measured with a throwaway fallback that counted a hit within 0.05 mm of the trim:
- Simultaneous finishing on the shouldered shaft gained a whole pass at X 25.5 and longer passes at the shoulder edge, 833 moves becoming 971. Both programs passed the independent gouge check.
- Seven other STEP jobs were unchanged.
- With this decision built, the same shaft job is unchanged (833 moves, and the gouge check passes). So that pass came from the fallback's 0.05 mm band, which reaches past the true edges, not from the slivers themselves. The slivers' effect on programs found so far is the change in the face-flow reference program, below, which is within one written digit.

## Decision

1. **Each edge is sampled once.** Along its 3D curve, to a chordal deflection of 1 µm (OCCT's tangential deflection, with a tangent turn of at most 0.5 rad a step, so that the deflection binds), and worked out once per edge.
2. **Both faces' trims take their points from their own p-curves at those same parameters.** On a same-parameter edge, the p-curves share the 3D curve's parameter, so the two trims meet at the same 3D points, and each stays within about 1 µm of the true edge.
3. **Kept:**
   - A straight p-curve keeps its single start point, which is exact in (u, v).
   - A degenerate edge (a cone's apex, a sphere's pole), which has no length, keeps 48 even steps.
   - An edge that is not same-parameter is sampled to the same deflection along its curve on each face, since its p-curve does not share the 3D curve's parameter. None of the public STEP parts has one.
   - A seam edge is sampled once for each of its two p-curves on its face.
   - A converted face's trim points are still projected onto its patch (`remap_to_patch`).
4. **The output is deterministic.** The same file gives the same points.

Only `openchuck-import`'s C++ wrapper changes, so the STEP kernel stays isolated there (architecture rule). The pathing library's output for the same input is unchanged, so the library family needs no new version. The imported model's format is unchanged; trims carry more points.

## Visible change

The programs of STEP projects can change near curved edges at their next Generate. On the public parts the changes found are slight: the face-flow reference program on the blisk moves by at most one written digit (0.001 mm, 0.005° of A). Regenerate and check them as usual (NFR-12). There is no migration and no stored marker, the owner's answer Q3. STL projects are unchanged.

## Verification

- New kernel test, written first and seen failing on the old importer: `step_exact::req_imp_02_a_ray_just_inside_a_shared_edge_hits_its_face`, in `openchuck-app`, with `--features kernel-tests`.
  - What it checks: on every public STEP part with shared edges, a ray aimed along the normal at a point 2 µm and 10 µm in from each point of a shared edge between its ends hits that face. The face's own trimmed hits are asked, so a neighbour standing over it (a blade wall over the floor at its root) does not hide it. An edge's ends are left out: at a corner, a step across the edge can land on the next edge.
  - Before, it failed on 8 parts:
    - blisk 195 of 4016;
    - cone 188/552;
    - crankshaft 270/1824;
    - cylinder 240/560;
    - extruded spline 42/400;
    - multi-body 204/496;
    - rotary blank drilled 270/1360;
    - shouldered shaft 472/1880.
  - After, no ray misses on any part.
- On the blisk, the 3D curve and both faces' p-curve images agree within 6 µm at the shared parameters, with every edge same-parameter (tolerance 1e-7 mm).
- Import time on the blisk is unchanged, about 0.15 s by the worker alone, old and new alike. The imported model grows from 1.29 to 1.33 MB.
- The face-flow reference job on the blisk (`step_pipeline::req_op_28_req_gc_07_the_committed_face_flow_project_writes_the_golden_programs`) writes a different program.
  - Its golden files are blessed again.
  - Over its 1554 lines, the moves are the same: each coordinate moves by at most one written digit (0.001 mm, 0.005° of A).
  - 261 lines change only in which modal words are written (an F or Y repeated or left out).
  - Its checks are clean through both posts.
- Suites in release:
  - `openchuck-import --features kernel` passes;
  - `openchuck-app --features kernel-tests` passes, including every STEP test binary: step_accuracy 5, step_drilling 1, step_exact 3, step_pipeline 14, step_queries 2, step_selection 2 and step_strategies 15;
  - `openchuck-app`: 318 unit (1 ignored) and 645 integration tests pass;
  - `openchuck-path`: 160 unit and 954 integration tests pass.

Not verified: no air or wood cut. Generated G-code remains the user's to verify before it is run on a machine (NFR-12).
