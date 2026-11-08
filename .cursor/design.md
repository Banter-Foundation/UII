# Design

The window is one surface. The canvas is the large area. Code is not the main screen.

## Top

A tab bar with three tabs: Design, Animation, and Function. Assets, Components, and Inspect are not tabs.

Under the tabs, a tool row: Select, Frame, Text, Rectangle, Ellipse, Inspect, then Delete, Import, Export, and HTML / CSS.

The active tool is drawn in brackets. Inspect, when it is the active tool, adds one line under the tools. With a selection that line is the type, the size, and the color. With no selection it asks for one.

HTML / CSS opens two editors, HTML and CSS, plus Apply markup and Reload from canvas. It starts closed.

## Body

A layers list on the left, about 200px, and a properties column on the right, about 250px. Both stay visible on Design and Animation. Properties shows type, position, size, and fill for the selection. Text also shows font size. Rectangles and frames also show corner radius. The shortcut list sits under the properties.

The canvas is the area between those columns. Objects are drawn from the document. On the Animation tab they are drawn from the playhead. The selection has a blue outline and eight handles. Shift-click adds to the selection. Dragging on empty space selects what the rectangle covers. Moving snaps edges and centers to other objects and draws a guide. Frames clip their children. A new shape dropped on a selected frame is parented inside it.

The canvas starts empty. There is no sample layout.

Zoom is Ctrl+= and Ctrl+-. Fit is Shift+1. The percentage is on the tool row.

## Animation

Under the canvas, only on the Animation tab: Play / Pause, Key, and a time slider. Key stores x, y, width, and height of the selection at the playhead. Dragging on this tab writes keys, not the design rect.

## Function

The Function tab is the behavior graph: triggers, actions, values, and the wires between them. It saves `function.graph` under the UII data directory. Ctrl+S saves it.

## Content drawer

A Content row at the bottom. It starts as `Content  >`. Ctrl+Space toggles it. Pin keeps it open; without a pin, a click outside the drawer closes it. Opening it shows Assets and Components and the page for the one that is selected. Assets take a path, Add, and Place. Components take a name, Save selection, and Insert.

## Import and export

Import and Export are separate windows, not the canvas. Each lists the fifteen formats. HTML and CSS import has two fields. The other formats have one source field. Export shows the generated file.
