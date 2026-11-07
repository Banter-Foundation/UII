# Animation tab

One Dear ImGui surface. The host draws it in a 1440×900 window titled `UII — Animation`. The shell should give this the same content region later.

## Transport

First row: the word Animation, the clip combo, Add clip, Duplicate, Delete clip, the clip name, duration, and Loop.

Second row: Play or Pause, Stop, the playhead time, Undo, and Redo.

## Body

Three columns.

- Layout targets, about 248px. Add target, the target list, and the rest pose (name, X, Y, width, height, rotation, opacity, fill).
- Timeline. A ruler in seconds, then one row per property of the selected target. Other targets show only properties that already have keys. Keys are diamonds. The playhead is an orange line. The wheel zooms around the cursor. The middle button pans.
- Keyframe inspector, about 268px. Time, value, easing, and delete. Easing is Linear, Ease In, Ease Out, Ease In Out, Hold, or Cubic Bezier.

## Motion preview

Under the columns, at least 140px tall. The rest pose is a gray outline. The playhead pose is the filled box, with the target name. The frame fits both, so the box moves instead of the view recentering on it.

Empty states: no clip, no target, and no selected key each say what to add. Nothing is seeded.
