#!/usr/bin/env python3
"""Records the engine calls of the original program as a silky-trace file.

Runs the game's program under the Win32 debug API (on Windows, or under
Wine with a 32-bit Windows Python), puts a breakpoint on each address the
spec names, and at each hit reads the call's name, arguments and script
position from registers and memory as the spec says. The result is a trace
in the format of docs/trace.md, to compare with `silky-run diff`.

The program is only read: the one write is the breakpoint byte (0xcc),
put back at once after each hit. Nothing is patched, nothing in the game
folder is written.

    python probe.py --exe doukyuusei.exe --cwd <game folder> \\
        --spec spec.json --out original.json --seconds 120 --arm-after 20

The spec (JSON) is kept outside the repository; the addresses it uses come
from the build's address table in docs/. See docs/trace.md for its fields.

Python 3.8 or newer, 32-bit (the programs are 32-bit; a 64-bit debugger
would need the WoW64 context calls). No packages.
"""

import argparse
import ctypes
import ctypes.wintypes as wt
import json
import re
import sys
import time

FORMAT, VERSION = "silky-trace", 1

DEBUG_ONLY_THIS_PROCESS = 0x2
CREATE_PROCESS_DEBUG_EVENT = 3
EXIT_PROCESS_DEBUG_EVENT = 5
EXCEPTION_DEBUG_EVENT = 1
EXCEPTION_BREAKPOINT = 0x80000003
EXCEPTION_SINGLE_STEP = 0x80000004
DBG_CONTINUE = 0x00010002
DBG_EXCEPTION_NOT_HANDLED = 0x80010001
THREAD_ALL_ACCESS = 0x1F03FF

# i386 CONTEXT: flags and the offsets of the registers used here.
CONTEXT_FULL = 0x10007
CONTEXT_SIZE = 716
REGS = {
    "edi": 0x9C, "esi": 0xA0, "ebx": 0xA4, "edx": 0xA8, "ecx": 0xAC,
    "eax": 0xB0, "ebp": 0xB4, "eip": 0xB8, "eflags": 0xC0, "esp": 0xC4,
}

k32 = ctypes.WinDLL("kernel32", use_last_error=True) if sys.platform == "win32" else None


class EXCEPTION_RECORD(ctypes.Structure):
    _fields_ = [
        ("ExceptionCode", wt.DWORD), ("ExceptionFlags", wt.DWORD),
        ("ExceptionRecord", ctypes.c_void_p), ("ExceptionAddress", ctypes.c_void_p),
        ("NumberParameters", wt.DWORD), ("ExceptionInformation", ctypes.c_void_p * 15),
    ]


class EXCEPTION_DEBUG_INFO(ctypes.Structure):
    _fields_ = [("ExceptionRecord", EXCEPTION_RECORD), ("dwFirstChance", wt.DWORD)]


class CREATE_PROCESS_DEBUG_INFO(ctypes.Structure):
    _fields_ = [
        ("hFile", wt.HANDLE), ("hProcess", wt.HANDLE), ("hThread", wt.HANDLE),
        ("lpBaseOfImage", ctypes.c_void_p), ("dwDebugInfoFileOffset", wt.DWORD),
        ("nDebugInfoSize", wt.DWORD), ("lpThreadLocalBase", ctypes.c_void_p),
        ("lpStartAddress", ctypes.c_void_p), ("lpImageName", ctypes.c_void_p),
        ("fUnicode", wt.WORD),
    ]


class DEBUG_EVENT(ctypes.Structure):
    class U(ctypes.Union):
        _fields_ = [
            ("Exception", EXCEPTION_DEBUG_INFO),
            ("CreateProcessInfo", CREATE_PROCESS_DEBUG_INFO),
            ("raw", ctypes.c_byte * 160),
        ]

    _fields_ = [("dwDebugEventCode", wt.DWORD), ("dwProcessId", wt.DWORD),
                ("dwThreadId", wt.DWORD), ("u", U)]


