# Third-party references

## Included code

- **pl_mpeg** by Dominic Szablewski, MIT license, <https://github.com/phoboslab/pl_mpeg>: `crates/silky-runtime/vendor/pl_mpeg/pl_mpeg.h`, unchanged, with its license text at the top of the file. It decodes the MPEG-1 video and MP2 sound of `.vsd` movies. `shim.c` next to it is this project's own.

## References

No other third-party source code is included in this repository. The format code was written from the following references:

- **GARbro** by morkt, MIT license, <https://github.com/morkt/GARbro>: `ArcFormats/Silky/ArcARC.cs` (archive index), `ArcFormats/Silky/ImageAKB.cs` (AKB images), `ArcFormats/elf/ArcVSD.cs` (VSD container), the LZSS reader in `GameRes`.
- **mesScriptAsseAndDisassembler** by TesterTesterov, GPL-3.0, <https://github.com/TesterTesterov/mesScriptAsseAndDisassembler>: the `.MES` header, the opcode/operand table and the string obfuscation of opcode 0x0A.
- **SilkyArcTool** and **MAPTool** by TesterTesterov, <https://github.com/TesterTesterov/SilkyArcTool>, <https://github.com/TesterTesterov/MAPTool>: archive name obfuscation and the MAP layout, cross-checked against the game files.
