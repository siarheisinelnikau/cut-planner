"""Read a packaged Windows PE without loading or executing it."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct


def inspect(path: Path) -> dict:
    data = path.read_bytes()
    u16 = lambda at: struct.unpack_from("<H", data, at)[0]
    u32 = lambda at: struct.unpack_from("<I", data, at)[0]
    if data[:2] != b"MZ":
        raise ValueError("missing DOS header")
    pe = u32(60)
    if data[pe:pe + 4] != b"PE\0\0" or u16(pe + 4) != 0x8664:
        raise ValueError("expected a Windows x64 executable")
    optional = pe + 24
    if u16(optional) != 0x20B:
        raise ValueError("expected PE32+")
    sections = optional + u16(pe + 20)

    def offset(rva: int) -> int:
        for i in range(u16(pe + 6)):
            at = sections + i * 40
            start = u32(at + 12)
            size = max(u32(at + 8), u32(at + 16))
            if start <= rva < start + size:
                return rva - start + u32(at + 20)
        raise ValueError(f"unmapped PE RVA: {rva}")

    def string(at: int) -> str:
        return data[at:data.index(b"\0", at)].decode("ascii")

    imports = []
    import_rva = u32(optional + 112 + 8)
    if import_rva:
        at = offset(import_rva)
        while any(data[at:at + 20]):
            imports.append(string(offset(u32(at + 12))))
            at += 20
    delayed = u32(optional + 112 + 13 * 8)
    if delayed:
        at = offset(delayed)
        while any(data[at:at + 32]):
            if u32(at) & 1 == 0:
                raise ValueError("unsupported delay import address mode")
            imports.append(string(offset(u32(at + 4))))
            at += 32
    forbidden = ("vcruntime", "msvcp", "d3d", "dxgi", "opengl", "vulkan")
    if any(name.lower().startswith(forbidden) for name in imports):
        raise ValueError(f"unexpected runtime or graphics dependency: {imports}")

    base = offset(u32(optional + 112 + 16))

    def entries(at: int) -> list[tuple[int, int]]:
        return [(u32(at + 16 + i * 8), u32(at + 20 + i * 8))
                for i in range(u16(at + 12) + u16(at + 14))]

    root = dict(entries(base))
    if 14 not in root or 16 not in root:
        raise ValueError("application icon group and version resources are required")
    sizes = []
    for _, directory in entries(base + (root[3] & 0x7FFFFFFF)):
        _, descriptor = entries(base + (directory & 0x7FFFFFFF))[0]
        descriptor += base
        start = offset(u32(descriptor))
        size = u32(descriptor + 4)
        image = data[start:start + size]
        header, width, doubled_height = struct.unpack_from("<III", image)
        if header != 40 or doubled_height != width * 2:
            raise ValueError("invalid embedded icon bitmap")
        pixels = image[40:40 + width * width * 4]
        visible = sum(pixels[i] > 0 for i in range(3, len(pixels), 4))
        if not width * width / 2 < visible < width * width:
            raise ValueError("blank or malformed icon artwork")
        sizes.append(width)
    if sorted(sizes) != [16, 20, 24, 32, 48, 64, 128, 256]:
        raise ValueError("missing native application icon sizes")
    return {"artifact": path.name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
            "machine": "Windows x64", "embedded_icon_sizes": sorted(sizes), "imports": sorted(set(imports)),
            "executed": False}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("artifact", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = json.dumps(inspect(args.artifact), indent=2)
    if args.output:
        args.output.write_text(result + "\n", encoding="utf-8")
    print(result)


if __name__ == "__main__":
    main()