class STARTUPINFO(ctypes.Structure):
    _fields_ = [("cb", wt.DWORD), ("pad", ctypes.c_byte * 64)]


class PROCESS_INFORMATION(ctypes.Structure):
    _fields_ = [("hProcess", wt.HANDLE), ("hThread", wt.HANDLE),
                ("dwProcessId", wt.DWORD), ("dwThreadId", wt.DWORD)]


# ---- reading values -------------------------------------------------------

TOKEN = re.compile(r"\s*(0x[0-9a-fA-F]+|\d+|[a-z]{3}|\[|\]|\+|-)")


def parse_expr(text):
    """`esp+4`, `[esp+8]`, `[[ecx+0x10]+4]`: registers, numbers, `+`/`-`
    and `[...]` for the 32-bit value at an address. Returns a tree."""
    tokens = TOKEN.findall(text)
    if "".join(tokens).replace(" ", "") != text.replace(" ", ""):
        raise ValueError(f"cannot read expression {text!r}")
    pos = 0

    def term():
        nonlocal pos
        t = tokens[pos]
        pos += 1
        if t == "[":
            inner = expr()
            if tokens[pos] != "]":
                raise ValueError(f"missing ] in {text!r}")
            pos += 1
            return ("deref", inner)
        if t in REGS:
            return ("reg", t)
        return ("num", int(t, 0))

    def expr():
        nonlocal pos
        node = term()
        while pos < len(tokens) and tokens[pos] in "+-":
            op = tokens[pos]
            pos += 1
            node = (op, node, term())
        return node

    try:
        tree = expr()
    except IndexError:
        raise ValueError(f"unfinished expression {text!r}") from None
    if pos != len(tokens):
        raise ValueError(f"trailing text in {text!r}")
    return tree


def evaluate(tree, regs, read_u32):
    kind = tree[0]
    if kind == "num":
        return tree[1]
    if kind == "reg":
        return regs[tree[1]]
    if kind == "deref":
        return read_u32(evaluate(tree[1], regs, read_u32))
    left = evaluate(tree[1], regs, read_u32)
    right = evaluate(tree[2], regs, read_u32)
    return (left + right if kind == "+" else left - right) & 0xFFFFFFFF


def signed(v):
    return v - (1 << 32) if v & 0x80000000 else v


# ---- the debugger ---------------------------------------------------------

class Process:
    def __init__(self, handle):
        self.handle = handle

    def read(self, addr, size):
        buf = ctypes.create_string_buffer(size)
        got = ctypes.c_size_t()
        if not k32.ReadProcessMemory(self.handle, ctypes.c_void_p(addr), buf, size, ctypes.byref(got)):
            raise OSError(ctypes.get_last_error(), f"read at {addr:#x}")
        return buf.raw[: got.value]

    def write(self, addr, data):
        got = ctypes.c_size_t()
        if not k32.WriteProcessMemory(self.handle, ctypes.c_void_p(addr), data, len(data), ctypes.byref(got)):
            raise OSError(ctypes.get_last_error(), f"write at {addr:#x}")
        k32.FlushInstructionCache(self.handle, ctypes.c_void_p(addr), len(data))

    def u32(self, addr):
        return int.from_bytes(self.read(addr, 4), "little")

    def cstr(self, addr, encoding, limit=4096):
        out = b""
        while len(out) < limit:
            chunk = self.read(addr + len(out), 64)
            end = chunk.find(b"\0")
            if end >= 0:
                return (out + chunk[:end]).decode(encoding, "replace")
            out += chunk
        return out.decode(encoding, "replace")

    def wstr(self, addr, limit=4096):
        out = b""
        while len(out) < limit:
            chunk = self.read(addr + len(out), 64)
            for i in range(0, len(chunk) - 1, 2):
                if chunk[i:i + 2] == b"\0\0":
                    return (out + chunk[:i]).decode("utf-16-le", "replace")
            out += chunk
        return out.decode("utf-16-le", "replace")


