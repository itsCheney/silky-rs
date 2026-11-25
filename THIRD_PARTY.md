# Third-party references

## Included code

- **pl_mpeg** by Dominic Szablewski, MIT license, <https://github.com/phoboslab/pl_mpeg>: `crates/silky-runtime/vendor/pl_mpeg/pl_mpeg.h`, unchanged, with its license text at the top of the file. It decodes the MPEG-1 video and MP2 sound of `.vsd` movies. `shim.c` next to it is this project's own.
- **FFmpeg** (LGPL 2.1 or later, <https://ffmpeg.org>), only with the optional `ffmpeg` feature of `silky-runtime`: not part of this repository. `tools/ffmpeg/build.sh` fetches release n7.1 and builds libavformat, libavcodec and libavutil statically without GPL or non-free parts, for the ASF movies (VC-1, WMA Pro) of 先輩が私の妄想にドージンする. `crates/silky-runtime/ffmpeg/shim.c` is this project's own.

- **Eluna** by xmoezzz, MPL-2.0, <https://github.com/xmoezzz/eluna>: `vendor/eluna_rs`, the E-mote player (`eluna_rs` 0.1.0) as vendored by siglus_rs, with this project's changes listed in `vendor/eluna_rs/README.silky.md`. It plays the E-mote models of きまぐれテンプテーション2.
- **siglus_rs** by xmoezzz, MPL-2.0, <https://github.com/xmoezzz/siglus_rs>: `crates/silky-runtime/src/emote/draws.rs` is adapted from its E-mote render plan.

## References

No other third-party source code is included in this repository. The format code was written from the following references:

- **GARbro** by morkt, MIT license, <https://github.com/morkt/GARbro>: `ArcFormats/Silky/ArcARC.cs` (archive index), `ArcFormats/Silky/ImageAKB.cs` (AKB images), `ArcFormats/elf/ArcVSD.cs` (VSD container), the LZSS reader in `GameRes`.
- **mesScriptAsseAndDisassembler** by TesterTesterov, GPL-3.0, <https://github.com/TesterTesterov/mesScriptAsseAndDisassembler>: the `.MES` header, the opcode/operand table and the string obfuscation of opcode 0x0A.
- **SilkyArcTool** and **MAPTool** by TesterTesterov, <https://github.com/TesterTesterov/SilkyArcTool>, <https://github.com/TesterTesterov/MAPTool>: archive name obfuscation and the MAP layout, cross-checked against the game files.
