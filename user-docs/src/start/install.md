# Install and first run

OpenChuckCAM's releases so far are early alphas. Each is on the project's [releases page](https://github.com/OpenChuckCAM/OpenChuckCAM/releases) and described in the [release notes](../release-notes.md). You can also download a development build, or build the program from its source code.

> [!NOTE]
> A development build has passed none of the project's release checks. Use it to learn the program and to try it on your own parts, and check every program it writes before you run it on a machine (see [Checking G-code before you run it](../safety/checking-gcode.md)).

## Download a development build

When the program's code changes, the project builds it for three systems and publishes the result as the rolling [`nightly` pre-release](https://github.com/OpenChuckCAM/OpenChuckCAM/releases/tag/nightly) on GitHub. The pre-release page lists what has changed since the last release, as does the [Development version](../release-notes.md#development-version) section of the release notes in this version of the guide.

| System | Archive | Notes |
| --- | --- | --- |
| Linux | x86-64 | Needs glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 or later), a Vulkan or OpenGL driver, and X11 or Wayland. |
| Windows | x64 | Not signed: Windows SmartScreen may warn before it runs. |
| macOS | Apple silicon, macOS 11 or later | Not signed or notarised: macOS blocks it the first time (see below). |

The programs are not signed, and no release so far has been signed.

Each archive holds:

| File | What it is |
| --- | --- |
| `openchuck-ui` (on macOS, `OpenChuckCAM.app`) | The program with a window. This is the one most people use. |
| `openchuck-cli` | The command-line program, which does the same work without a window. |
| `openchuck-step-worker` | The reader for STEP files. Keep it in the same folder as the other two. On macOS the application carries its own copy inside `OpenChuckCAM.app`, and this one is for `openchuck-cli`. |
| Licence and README | The licence texts, the third-party notices, the credits for the Makera Z1 tool data, and notes for the system. |

The same page also offers `openchuckcam-...-examples.zip`, the example projects (such as the tutorial's baluster). It is not needed to run the program. Generated G-code must be verified by you before it is run on a machine.

To install it:

1. Download the archive for your system from the [`nightly` pre-release](https://github.com/OpenChuckCAM/OpenChuckCAM/releases/tag/nightly).
2. Unpack it into a folder of its own.
3. Start `openchuck-ui`, or on macOS open `OpenChuckCAM.app`, as below for the first time.

**macOS, the first time.** macOS says "OpenChuckCAM" Not Opened, because Apple could not check it. Click **Done**. Open **System Settings > Privacy & Security**; under **Security** there is a line about OpenChuckCAM: click **Open Anyway** and confirm with your password. After that it opens normally. Alternatively, in Terminal, in the unpacked folder, remove the download's quarantine flag:

```text
xattr -dr com.apple.quarantine OpenChuckCAM.app openchuck-cli openchuck-step-worker
```

**Windows, the first time.** If SmartScreen shows "Windows protected your PC", click **More info**, then **Run anyway**. Or, before unpacking, right-click the zip file, choose **Properties**, tick **Unblock** and click **OK**.

**Linux.** Nothing extra: the archive has no signature to check. If the program does not start, check that your system meets the needs in the table above.

### The command line

`openchuck-cli` does the same work as the window, from a terminal: it creates a project, imports a model, sets up the machine, stock and tools, generates the toolpaths, writes the G-code and simulates. In the unpacked folder, run:

```text
./openchuck-cli --help
```

On Windows, run `openchuck-cli.exe --help`. On macOS, run the `openchuck-cli` that sits beside `OpenChuckCAM.app`. [Command line](../reference/command-line.md) describes each command.

## Build from source

The program is written in Rust. The version of Rust it needs is pinned in the repository's `rust-toolchain.toml`, and `rustup` installs it for you.

1. Install Rust with [rustup](https://rustup.rs).
2. Get the source: `git clone https://github.com/OpenChuckCAM/OpenChuckCAM.git`.
3. In the `OpenChuckCAM` folder, build and start the program:

   ```text
   cargo run --release -p openchuck-ui
   ```

4. For the command line, run `cargo run -p openchuck-cli -- --help`.

Reading STEP files needs the OpenCASCADE geometry kernel, which is not built by default and takes a long time to build the first time. The [`openchuck-import` README](https://github.com/OpenChuckCAM/OpenChuckCAM/blob/main/crates/openchuck-import/README.md) says how.

## The first time it opens

The program opens with a start card in the middle of the window: **New project...** (also **File > New project...**, <kbd>Ctrl</kbd>+<kbd>N</kbd>) and **Open project...**, then the projects you opened recently, newest first. A [project](../glossary.md#project) is a folder of plain text files that holds the model, the setup, the tools and the operations of one job.

- **Recent projects.** Choose one, with the mouse or with <kbd>Tab</kbd> and <kbd>Enter</kbd>, and the **Open project** dialog opens with its folder filled in; press **OK** to open it. The same list is under **File > Open recent**. A folder that no longer exists drops off the list, and **Clear recent list** empties it. The list is kept on your computer, never in a project.
- **Dropping a file.** Drag an STL or STEP file onto the window to start a new project with it: **New project** opens with the project named after the file, and after **OK** the import window opens on the file. With a project open, a dropped model opens the import window instead. Drag a project folder (or its `project.toml`) onto the window to open it. While you drag, the window says what the drop will do. Drop one file at a time: of several, only the first that can be used is taken, and **Messages** names the rest.
- **On Linux with Wayland,** dropping files does not work yet: the program sees no drop. Run it in an X11 session (or under XWayland), or use the buttons and the recent list, which work everywhere.

The program does not save by itself. Use **File > Save** (<kbd>Ctrl</kbd>+<kbd>S</kbd>) as you work; the window title says when there are changes you have not saved. As a safety net the program also keeps recovery copies of an open project (**File > Settings** sets how often), and offers to recover one when the project is opened again after a crash. They do not replace saving: a recovered project is kept only until you save it, and Messages warns "Recovered checkpoint" until you do. Edits in a form take effect when you press <kbd>Enter</kbd> or leave the field, and **Edit > Undo** reverses them.

Next: [Your first project](../tutorial/index.md) takes you through one job from start to finish.

## Where the program keeps its log

The program writes a log file, `openchuckcam.log`, with what happened at start-up, the graphics adapter, warnings and errors. It never leaves your computer. **Help > About** has a button that opens the folder it is in.

| System | Folder, when the program's own folder cannot be written |
| --- | --- |
| Linux | `~/.local/state/openchuckcam` |
| macOS | `~/Library/Logs/OpenChuckCAM` |
| Windows | `%LOCALAPPDATA%\OpenChuckCAM\logs` |

Otherwise the log is in the program's own folder. When you report a problem, you can attach it if you choose to.
