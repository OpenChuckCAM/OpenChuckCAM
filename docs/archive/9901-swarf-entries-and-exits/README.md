# 9901: swarf entries and exits against the faces next to the face set (issue #572)

Raw evidence cited by [decision record 9901](../../decisions/9901-swarf-entries-and-exits-meet-the-faces-next-to-the-face-set.md) (number provisional), lane SWARF-LEADS, 23 Nov 2026. Nothing here verifies generated G-code (NFR-12).

| File | What it is |
| --- | --- |
| `blisk_entries.rs` | The helper test that made both logs: swarf on the public blisk (`testdata/step/valid/blisk.step`), blades 7, 9, 13 and 19, the 3 mm flat end mill, eight entry variants each, the faces next to each blade given (`WITH_NEIGHBOURS=1`), each entry and exit move checked by the test kit's bounded checker against every face. Paste it at the end of `crates/openchuck-app/tests/step_strategies.rs` and run `WITH_NEIGHBOURS=1 OPENCHUCK_STEP_WORKER=<release openchuck-step-worker> cargo test --release -p openchuck-app --features kernel-tests --test step_strategies zz_blisk_572 -- --ignored --nocapture`. |
| `blisk-main-0ff4b13.txt` | Its output with the new check switched off, which is `main` `0ff4b13`'s behaviour. |
| `blisk-option-b.txt` | Its output on branch `claude/lane-swarf-leads` with the check built as option B. |
