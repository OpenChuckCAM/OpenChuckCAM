# 0655: A copied machine profile keeps its confirmed marks, and the window says where they were made (SET-10, UI-13, NFR-12; issue #603; lane POST-SAFETY)

- **Status:** Accepted by the project owner on 22 Nov 2026, relayed by the coordinator (decision 0049). Proposed by lane POST-SAFETY on 22 Nov 2026; the owner chose option C the same day (relayed): keep the marks as [decision 0380](0380-machine-library-in-the-application.md) item 3 says and only warn. It clarifies decision 0380 item 3 and does not amend it. Built in lane POST-SAFETY's first pull request.
- **Date:** 22 Nov 2026.
- **Requirements:** SET-10 (manage machine profiles), UI-13 (provenance shown), NFR-12 (the user verifies G-code). Related: decisions [0380](0380-machine-library-in-the-application.md), [0013](0013-z1-profile-consensus.md), [0317](0317-z1-measured-axis-height-and-safe-z.md), [0649](0649-the-z1-profile-takes-the-firmware-1-1-3-measurements.md).
- **Open items:** OI-2156 (acceptance of this record; closed), OI-2158 (the user guide's custom machine figures show the window before the warning), OI-2159 (a saved copy does not say it is a copy).

Requirements and architecture are not edited by this record. No machine or project schema changes. No public API of the library family changes.

## Context

`MachineDraft::copy_of` (`crates/openchuck-app/src/machine_library.rs`), used by Duplicate and by Copy to edit on a built-in profile, clones the profile with its provenance, as decision 0380 item 3 says ("A copy keeps these sources until a value is edited"). Every value the user does not edit keeps "Confirmed on the machine" (on the built-in Makera Z1 after decision 0649, every machine value). A copy is often made to describe a different machine (the user guide's *Create a custom machine* example does exactly this), and then values show as confirmed on a machine where nobody confirmed them. The confirmed mark is a safety signal (UI-13, NFR-12).

## Options put to the owner

- **A:** a copy starts with every value not confirmed, its source naming the profile it was copied from (would amend 0380 item 3).
- **B:** ask at the copy whether it is for the same machine.
- **C (chosen):** keep the marks, as decision 0380 item 3 says, and warn.

The lane recommended A. The owner chose C on 22 Nov 2026 (relayed): keep the marks; in the machine library window, and wherever a copy's confirmed marks show, say in plain words that the marks were made on the source machine and must be checked again if the copy is for a different machine, naming the source; advice only, nothing to acknowledge, no schema change.

## Decision

1. **A copy keeps every value and its provenance**, "Confirmed on the machine" included, until a value is edited (decision 0380 item 3, unchanged).
2. **The draft remembers what it was copied from.** `MachineDraft::copy_of` records the source profile's name (`MachineDraft::copied_from`). It is not written to the profile file, so no schema changes.
3. **The machine library window warns above the copy's values** while it shows the copy and the copy has at least one value confirmed on the machine (`MachineDraft::copied_marks_warning`), in the warning colour with the warning mark:
   > This machine is a copy of Makera Z1. The values marked confirmed on the machine were confirmed on Makera Z1, not on this copy. If this copy is for another machine, check each of them on that machine before you rely on it.
   It is advice: nothing changes and nothing has to be acknowledged. It goes when no value of the copy is confirmed any more (each edited value is marked not confirmed, decision 0380 item 3). It stays after the copy is saved, while the window shows it.
4. **Where else the marks show.** The copy's marks show only in the machine library window while the draft is open; the project's copy of a machine is not a Duplicate or Copy to edit and is not changed. Once the window shows another profile or is closed, the saved file cannot tell that it is a copy (item 2), so the warning is not shown for it; that is OI-2159.
5. **The red refusal line** at the foot of the window (the sev3 part of #603) is in [decision 0657](0657-post-safety-fixes-without-a-design-choice.md) item 4.

## Consequences

- `openchuck-app`: `MachineDraft` gains `copied_from` and `copied_marks_warning`. `openchuck-ui`: the window draws the warning above the form and exposes `copy_warning`.
- The user guide's *Create a custom machine* example says the window warns about the copied marks. Its figures 3 and 5 were captured before the warning existed (OI-2158).
- G-code and its wording do not change (decision 0380 item 6).

## Verification

See [decision 0657](0657-post-safety-fixes-without-a-design-choice.md), *Verification*: tests `machine_library::tests::req_set_10_req_ui_13_a_copy_keeps_its_marks_and_warns_where_they_were_made` (`openchuck-app`), `shell::machine_library_tests::req_set_10_req_ui_13_a_copy_says_its_confirmed_marks_were_made_on_the_source` (`openchuck-ui`), and the existing `req_set_10_a_copy_has_a_free_name_and_a_new_id`, which still checks that the copy keeps its provenance.
