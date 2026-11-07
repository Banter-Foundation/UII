# Inspect

Dear ImGui docking branch, default dark style. One OS window titled UII. The clear color is the same dark gray Banter uses (`0.08, 0.08, 0.10`).

The Inspect surface fills that window. It is not a tab bar and it has no title bar of its own. The tool control is a selectable labeled Inspect, sized to the label, and it is the active tool.

When an item is selected, the window shows three rows:

- Size — width × height
- Color — swatch, then `#RRGGBB`, or `#RRGGBBAA` when alpha is below 255
- Type — the item's type name

When nothing is selected, the window shows "Nothing selected" and does not invent size, color, or type.
