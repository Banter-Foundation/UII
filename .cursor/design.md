# Design

The window is one surface. The canvas is the large area. Code is not the main screen.

## Top

A tab bar with two tabs: Design, Animation. Nothing else is a top tab. Assets, Components, and Inspect are not tabs.

Under the tabs, a tool row: Select, Frame, Text, Rectangle, Ellipse, Inspect, then Delete, Import, Export, and HTML / CSS.

The active tool is drawn in brackets. Inspect, when it is the active tool, adds one line under the tools. With a selection that line is the type, the size, and the color. With no selection it asks for one.

HTML / CSS opens two editors, HTML and CSS, plus Apply markup and Reload from canvas. It starts closed.

## Body

A layers list on the left, about 200px. The rest is the canvas. Objects are drawn from the document. On the Animation tab they are drawn from the playhead. The selection has a blue outline and eight handles.

The canvas starts empty. There is no sample layout.

## Animation

Under the canvas, only on the Animation tab: Play / Pause, Key, and a time slider. Key stores x, y, width, and height of the selection at the playhead. Dragging on this tab writes keys, not the design rect.

## Content drawer

A Content row at the bottom. It starts as `Content  >`. Opening it shows Assets and Components and the page for the one that is selected. Assets take a path, Add, and Place. Components take a name, Save selection, and Insert.

## Import and export

Import and Export are separate windows, not the canvas. Each lists the fifteen formats. HTML and CSS import has two fields. The other formats have one source field. Export shows the generated file.
