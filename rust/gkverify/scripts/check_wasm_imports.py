#!/usr/bin/env python3
"""Fail unless a wasm module imports nothing, and print its size.

A Freenet contract module may import only what the node provides, and a module
that links ghostkey-verify and nothing else should import nothing at all: no
`__wbindgen_*` placeholders, no `getrandom`, no WASI. Reads the import section
directly so CI needs no wasm tooling.

Usage: check_wasm_imports.py MODULE.wasm
"""

import sys


def leb128(data, pos):
    result = shift = 0
    while True:
        byte = data[pos]
        pos += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if not byte & 0x80:
            return result, pos


def name(data, pos):
    length, pos = leb128(data, pos)
    return data[pos : pos + length].decode("utf-8"), pos + length


def imports(data):
    if data[:4] != b"\0asm":
        sys.exit("not a wasm module")
    pos = 8
    found = []
    while pos < len(data):
        section, pos = data[pos], pos + 1
        size, pos = leb128(data, pos)
        if section == 2:  # import section
            count, p = leb128(data, pos)
            for _ in range(count):
                module, p = name(data, p)
                field, p = name(data, p)
                found.append(f"{module}.{field}")
                kind, p = data[p], p + 1
                # Skip the import's type so the next entry parses.
                if kind == 0:  # function: type index
                    _, p = leb128(data, p)
                elif kind == 1:  # table: reftype + limits
                    p += 1
                    flags, p = leb128(data, p)
                    _, p = leb128(data, p)
                    if flags & 1:
                        _, p = leb128(data, p)
                elif kind == 2:  # memory: limits
                    flags, p = leb128(data, p)
                    _, p = leb128(data, p)
                    if flags & 1:
                        _, p = leb128(data, p)
                elif kind == 3:  # global: valtype + mutability
                    p += 2
                else:
                    sys.exit(f"unknown import kind {kind}")
        pos += size
    return found


def main():
    path = sys.argv[1]
    with open(path, "rb") as f:
        data = f.read()
    found = imports(data)
    print(f"{path}: {len(data)} bytes, {len(found)} imports")
    for imp in found:
        print(f"  import {imp}")
    if found:
        sys.exit("the module imports from the host; a Freenet contract linking only this crate must not")


if __name__ == "__main__":
    main()
