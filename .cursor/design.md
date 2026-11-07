# Assets and Components drawer

The window is the Banter Dear ImGui stack (dark tokens from Banter `theme.rs`). The stage uses `0x313338`. The drawer body uses `0x232428`. The handle uses `0x1A1B1E`.

```
+--------------------------------------+
| stage dockspace                      |
|                                      |
+--------------------------------------+
| -  Assets    Components      [Pin]   |  32 px handle, always visible
+-------------------+------------------+
| Assets            | Components       |  open body, default 280 px
| tree | contents   | list             |
+-------------------+------------------+
```

Collapsed, only the handle remains and the stage reaches it. Pinned open, the stage shrinks by the body height. Unpinned open, the body overlays the stage. The two panes sit side by side in that body. The asset pane is a folder tree beside the selected folder's contents. There is no top tab for Assets or Components.

The top 6 px of the open body resizes it. Stored height stays between 160 and 1200 px, and the laid-out height also stays within 75% of the window. The Assets pane stays between 25% and 75% of the body width.
