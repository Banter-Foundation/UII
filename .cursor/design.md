# Function tab

One OS window, 1440×900, titled UII. The Dear ImGui window fills the viewport. The only top-level tab is Function.

## Panes

Toolbar: New trigger, New action, New value, Save.

- Where, on the left: 240 px, or 28% of the row, and at least 140. Groups use the place string. An empty place is the group "No place".
- Canvas, in the middle.
- How, on the right: 280 px, or 32% of the row, and at least 160.

The path of the graph file sits under the panes, then a save notice or the last connection error.

## Canvas

Background `rgb(0.10, 0.11, 0.13)`. The GL clear behind the window is `rgb(0.08, 0.08, 0.10)`. Grid lines are white at 7% alpha, every 24 world units.

A node is 200 px wide, corner radius 4, fill `rgb(0.18, 0.19, 0.22)`. A 4 px stripe on the top edge marks the kind:

| Kind | Stripe |
| --- | --- |
| Trigger | `rgb(0.36, 0.58, 0.86)` |
| Action | `rgb(0.86, 0.55, 0.28)` |
| Value | `rgb(0.42, 0.70, 0.48)` |

The selected node has a 2 px outline `rgb(0.95, 0.84, 0.42)`. The same gold is the in-progress wire. Settled wires are `rgb(0.74, 0.76, 0.82)`, a cubic with an 80 px horizontal bend on screen. Port dots are `rgb(0.93, 0.93, 0.95)`, radius 5, and at least 3 px on screen. Title and port labels are `rgb(0.94, 0.94, 0.96)`. An empty name is drawn as Untitled. A title longer than 24 characters is cut and ends with an ellipsis.

Empty canvas copy: "Add a function to say how and where something works, then connect it."

## Geometry

Header 28. Each port row 22. Bottom pad 8. Height is the header, plus the taller of the input column and the output column (at least one row), plus the pad. An input is centered on the left edge. An output is centered on the right edge. The hit circle is the dot radius plus 2 px.

New functions from the toolbar land at canvas `(48, 48)` and then step by `(36, 96)` so a two-port node does not cover the previous one. Double-click on empty canvas adds a trigger at that point.