def get_context(thread):
    ctx = ctypes.create_string_buffer(CONTEXT_SIZE)
    ctypes.c_uint32.from_buffer(ctx, 0).value = CONTEXT_FULL
    if not k32.GetThreadContext(thread, ctx):
        raise OSError(ctypes.get_last_error(), "GetThreadContext")
    return ctx


def reg(ctx, name):
    return ctypes.c_uint32.from_buffer(ctx, REGS[name]).value


def set_reg(ctx, name, value):
    ctypes.c_uint32.from_buffer(ctx, REGS[name]).value = value & 0xFFFFFFFF


def msvc_string(proc, addr, encoding):
    """An MSVC `std::string` at `addr`: the text inline in its first 16
    bytes while the capacity (at +0x14) is 15 or less, else pointed to;
    the length at +0x10."""
    size = proc.u32(addr + 0x10)
    cap = proc.u32(addr + 0x14)
    data = addr if cap <= 15 else proc.u32(addr)
    return proc.read(data, min(size, 1 << 16)).decode(encoding, "replace") if size else ""


class Stack:
    """The script's value stack when it is a vector of fixed-size
    elements: `vector` is an expression for the vector object (its first
    two words are the begin and end pointers). Each element holds its kind
    at `kind`, a number at `int` and a string (an MSVC std::string) at
    `str`; `kinds` names the kind values: {"0": "int", "1": "str"}."""

    def __init__(self, spec, encoding):
        self.vector = parse_expr(spec["vector"])
        self.size = int(spec["size"])
        self.kind = int(spec.get("kind", 0))
        self.int = int(spec.get("int", 4))
        self.str = int(spec["str"])
        self.kinds = {int(k): v for k, v in spec["kinds"].items()}
        self.encoding = spec.get("encoding", encoding)
        self.limit = int(spec.get("limit", 64))

    def read(self, proc, regs):
        vec = evaluate(self.vector, regs, proc.u32)
        begin, end = proc.u32(vec), proc.u32(vec + 4)
        n = (end - begin) // self.size
        if n < 0 or n > 100000:
            raise ValueError(f"stack of {n} values")
        values = []
        for i in range(max(0, n - self.limit), n):
            e = begin + i * self.size
            kind = self.kinds.get(proc.u32(e + self.kind))
            if kind == "str":
                values.append(msvc_string(proc, e + self.str, self.encoding))
            elif kind == "int":
                values.append(signed(proc.u32(e + self.int)))
            else:
                values.append(f"?kind {proc.u32(e + self.kind)}")
        return values


