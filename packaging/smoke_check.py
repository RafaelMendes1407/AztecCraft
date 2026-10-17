from __future__ import annotations

import argparse
import stat
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

from package import Platform, binary_architecture

MEMBERS: dict[Platform, str] = {
    "macos": "AztecCraft.app/Contents/MacOS/azteccraft",
    "linux": "azteccraft/azteccraft",
    "windows": "azteccraft/azteccraft.exe",
}


def extract_binary(archive: Path, target: Platform, destination: Path) -> Path:
    member = MEMBERS[target]
    binary = destination / member
    binary.parent.mkdir(parents=True)
    if target == "linux":
        with tarfile.open(archive) as tar:
            info = tar.getmember(member)
            mode = info.mode
            source = tar.extractfile(info)
            if source is None:
                raise ValueError(f"{member} is not a regular file")
            binary.write_bytes(source.read())
    else:
        with zipfile.ZipFile(archive) as zip_file:
            mode = zip_file.getinfo(member).external_attr >> 16
            binary.write_bytes(zip_file.read(member))
    if target != "windows" and not stat.S_IMODE(mode) & 0o111 == 0o111:
        raise ValueError(f"{member} is not marked executable in the archive")
    binary.chmod(0o755)
    return binary


def smoke_check(archive: Path, target: Platform, architecture: str, run: bool) -> None:
    with tempfile.TemporaryDirectory() as directory:
        binary = extract_binary(archive, target, Path(directory))
        found = binary_architecture(binary, target)
        if found != architecture:
            raise ValueError(f"packaged binary is {found}, expected {architecture}")
        if run:
            # An unknown argument makes the CLI print its usage and exit 2 before any window
            # opens, which proves the packaged binary loads and starts on this OS.
            result = subprocess.run(
                [str(binary), "--help"], capture_output=True, text=True, timeout=60, check=False
            )
            if result.returncode != 2 or "Usage: azteccraft" not in result.stderr:
                raise ValueError(
                    f"unexpected exit {result.returncode}: {result.stdout}{result.stderr}"
                )


def main() -> None:
    parser = argparse.ArgumentParser(description="Smoke-check a packaged AztecCraft archive")
    parser.add_argument("--packages", type=Path, required=True, help="directory with one archive")
    parser.add_argument("--platform", choices=sorted(MEMBERS), required=True)
    parser.add_argument("--arch", choices=("x86_64", "aarch64"), required=True)
    parser.add_argument("--run", action="store_true", help="also run the binary's usage exit")
    args = parser.parse_args()
    archives = [*args.packages.glob("*.zip"), *args.packages.glob("*.tar.gz")]
    if len(archives) != 1:
        raise SystemExit(f"expected exactly one archive in {args.packages}, found {len(archives)}")
    smoke_check(archives[0], args.platform, args.arch, args.run)
    print(f"{archives[0].name}: {args.platform} {args.arch} ok")


if __name__ == "__main__":
    main()
