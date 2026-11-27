# Release notes

What changed in each release of OpenChuckCAM. Each release adds a section headed with its version number, newest first.

## 0.60.0

<!-- release-notes draft: made by cargo xtask changelog start-release. Write the release's summary, the known limitations, the unconfirmed Makera Z1 values and the known issues (docs/releasing.md), edit the changes below for users (in each section, the fixes may go last), then delete this comment. -->

### UI and workflow

- **Keyboard.** A window that opens takes the keyboard (#580). See [Keyboard and shortcuts](reference/keyboard.md).

### Operations and strategies

- **Facing.** Facing works on a STEP part,
  bounded from its exact faces (#800).
  - Flat and round shapes as before.
- **Fixed: the stop marker.** It marks the real stop (#895).

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.

## 0.55.0

The first release of OpenChuckCAM, an alpha.

### Known limitations

- **Very little has run on a real machine.**

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.

## Development version

The *latest* version of this guide describes the nightly build, the [development build](start/install.md#download-a-development-build) made each night from the main branch.

<!-- The sections, in this order: UI and workflow. -->

<!-- Add a bullet at the end of its section above. -->

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.
