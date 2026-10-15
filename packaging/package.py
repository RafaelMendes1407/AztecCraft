from __future__ import annotations

import argparse
import hashlib
import json
import os
import plistlib
import re
import shutil
import stat
import struct
import tarfile
import tomllib
import zipfile
from pathlib import Path
from typing import Literal

Platform = Literal["macos", "windows", "linux"]
ATOMIC_NOTICES = ("LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md")
NOTICE_LIMIT = 64 * 1024


def capture_atomic_notices(manifest: Path) -> tuple[tuple[str, bytes], ...]:
    root = manifest.parent / "vendor" / "craft-atomic"
    notices = []
    for name in ATOMIC_NOTICES:
        path = root / name
        if not stat.S_ISREG(path.lstat().st_mode):
            raise ValueError(f"atomic notice {name} must be a regular file, not a symlink")
        with path.open("rb") as source:
            if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
                raise ValueError(f"atomic notice {name} must be a regular file")
            content = source.read(NOTICE_LIMIT + 1)
        if len(content) > NOTICE_LIMIT:
            raise ValueError(f"atomic notice {name} exceeds 64 KiB")
        notices.append((name, content))
    return tuple(notices)


def binary_architecture(binary: Path, target: Platform) -> str:
    if binary.is_symlink() or not binary.is_file():
        raise ValueError("binary must be a regular file, not a symlink")
    expected_name = "azteccraft.exe" if target == "windows" else "azteccraft"
    if binary.name != expected_name:
        raise ValueError(f"expected binary named {expected_name}")
    with binary.open("rb") as source:
        header = source.read(64)
        if target == "macos" and header[:4] == b"\xcf\xfa\xed\xfe":
            machine = int.from_bytes(header[4:8], "little")
            architectures = {0x01000007: "x86_64", 0x0100000C: "aarch64"}
        elif target == "linux" and header[:6] == b"\x7fELF\x02\x01":
            machine = int.from_bytes(header[18:20], "little")
            architectures = {62: "x86_64", 183: "aarch64"}
        elif target == "windows" and len(header) == 64 and header[:2] == b"MZ":
            source.seek(struct.unpack_from("<I", header, 60)[0])
            pe_header = source.read(6)
            if pe_header[:4] != b"PE\0\0":
                raise ValueError("binary has no PE signature")
            machine = int.from_bytes(pe_header[4:6], "little")
            architectures = {0x8664: "x86_64", 0xAA64: "aarch64"}
        else:
            raise ValueError("binary format does not match selected platform")
    if machine not in architectures:
        raise ValueError("only native x86_64 and aarch64 binaries are supported")
    if target != "windows" and not os.access(binary, os.X_OK):
        raise ValueError("binary is not executable")
    return architectures[machine]


def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def archive_metadata(info: tarfile.TarInfo) -> tarfile.TarInfo:
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    info.mode = 0o755
    return info


def package(binary: Path, manifest: Path, output: Path, target: Platform) -> Path:
    architecture = binary_architecture(binary, target)
    with manifest.open("rb") as source:
        cargo = tomllib.load(source)
    package_data = cargo["package"]
    version = package_data["version"]
    if package_data["name"] != "azteccraft" or not isinstance(version, str):
        raise ValueError("manifest must describe the AztecCraft package")
    if not re.fullmatch(
        r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
        r"(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?",
        version,
    ):
        raise ValueError("package version must be a semantic version")
    notices = capture_atomic_notices(manifest)
    output.mkdir()
    if target == "macos":
        bundle = output / "AztecCraft.app"
        executable = bundle / "Contents" / "MacOS" / "azteccraft"
        executable.parent.mkdir(parents=True)
        numeric_version = version.split("-", 1)[0].split("+", 1)[0]
        with (bundle / "Contents" / "Info.plist").open("xb") as destination:
            plistlib.dump(
                {
                    "CFBundleInfoDictionaryVersion": "6.0",
                    "CFBundleIdentifier": "local.azteccraft.AztecCraft",
                    "CFBundleName": "AztecCraft",
                    "CFBundleExecutable": "azteccraft",
                    "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": numeric_version,
                    "CFBundleVersion": numeric_version,
                    "NSHighResolutionCapable": True,
                    "LSUIElement": False,
                },
                destination,
            )
    else:
        bundle = output / "azteccraft"
        bundle.mkdir()
        executable = bundle / binary.name
    licenses = (
        bundle / "Contents" / "Resources" / "licenses" / "craft-atomic"
        if target == "macos"
        else bundle / "licenses" / "craft-atomic"
    )
    licenses.mkdir(parents=True)
    for name, content in notices:
        with (licenses / name).open("xb") as destination:
            destination.write(content)
    with binary.open("rb") as source, executable.open("xb") as destination:
        shutil.copyfileobj(source, destination)
    if target != "windows":
        executable.chmod(0o755)
    basename = f"azteccraft-{version}-{target}-{architecture}"
    if target == "linux":
        archive = output / f"{basename}.tar.gz"
        with archive.open("xb") as destination:
            with tarfile.open(fileobj=destination, mode="w:gz") as tar:
                tar.add(bundle, arcname=bundle.name, filter=archive_metadata)
    else:
        archive = output / f"{basename}.zip"
        with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED) as zip_file:
            for path in sorted(bundle.rglob("*")):
                info = zipfile.ZipInfo.from_file(path, str(path.relative_to(output)))
                info.create_system = 3
                info.compress_type = zipfile.ZIP_DEFLATED
                mode = 0o755 if path.is_dir() or path == executable else 0o644
                file_type = stat.S_IFDIR if path.is_dir() else stat.S_IFREG
                info.external_attr = (file_type | mode) << 16
                if path.is_dir():
                    info.external_attr |= 0x10
                    zip_file.writestr(info, b"")
                else:
                    with path.open("rb") as source, zip_file.open(info, "w") as destination:
                        shutil.copyfileobj(source, destination)
    with (output / "manifest.json").open("x", encoding="utf-8") as destination:
        json.dump(
            {
                "package": "azteccraft",
                "version": version,
                "platform": target,
                "architecture": architecture,
                "binary_sha256": sha256(executable),
                "archive": archive.name,
                "archive_sha256": sha256(archive),
            },
            destination,
            indent=2,
        )
        destination.write("\n")
    return archive


def main() -> None:
    parser = argparse.ArgumentParser(description="Package an existing AztecCraft binary")
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=Path("Cargo.toml"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--platform", choices=("macos", "windows", "linux"), required=True)
    args = parser.parse_args()
    archive = package(args.binary, args.manifest, args.output, args.platform)
    print(archive)


if __name__ == "__main__":
    main()
