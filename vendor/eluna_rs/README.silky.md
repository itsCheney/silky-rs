# Eluna in silky-rs

This is the copy of Eluna (`eluna_rs` 0.1.0, MPL-2.0) that siglus_rs vendors,
with its speed and physics changes (see `README.siglus.md`). silky-rs adds:

- `MEmotePlayer::PlayTimeline` without the parallel flag stops the other
  main timelines but leaves difference timelines (faces) playing. The game
  sets a face before it starts a pose, and the face must stay.
- `#![allow(dead_code, non_upper_case_globals)]` in `lib.rs`.
