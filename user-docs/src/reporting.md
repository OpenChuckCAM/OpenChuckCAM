# Reporting a problem or asking for a feature

Use the project's issue forms on GitHub for a bug or a feature. [Open the issue chooser](https://github.com/OpenChuckCAM/OpenChuckCAM/issues/new/choose) and pick **Bug report** for something that does not work, or **Enhancement request** for a feature or a change. Questions are not issues: ask them in [GitHub Discussions](https://github.com/OpenChuckCAM/OpenChuckCAM/discussions) (see [Asking a question or sharing your work](#asking-a-question-or-sharing-your-work) below). The maintainer reads every issue and sets its priority. A request can be accepted, changed or recorded for a later phase; opening one does not promise a date.

Check [Known limitations](start/limitations.md) and [Troubleshooting](troubleshooting.md) first: what you found may already be listed.

> [!WARNING]
> Generated G-code is yours to verify before you run it on a machine. OpenChuckCAM does not guarantee that a program is safe or correct. If G-code looks wrong or unsafe, report it and do not run it. If you ran a program on a machine, say so in the report, and say what happened. [Checking G-code before you run it](safety/checking-gcode.md) lists what to check.

## What to include in a bug report

- What happened, what you expected, and the steps that lead to it. Numbered steps that someone else can follow are the most useful part.
- The version of the program: **Help > About** shows it, and `openchuck-cli --version` prints it for the command line. Give your operating system and its version.
- The machine and post you used, if the problem is in the G-code or the export.
- Your guess at the severity. The maintainer sets the final one. The form's four choices map to the repository's labels: Critical is `sev1`, Major is `sev2`, and Minor and Trivial are both `sev3`.
- A small model that shows the problem, if you can make one.

## Asking a question or sharing your work

Go to [Discussions](https://github.com/OpenChuckCAM/OpenChuckCAM/discussions) and pick a category; each has a short form.

- **Q&A** for how to do something, or something the guide does not explain.
- **Machines and post processors** for a machine, its controller and firmware, or a post.
- **Ideas** for a feature you would like; an idea the maintainer takes up becomes an enhancement request.
- **Show and tell** for what you made. Sharing a result does not mean the project has verified the G-code.

The maintainer answers when time allows; there is no promised reply time. Everyone taking part follows the [code of conduct](https://github.com/OpenChuckCAM/OpenChuckCAM/blob/main/CODE_OF_CONDUCT.md).

## Labels on your issue

Every form adds two labels. `user-reported` means the issue was filed through the forms. `awaiting-triage` means the maintainer has not yet looked at it and set its severity, labels and any milestone; it is removed when they have. Each form adds one more label: `bug` for a bug report and `enhancement` for an enhancement request. The maintainer may add `question` to an issue that is really a question.

## Issues and discussions are public

Everything you post on GitHub, in an issue or a discussion, including attachments, is visible to everyone. A project is a folder that holds your model and your settings. Attach files only if you are happy to share them. If you can, repeat the problem with one of the example models in the repository and attach that; otherwise describe the model.

## The log file

The program keeps a log file, `openchuckcam.log`, on your computer. **Help > About** has a button that opens its folder; [Where the program keeps its log](start/install.md#where-the-program-keeps-its-log) lists the folders. It holds:

- what happened at start-up, the graphics adapter, warnings and errors;
- the text of a crash;
- a list of the names of the actions you took, such as the buttons you pressed and the commands you ran. It never holds the values you typed or the contents of files. **Settings** can turn this list off.

It also holds the paths of project files, which can include your user name. The program sends nothing to the project: there is no telemetry, and the log leaves your computer only if you post it. Read it before you attach it, remove anything you do not want to share, and attach the part around the problem.
