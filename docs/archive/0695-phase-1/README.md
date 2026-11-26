# 0695, phase 1: reproduction notes

Raw notes of lane PATH-QUALITY's phase 1 investigation, cited by [decision 0695](../../decisions/0695-finishing-edges-points-and-ramps.md) (Context and Verification). Each file gives the commands, the settings, the messages and the numbers of its runs, on `main` at `d1674b7c5ba33e15227dff542814eeff1722a23e` (0.63.0), on the repository's public models only (the blisk, the baluster, the crankshaft and the stepped first-cut part). No program or project is stored here; the scratch projects were not kept.

| File | What it holds |
| --- | --- |
| `blisk-simultaneous-and-engagement.md` | Issue #710: simultaneous finishing on the STL and STEP blisk with a 3 mm ball and flat end, generation and simulation; the causes in `simultaneous.rs`; issue #844's engagement measurement on the crankshaft example. |
| `finial-tip-and-adaptive-spacing.md` | Issue #779: the radius the finish leaves at the baluster's finial tip under each setting; OI-2198 (issue #826): adaptive spacing's warning on the stepped first-cut part and its causes. |
| `ramp-entry-survey.md` | Issue #786: Entry = Ramp in every strategy, the simulated overcuts with Ramp and Plunge, the refusals; issue #421's check on STEP parts. |

These are working notes written during the investigation; where they differ from the record, the record holds. Generated G-code remains the user's to verify before running it on a machine.
