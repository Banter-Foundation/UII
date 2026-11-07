# Design

The window is one surface. The canvas is the large area on the Design tab. HTML and CSS are not the main screen.

## Top

A tab bar with three tabs: Design, Animation, Function. Animation and Function open empty placeholders. Assets, Components, and Inspect are not tabs.

On Design, the tool row is Select, Frame, Text, Rectangle, Ellipse, Delete, and HTML / CSS. The active tool is drawn in brackets.

HTML / CSS opens two editors, HTML and CSS, plus Apply markup and Reload from canvas. It starts closed.

## Body

On Design, a layers list on the left, about 200px. The rest is the canvas. Objects are drawn from the document. The selection has a blue outline and eight handles. The canvas starts empty.

Animation and Function do not draw a canvas. Their draw functions are empty.

## Hooks

After the tab, the shell calls `inspect::draw` and `content_dock::draw`. Both functions are empty. They do not reserve a panel and they do not draw a tool.
