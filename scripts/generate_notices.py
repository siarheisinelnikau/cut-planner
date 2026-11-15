"""Regenerate Windows dependency attributions from the locked local Cargo sources."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    result = subprocess.run(
        ["cargo", "metadata", "--offline", "--locked", "--format-version", "1",
         "--filter-platform", "x86_64-pc-windows-msvc"],
        cwd=ROOT, check=True, capture_output=True, encoding="utf-8",
    )
    metadata = json.loads(result.stdout)
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = metadata["resolve"]["root"]
    reachable: set[str] = set()
    pending = [root]
    while pending:
        current = pending.pop()
        for dep in nodes[current]["deps"]:
            if not any(kind["kind"] is None for kind in dep["dep_kinds"]):
                continue
            if dep["pkg"] not in reachable:
                reachable.add(dep["pkg"])
                pending.append(dep["pkg"])

    selected = sorted((packages[key] for key in reachable), key=lambda p: (p["name"], p["version"]))
    texts: dict[str, tuple[str, str]] = {}
    notices: list[tuple[dict, list[str]]] = []
    without_files: list[str] = []
    for package in selected:
        directory = Path(package["manifest_path"]).parent
        candidates = [p for p in directory.iterdir() if p.is_file() and
                      any(word in p.name.upper() for word in ("LICENSE", "LICENCE", "NOTICE", "COPYING", "COPYRIGHT"))]
        if package["license_file"]:
            candidates.append(directory / package["license_file"])
        fonts = directory / "fonts"
        if fonts.is_dir():
            candidates.extend(p for p in fonts.iterdir() if p.suffix.lower() == ".txt")
        references = []
        for path in sorted(set(candidates)):
            if not path.is_file():
                continue
            content = path.read_text(encoding="utf-8", errors="replace").strip()
            digest = hashlib.sha256(content.encode()).hexdigest()[:16]
            texts.setdefault(digest, (f"{package['name']} / {path.name}", content))
            references.append(digest)
        if not references:
            without_files.append(f"{package['name']} {package['version']}")
        notices.append((package, references))

    # Some published workspace crates omit their repository-level license files.
    # Supply the declared SPDX text once, retaining all packaged notices separately.
    apache = next((key for key, (_, value) in texts.items() if "Apache License" in value and "Version 2.0" in value), None)
    mit = next((key for key, (_, value) in texts.items() if "Permission is hereby granted, free of charge" in value), None)
    boost = next((key for key, (_, value) in texts.items() if "Boost Software License - Version 1.0" in value), None)
    lines = ["# Third-party notices", "",
             "Generated from Cargo.lock for the Windows x64 runtime dependency graph. "
             "Dependencies retain their own licenses; the Cut Planner MIT license does not relicense them. "
             "Bundled font notices are reproduced as supplied.", "",
             "For dual MIT / Apache-2.0 dependencies, Apache-2.0 may be used. "
             "The complete license texts and packaged copyright notices follow the attribution list.", ""]
    for package, references in notices:
        lines += [f"## {package['name']} {package['version']}", "",
                  f"Declared license: {package['license'] or 'SEE PACKAGE LICENSE'}.  ",
                  "Authors: " + "; ".join(package["authors"]) + ".  "]
        if package["repository"]:
            lines.append(f"Source: {package['repository']}")
        if not references:
            if "Apache-2.0" in (package["license"] or "") and apache:
                references = [apache]
            elif "MIT" in (package["license"] or "") and mit:
                references = [mit]
            elif package["license"] == "BSL-1.0" and boost:
                references = [boost]
            if not references:
                raise RuntimeError(f"No license text available for {package['name']}")
            lines.append("No separate license file was included in the published crate; declared SPDX terms apply.")
        lines += ["License/notice text: " + ", ".join(f"[{key}](#notice-{key})" for key in references), ""]
    lines += ["# License and notice texts", ""]
    for key, (name, value) in texts.items():
        lines += [f'<a id="notice-{key}"></a>', f"## {name}", "", "```text", value, "```", ""]
    (ROOT / "THIRD_PARTY_NOTICES.md").write_text("\n".join(lines), encoding="utf-8")
    summary = {"runtime_packages": len(notices), "distinct_notice_texts": len(texts), "crates_without_packaged_license_files": without_files}
    (ROOT / "target" / "notice-inventory.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