class Breakpoint:
    def __init__(self, spec, encoding, stack=None):
        self.at = int(str(spec["at"]), 0)
        # A handler shared by several classes names its call by a value it
        # can read, such as the vtable of `this`:
        # {"read": "[ecx]", "names": {"0x5175d0": "Voice", ...}}.
        self.call_by = None
        if isinstance(spec["call"], dict):
            self.call_by = (
                parse_expr(spec["call"]["read"]),
                {int(k, 0): v for k, v in spec["call"]["names"].items()},
            )
        # Arguments from the script's value stack: everything on it, the
        # last value (pushed last) being the sub-command.
        self.stack = stack if spec.get("stack") else None
        self.call = spec["call"] if isinstance(spec["call"], str) else "?"
        self.sub = parse_expr(spec["sub"]) if spec.get("sub") else None
        self.args = [self.arg(a, encoding) for a in spec.get("args", [])]
        self.by_sub = {int(k): [self.arg(a, encoding) for a in v] for k, v in spec.get("by_sub", {}).items()}
        self.script = self.arg(spec["script"], encoding) if spec.get("script") else None
        self.pc = parse_expr(spec["pc"]) if spec.get("pc") else None
        # The engine can also run calls itself, not from the script: record
        # only the hits where `read` gives one of `is`, such as the return
        # address of the script's own loop.
        self.only = None
        if spec.get("only"):
            self.only = (parse_expr(spec["only"]["read"]), {int(str(v), 0) for v in spec["only"]["is"]})
        self.result = None  # results are not read at the call's entry
        self.original = None
        self.address = None

    @staticmethod
    def arg(a, encoding):
        if isinstance(a, str):
            a = {"read": a}
        return (parse_expr(a["read"]), a.get("type", "int"), a.get("encoding", encoding))

    def record(self, proc, regs):
        def value(arg):
            tree, kind, enc = arg
            v = evaluate(tree, regs, proc.u32)
            if kind == "int":
                return signed(v)
            if kind == "str":
                return proc.cstr(v, enc)
            if kind == "wstr":
                return proc.wstr(v)
            raise ValueError(f"unknown type {kind}")

        if self.only is not None and evaluate(self.only[0], regs, proc.u32) not in self.only[1]:
            return None
        call = {}
        name = self.call
        if self.call_by is not None:
            v = evaluate(self.call_by[0], regs, proc.u32)
            name = self.call_by[1].get(v, f"?{v:#x}")
        args = self.args
        if self.sub is not None:
            sub = signed(evaluate(self.sub, regs, proc.u32))
            name = f"{name}.{sub}"
            args = self.by_sub.get(sub, args)
        if self.script is not None:
            call["script"] = value(self.script)
        if self.pc is not None:
            call["pc"] = evaluate(self.pc, regs, proc.u32)
        if self.stack is not None:
            values = self.stack.read(proc, regs)
            if values and isinstance(values[-1], int):
                name = f"{name}.{values.pop()}"
            call["call"] = name
            if values:
                call["args"] = values
            return call
        call["call"] = name
        values = [value(a) for a in args]
        if values:
            call["args"] = values
        return call


def run(opts, spec):
    encoding = spec.get("encoding", "cp932")
    image_base = int(str(spec.get("image_base", "0x400000")), 0)
    stack = Stack(spec["stack"], encoding) if spec.get("stack") else None
    bps = [Breakpoint(b, encoding, stack) for b in spec["breakpoints"]]
    si = STARTUPINFO(cb=ctypes.sizeof(STARTUPINFO))
    pi = PROCESS_INFORMATION()
    if not k32.CreateProcessW(opts.exe, None, None, None, False, DEBUG_ONLY_THIS_PROCESS,
                              None, opts.cwd, ctypes.byref(si), ctypes.byref(pi)):
        raise OSError(ctypes.get_last_error(), f"cannot start {opts.exe}")
    start = time.monotonic()
    proc = None
    by_address = {}
    stepping = {}  # thread id -> breakpoint to put back after one step
    calls, errors = [], []
    armed = False
    ev = DEBUG_EVENT()
    try:
        while time.monotonic() - start < opts.seconds and len(calls) < opts.max_calls:
            if proc and not armed and time.monotonic() - start >= opts.arm_after:
                base = proc.base
                for b in bps:
                    b.address = b.at - image_base + base
                    b.original = proc.read(b.address, 1)
                    proc.write(b.address, b"\xcc")
                    by_address[b.address] = b
                armed = True
            if not k32.WaitForDebugEvent(ctypes.byref(ev), 100):
                continue
            status = DBG_CONTINUE
            code = ev.dwDebugEventCode
            if code == CREATE_PROCESS_DEBUG_EVENT:
                info = ev.u.CreateProcessInfo
                proc = Process(info.hProcess)
                proc.base = info.lpBaseOfImage or image_base
            elif code == EXIT_PROCESS_DEBUG_EVENT:
                break
            elif code == EXCEPTION_DEBUG_EVENT:
                rec = ev.u.Exception.ExceptionRecord
                addr = rec.ExceptionAddress or 0
                if rec.ExceptionCode == EXCEPTION_BREAKPOINT and addr in by_address:
                    b = by_address[addr]
                    thread = k32.OpenThread(THREAD_ALL_ACCESS, False, ev.dwThreadId)
                    ctx = get_context(thread)
                    regs = {r: reg(ctx, r) for r in REGS}
                    try:
                        call = b.record(proc, regs)
                        if call is not None:
                            if opts.tick is not None:
                                call = {"tick": proc.u32(opts.tick - image_base + proc.base), **call}
                            calls.append(call)
                    except (OSError, ValueError) as e:
                        errors.append(f"{b.call} at {addr:#x}: {e}")
                    # Run the original instruction once, then put the
                    # breakpoint back (single step).
                    proc.write(addr, b.original)
                    set_reg(ctx, "eip", addr)
                    set_reg(ctx, "eflags", reg(ctx, "eflags") | 0x100)
                    k32.SetThreadContext(thread, ctx)
                    k32.CloseHandle(thread)
                    stepping[ev.dwThreadId] = b
                elif rec.ExceptionCode == EXCEPTION_SINGLE_STEP and ev.dwThreadId in stepping:
                    b = stepping.pop(ev.dwThreadId)
                    proc.write(b.address, b"\xcc")
                elif rec.ExceptionCode != EXCEPTION_BREAKPOINT:
                    # The program's own exceptions go to its own handlers.
                    status = DBG_EXCEPTION_NOT_HANDLED
            k32.ContinueDebugEvent(ev.dwProcessId, ev.dwThreadId, status)
    finally:
        if pi.hProcess:
            k32.TerminateProcess(pi.hProcess, 0)
    return calls, errors


