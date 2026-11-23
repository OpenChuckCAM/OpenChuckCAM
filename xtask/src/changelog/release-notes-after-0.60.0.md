# Release notes

What changed in each release of OpenChuckCAM. Each release adds a section headed with its version number, newest first.

## 0.60.0

<!-- release-notes draft: made by cargo xtask changelog start-release. Write the release's summary, the known limitations, the unconfirmed Makera Z1 values and the known issues (docs/releasing.md), edit the changes below into groups for users, then delete this comment. -->

### What has changed

- **Keyboard.** A window that opens takes the keyboard (#580). See [Keyboard and shortcuts](reference/keyboard.md).
- **Facing.** Facing works on a STEP part,
  bounded from its exact faces (#800).
  - Flat and round shapes as before.

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.

## 0.55.0

The first release of OpenChuckCAM, an alpha.

### Known limitations

- **Very little has run on a real machine.**

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.

## Development version

The *latest* version of this guide describes the nightly build, the [development build](start/install.md#download-a-development-build) made each night from the main branch.

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.
