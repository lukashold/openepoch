#!/usr/bin/env python3
"""Build a complete, reproducible game ZIP including all imported local media."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import stat
import zipfile


def file_hash(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    project = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=project.parent / "OpenEpoch-Mac-Komplett.zip")
    args = parser.parse_args()
    destination = args.output.resolve()
    destination.parent.mkdir(parents=True, exist_ok=True)
    files = []
    for path in sorted((project / "docs").rglob("*")):
        if path.is_symlink():
            raise ValueError("Symlink is not portable: " + str(path))
        if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc":
            files.append((path, path.relative_to(project).as_posix()))
    for name in ("Start-OpenEpoch.command", "Start-OpenEpoch.cmd", "MAC-START.txt", "LICENSE",
                 "tools/serve_game.py", "tools/portable-assets-test.py", "tools/portable-server-test.py"):
        path = project / name
        if not path.is_file():
            raise ValueError("Required package file missing: " + name)
        files.append((path, name))
    required = {"docs/index.html", "docs/openepoch.wasm", "docs/openepoch-ee2.wasm",
                "docs/openepoch-ee2-up15.wasm", "docs/rules-profile.js",
                "docs/original-hud.js", "docs/original-hud-layout.js",
                "docs/local-original/models/manifest.json", "docs/local-original/audio/manifest.json",
                "docs/local-original/textures.json", "docs/local-original/ground-decals.json",
                "docs/local-original/team-textures/manifest.json",
                "docs/local-original/model-seasons.json", "docs/local-original/model-scales.json",
                "docs/local-original/farm-composites.json", "docs/local-original/hud-native/manifest.json",
                "docs/local-original/water-native/manifest.json",
                "docs/original-diplomacy.js", "docs/original-diplomacy-layout.js",
                "docs/original-labels.js", "docs/original-lighting.js",
                "docs/original-fonts.js", "docs/original-font-ui.js",
                "docs/original-render-namespace.js", "docs/render-namespace.js",
                "docs/native-terrain-lightmap.js", "docs/native-terrain-lighting.js",
                "docs/local-original/fonts-native/bitmap-manifest.json", "docs/local-original/fonts-native/manifest.json",
                "docs/local-original/diplomacy-native/manifest.json",
                "docs/local-original/labels-native.json", "docs/local-original/climate-lighting.json",
                "docs/local-original/models/manifest-up15.json", "docs/local-original/model-scales-up15.json",
                "docs/local-original/season-model-textures/team/manifest.json"}
    names = [name for _, name in files]
    main_source=(project / 'docs/main.js').read_text(encoding='utf8')
    if 'manifest-roster6.json' in main_source:
        required.update({'docs/local-original/models/manifest-roster6.json','docs/local-original/model-scales-roster6.json'})
    if 'manifest-civ8.json' in main_source:
        required.update({'docs/civilization-setup.js','docs/original-pack-states.js',
                         'docs/local-original/models/manifest-civ8.json','docs/local-original/model-scales-civ8.json',
                         'docs/local-original/team-textures/manifest-civ8.json','docs/local-original/labels-native-civ8.json',
                         'docs/local-original/audio/native-families-civ8.json','docs/local-original/pack-animations/manifest.json'})
    if 'configureOriginalDefinitions' in main_source:
        required.update({'docs/original-audio-bindings.js','docs/local-original/audio/native-families.json'})
    if (project/'docs/original-sail-composite.js').is_file():
        required.update({'docs/original-sail-composite.js','docs/local-original/sails-native/manifest.json'})
    if 'loadSourceSailRuntime' in main_source:
        required.update({'docs/render-actor-metadata.js','docs/source-sail-runtime.js',
                         'docs/source-sail-controller.js','docs/source-cloth-kernel.js',
                         'docs/source-cloth-lighting.js','docs/source-cloth-arithmetic.js',
                         'docs/source-cloth-wasm.js','docs/original-dynamic-sails.js',
                         'docs/openepoch-cloth7.wasm'})
    if 'loadCitizenPresentation' in main_source:
        required.update({'docs/openepoch-citizen9.wasm', 'docs/source-citizen-context.js',
                         'docs/original-citizen-presentation.js', 'docs/original-citizen-models.js',
                         'docs/original-animation-payload.js', 'docs/local-original/citizens9/manifest.json',
                         'docs/local-original/citizens9/presentation.json', 'docs/local-original/citizens9/team/manifest.json',
                         'docs/original-regional-models.js', 'docs/local-original/regional9/manifest.json'})
    if 'regional-citycenters/manifest.json' in main_source:
        required.update({'docs/local-original/regional-citycenters/manifest.json',
                         'docs/local-original/regional-citycenters/model-seasons.json',
                         'docs/local-original/regional-citycenters/team/manifest.json'})
    if 'citycenter-terrain/manifest.json' in main_source:
        required.update({'docs/original-citycenter-terrain.js', 'docs/local-original/citycenter-terrain/manifest.json'})
    if 'loadCityCenterIcons' in main_source:
        required.update({'docs/original-citycenter-icons.js', 'docs/local-original/citycenter-icons/manifest.json'})
    if 'regionalMusic:rulesProfile.ruleset===2' in main_source:
        required.update({'docs/original-regional-music.js', 'docs/source-music-controller.js', 'docs/source-menu-music.js',
                         'docs/local-original/music11/manifest.json'})
    if 'loadHouseIcons' in main_source:
        required.update({'docs/local-original/house-icons/manifest.json', 'docs/local-original/regional-houses/manifest.json',
                         'docs/local-original/regional-houses/terrain-fallback.json',
                         'docs/local-original/regional-houses/model-seasons.json', 'docs/local-original/regional-houses/team/manifest.json'})
    if 'houseLodManifestURL' in main_source:
        required.update({'docs/original-house-lod.js', 'docs/local-original/regional-house-lod/manifest.json'})
    if 'loadDockIcons' in main_source:
        required.update({'docs/local-original/dock-icons/manifest.json', 'docs/local-original/regional-docks/manifest.json',
                         'docs/local-original/regional-docks/model-seasons.json', 'docs/local-original/regional-docks/team/manifest.json'})
    if 'mountOriginalShellVideo' in main_source:
        required.update({'docs/original-shell-video.js', 'docs/local-original/shell-video/fe-base.mp4',
                         'docs/local-original/shell-video/fe-up15.mp4', 'docs/local-original/shell-video/manifest.json'})
    if 'regional-e15-seasons/with-docks/manifest.json' in main_source:
        required.update({'docs/local-original/regional-e15-seasons/with-docks/manifest.json',
                         'docs/local-original/regional-e15-seasons/with-docks/model-seasons.json',
                         'docs/local-original/regional-e15-seasons/with-docks/team/manifest.json',
                         'docs/local-original/regional-e15-seasons/terrain-fallback.json'})
    if 'loadBarracksIcons' in main_source:
        required.update({'docs/local-original/barracks-icons/manifest.json', 'docs/local-original/regional-barracks/manifest.json',
                         'docs/local-original/regional-barracks/model-seasons.json', 'docs/local-original/regional-barracks/team/manifest.json',
                         'docs/local-original/regional-barracks/ground-decals.json'})
    if 'mountSkirmishMenu' in main_source:
        required.update({'docs/skirmish-menu.js', 'docs/skirmish-menu.css',
                         'docs/local-original/skirmish-native/manifest.json'})
    if len(names) != len(set(names)) or not required.issubset(names):
        raise ValueError("Incomplete or duplicated game tree")
    print("Hashing complete game files ...", flush=True)
    entries = [{"path": name, "bytes": path.stat().st_size, "sha256": file_hash(path)} for path, name in files]
    media = [entry for entry in entries if entry["path"].startswith("docs/local-original/")]
    manifest = {"format": "openepoch-portable-package-v1", "includesOriginalMedia": True,
                "mediaFiles": len(media), "mediaBytes": sum(e["bytes"] for e in media),
                "totalFileBytes": sum(e["bytes"] for e in entries),
                "start": "Start-OpenEpoch.command", "pythonMinimum": "3.8", "files": entries}
    temporary = destination.with_suffix(destination.suffix + ".tmp")
    print(f"Packing{len(files)} files, including{len(media)} media files ...", flush=True)
    with zipfile.ZipFile(temporary, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for index, (path, name) in enumerate(files):
            info = zipfile.ZipInfo("OpenEpoch/" + name)
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | (0o755 if name.endswith(".command") else 0o644)) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            with path.open("rb") as source, archive.open(info, "w", force_zip64=True) as output:
                for chunk in iter(lambda: source.read(1024 * 1024), b""):
                    output.write(chunk)
            if index and index % 500 == 0:
                print(f"Packed{index}/{len(files)} ...", flush=True)
        for name, data in (("PORTABLE-MANIFEST.json", json.dumps(manifest, ensure_ascii=False, indent=2).encode("utf-8")),
                           ("README.txt", (project / "MAC-START.txt").read_bytes())):
            info = zipfile.ZipInfo("OpenEpoch/" + name)
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
    print("Verifying ZIP CRC and complete inventory ...", flush=True)
    with zipfile.ZipFile(temporary) as archive:
        if archive.testzip() is not None:
            raise ValueError("ZIP verification failed")
        if len(archive.infolist()) != len(files) + 2:
            raise ValueError("ZIP file count mismatch")
        command = archive.getinfo("OpenEpoch/Start-OpenEpoch.command")
        if command.create_system != 3 or (command.external_attr >> 16) & 0o777 != 0o755:
            raise ValueError("macOS launcher executable mode missing")
    temporary.replace(destination)
    print(json.dumps({"output": str(destination), "zipBytes": destination.stat().st_size,
                      "files": len(files) + 2, "originalFiles": len(media),
                      "originalBytes": manifest["mediaBytes"], "sha256": file_hash(destination)}), flush=True)


if __name__ == "__main__":
    main()