def write_trace(path, calls, notes, game=None):
    head = {
        "format": FORMAT,
        "version": VERSION,
        "producer": {"kind": "original", "name": "tools/trace/probe.py", "version": "1"},
    }
    if game:
        head["game"] = game
    lines = ["{"]
    for k, v in head.items():
        lines.append(f"  {json.dumps(k)}: {json.dumps(v, ensure_ascii=False)},")
    if notes:
        lines.append(f'  "notes": {json.dumps(notes, ensure_ascii=False)},')
    body = ",\n".join("    " + json.dumps(c, ensure_ascii=False, separators=(",", ":")) for c in calls)
    lines.append('  "calls": [' + ("\n" + body + "\n  ]" if calls else "]"))
    lines.append("}")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--exe", required=True, help="the game's program")
    p.add_argument("--cwd", help="folder to run it in (default: the program's)")
    p.add_argument("--spec", required=True, help="breakpoints and what to read (JSON)")
    p.add_argument("--out", required=True, help="trace file to write")
    p.add_argument("--seconds", type=float, default=60, help="stop after this long")
    p.add_argument("--max-calls", type=int, default=1_000_000)
    p.add_argument("--arm-after", type=float, default=0,
                   help="seconds to run before the breakpoints go in (skip the start-up)")
    p.add_argument("--tick", type=lambda s: int(s, 0),
                   help="address of a frame counter to write as each call's tick")
    p.add_argument("--game", help="game name for the trace header (as the runtime names it)")
    opts = p.parse_args()
    if k32 is None:
        sys.exit("probe.py needs Windows or Wine (a Windows Python)")
    if ctypes.sizeof(ctypes.c_void_p) != 4:
        sys.exit("probe.py needs a 32-bit Python")
    with open(opts.spec, encoding="utf-8") as f:
        spec = json.load(f)
    calls, errors = run(opts, spec)
    notes = [f"spec {spec.get('name', opts.spec)}", f"arm-after {opts.arm_after}s"]
    notes += [f"unread: {e}" for e in errors[:20]]
    write_trace(opts.out, calls, notes, opts.game)
    print(f"{len(calls)} calls, {len(errors)} unread, in {opts.out}")


if __name__ == "__main__":
    main()
