# Keyboard and shortcuts

Every command is in a menu, and most have a key. **Help > Keyboard shortcuts** shows the same list in the program. On macOS, <kbd>Ctrl</kbd> is <kbd>Cmd</kbd>.

## Moving around with the keyboard

- <kbd>Tab</kbd> moves to the next control, and <kbd>Shift</kbd>+<kbd>Tab</kbd> to the one before. A panel that scrolls brings the control into view.
- <kbd>Tab</kbd> goes through the window in reading order: the menus, the steps, the status badge, the buttons at the right of the step bar, the project panel, the view's toolbar and the view, the bottom panel (Messages, Jobs, Results), the Inspector, and last the bottom bar, ending on its main button. In a window such as **Post and export**, it goes through the window's contents, then its buttons from left to right, and round again inside the window.
- A control's tooltip opens when the control gets keyboard focus, as it does when you hold the mouse over it.
- A step that is not available yet stays in the <kbd>Tab</kbd> order, and its tooltip says what it needs. <kbd>Enter</kbd> on it does nothing.
- <kbd>Enter</kbd> or <kbd>Space</kbd> presses the button that has focus.
- When a window opens (import, **New operation**, the tool library, the machine library, **Post and export**), the keyboard goes to the window, and the first <kbd>Tab</kbd> takes you to its first field or button.
- <kbd>Esc</kbd> closes a window as its **Close** or **Cancel** button does. In a text field, <kbd>Esc</kbd> ends the edit first, and a second <kbd>Esc</kbd> closes the window. If the tool library or the machine library has changes you have not saved, it asks first.

A disabled button is not in the <kbd>Tab</kbd> order. Hold the mouse over it to read why it is disabled; for the main button at the bottom right, the bottom bar says the same.

## Shortcuts

| Keys | Command |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>N</kbd>, <kbd>Ctrl</kbd>+<kbd>O</kbd>, <kbd>Ctrl</kbd>+<kbd>S</kbd> | New, open, save project |
| <kbd>Ctrl</kbd>+<kbd>I</kbd> | Import model |
| <kbd>Ctrl</kbd>+<kbd>E</kbd> | Post and export |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> | Undo |
| <kbd>Ctrl</kbd>+<kbd>Y</kbd>, <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | Redo |
| <kbd>Alt</kbd>+<kbd>1</kbd> to <kbd>Alt</kbd>+<kbd>4</kbd> | The four steps: Model, Setup, Operations, Simulate |
| <kbd>Alt</kbd>+<kbd>5</kbd> | Post and export (as <kbd>Ctrl</kbd>+<kbd>E</kbd>) |
| <kbd>Ctrl</kbd>+<kbd>1</kbd>, <kbd>Ctrl</kbd>+<kbd>2</kbd>, <kbd>Ctrl</kbd>+<kbd>3</kbd> | Collapse the project panel, the inspector, the dock |
| <kbd>Ctrl</kbd>+<kbd>+</kbd>, <kbd>Ctrl</kbd>+<kbd>−</kbd>, <kbd>Ctrl</kbd>+<kbd>0</kbd> | Scale the interface up, down, back to 100 % |
| <kbd>F</kbd>, <kbd>V</kbd> | Fit the view to the part; open **Show** |
| <kbd>1</kbd>, <kbd>3</kbd>, <kbd>7</kbd>, <kbd>0</kbd> | Front, right, top, isometric view (the number row or the number pad) |
| <kbd>N</kbd> | New operation |
| <kbd>G</kbd> or <kbd>F5</kbd> | Generate the selected operation, and the ones before it |
| <kbd>Shift</kbd>+<kbd>G</kbd> or <kbd>Shift</kbd>+<kbd>F5</kbd> | Generate every stale operation |
| <kbd>Ctrl</kbd>+<kbd>D</kbd>, <kbd>Del</kbd>, <kbd>F2</kbd> | Duplicate, delete, rename the selected item |
| <kbd>Alt</kbd>+<kbd>↑</kbd>, <kbd>Alt</kbd>+<kbd>↓</kbd> | Move the selected operation earlier or later |
| <kbd>Q</kbd>, <kbd>W</kbd>, <kbd>E</kbd>, <kbd>R</kbd> | While picking: select one, flood fill, by angle, same direction. On an edge, <kbd>Q</kbd>, <kbd>W</kbd> and <kbd>E</kbd> are select one, chain and smooth chain |
| <kbd>Ctrl</kbd>+<kbd>Enter</kbd> | Apply the workholding draft |
| <kbd>Space</kbd> | Play or pause the simulation |
| <kbd>←</kbd>, <kbd>→</kbd> | Step the simulation back or on |
| <kbd>Shift</kbd>+<kbd>←</kbd>, <kbd>Shift</kbd>+<kbd>→</kbd> | Go to the previous or next operation in the simulation |
| <kbd>Esc</kbd> | Cancel the running job, leave picking, close a list, or close a window as its **Close** does |

The single-letter and digit keys and <kbd>F2</kbd>, <kbd>F5</kbd> and <kbd>Del</kbd> work whenever no text field has the keyboard, even while a button or menu has focus. <kbd>Space</kbd> and the arrow keys go to the control that has focus first: press <kbd>Esc</kbd> to leave it, then use them for the simulation.

## Not yet available

- There is no screen reader support (see [Known limitations](../start/limitations.md#accessibility)).
