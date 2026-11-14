### Overview

<sup>Since: 25.02</sup>

You can switch a column to present windows as tabs, rather than as vertical tiles.
All tabs in a column have the same window size, so this is useful to get more vertical space.

![Terminal with a tab indicator on the left.](https://github.com/user-attachments/assets/0e94ac0d-796d-4f85-a264-c105ef41c13f)

Use this bind to toggle a column between normal and tabbed display:

```kdl
binds {
   Mod+W { toggle-column-tabbed-display; }
}
```

All other binds remain the same: switch tabs with `focus-window-down/up`, add or remove windows with `consume-window-into-column`/`expel-window-from-column`, and so on.

Unlike regular columns, tabbed columns can go full-screen with multiple windows.

### Tab indicator

Tabbed columns show a tab indicator on the side.
You can click on the indicator to switch tabs.

See the [`tab-indicator` section in the layout section](./Configuration:-Layout.md#tab-indicator) to configure it.

By default, the indicator draws "outside" the column, so it can overlay other windows or go off-screen.
The `place-within-column` flag puts the indicator "inside" the column, adjusting the window size to make space for it.
This is especially useful for thicker tab indicators, or when you have very small gaps.

| Default | `place-within-column` |
| --- | --- |
| ![A screenshot showing 4 windows, with the middle column being focused. The tab indicator overflows onto the left column](https://github.com/user-attachments/assets/c2f51f50-3d87-403a-8beb-cbbe5ec5c880) | ![A screenshot showing 4 windows, with the middle column being focused. The tab indicator is contained within its respective column](https://github.com/user-attachments/assets/f1797cd0-d518-4be6-95b4-3540523c4370) |

### Vertical window-title rail

Enable `show-title` to replace the compact indicator with an interactive vertical rail:

```kdl
layout {
    tab-indicator {
        show-title true
    }
}
```

This changes the tab presentation, not which windows share a column. Use
`toggle-column-tabbed-display` or your existing `default-column-display` rules to
select tabbed display.

The rail sits to the left of the window, outside application content. Each tab has
a small marker with a larger hit area; active and urgent tabs remain distinct.
It reuses the column gap and requires a 16-logical-pixel gutter. With smaller gaps,
including zero, only the missing width is reserved. Hovering never resizes the
window or expands the rail.

Hover a marker to see its window title. This readable tooltip can appear over
window content, but it never receives mouse input: moving into the application
clears it and gives input to the application. There is no collapsed title pill,
expanding strip, tab count, or close button.

A single-window column hides its rail during normal use, while keeping the same
space reservation. During a window move it reveals a drop target without resizing
its client. Destination markers and insertion hints remain above the moving
window preview.
While the pointer is over a rail drop target, horizontal edge scrolling pauses so
the target stays under the pointer. Leaving the rail restores normal edge scrolling.

| Mouse action on the rail | Result |
| --- | --- |
| Left-click a tab marker | Activate that window. |
| Scroll over the rail | Switch relative to the column's active tab; stop at either end without wrapping. |
| Middle-click a tab marker | Request that window to close, without first focusing it. |
| Drag a tab vertically along the rail | Reorder it at the indicated insertion boundary. |
| Drag a tab onto another column's rail | Join that column at the indicated position. |
| Drag a tab away from the rails and release | Create a separate column at the indicated location. |

Dragging requires no modifier. Movement below the normal drag threshold remains
a click; releasing a drag commits the displayed destination. An ordinary window
drag can also target a rail. Configured mouse and scroll bindings retain priority.

Title tooltips use a single line with ellipsis, falling back to the application ID
and then `Untitled` when no title is available. Window capture protection also
applies to these titles.

With `show-title false` (the default), compact indicators retain their existing
click-to-activate behavior without the new close, scroll, or modifier-free drag
gestures. See [title tab configuration](./Configuration:-Layout.md#tab-indicator)
for appearance settings.
