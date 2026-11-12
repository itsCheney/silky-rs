# Eluna in silky-rs

[Eluna](https://github.com/xmoezzz/eluna) by xmoezzz (`eluna_rs` 0.1.0,
MPL-2.0) is an E-mote player. This copy starts from the one siglus_rs
vendors, with its speed and physics changes (see `README.siglus.md`).
silky-rs adds:

- `MEmotePlayer::PlayTimeline` without the parallel flag stops the other
  main timelines but leaves difference timelines (faces) playing. The game
  sets a face before it starts a pose, and the face must stay.
- PSB v4: the extra chunks are read (the header encrypts 0x30 bytes, and
  the mesh combinator's `rawMeshList` is an extra resource).
- A motion's own `parameterize` maps its variable onto the timeline; a
  null or negative `parameterize` on a layer is ignored.
- A type-0 key followed by more keys hides the layer until the next one.
- Root layers of a nested scope take their stencil owner from the enclosing
  type-3 layer.
- Icons on mesh-transform layers hang from their `originX`/`originY`.
- `#![allow(dead_code, non_upper_case_globals)]` in `lib.rs`.
