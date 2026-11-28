# Release notes

What changed in each release of OpenChuckCAM. Each release adds a section headed with its version number, newest first.

## 0.55.0

The first release of OpenChuckCAM, an alpha.

### Known limitations

- **Very little has run on a real machine.**

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.

## Development version

The *latest* version of this guide describes the nightly build, the [development build](start/install.md#download-a-development-build) made each night from the main branch.

<!-- The sections, in this order: UI and workflow. -->

### UI and workflow

- **Keyboard.** A window that opens takes the keyboard (#580). See [Keyboard and shortcuts](reference/keyboard.md).

### Operations and strategies

- **Fixed: the stop marker.** It marks the real stop (#895).
- **Facing.** Facing works on a STEP part,
  bounded from its exact faces (#800).
  - Flat and round shapes as before.

<!-- Add a bullet at the end of its section above. -->

Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.
