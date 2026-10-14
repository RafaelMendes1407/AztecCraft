from __future__ import annotations

import hashlib
import json
import plistlib
import stat
import struct
import tarfile
import zipfile
from pathlib import Path

import pytest
from package import Platform, package


def binary_fixture(directory: Path, target: Platform) -> Path:
    header = bytearray(128)
    if target == "macos":
        header[:4] = b"\xcf\xfa\xed\xfe"
        struct.pack_into("<I", header, 4, 0x0100000C)
    elif target == "linux":
        header[:6] = b"\x7fELF\x02\x01"
        struct.pack_into("<H", header, 18, 183)
    else:
        header[:2] = b"MZ"
        struct.pack_into("<I", header, 60, 64)
        header[64:68] = b"PE\0\0"
        struct.pack_into("<H", header, 68, 0xAA64)
    binary = directory / ("azteccraft.exe" if target == "windows" else "azteccraft")
    binary.write_bytes(header + b"fixture binary payload")
    binary.chmod(0o755)
    return binary


def manifest_fixture(directory: Path, version: str = "0.1.0") -> Path:
    manifest = directory / "Cargo.toml"
    manifest.write_text(f'[package]\nname = "azteccraft"\nversion = "{version}"\n')
    return manifest


@pytest.mark.parametrize("target", ["macos", "linux", "windows"])
def test_package_preserves_payload_metadata_and_archive_layout(
    tmp_path: Path, target: Platform
) -> None:
    binary = binary_fixture(tmp_path, target)
    manifest = manifest_fixture(tmp_path)
    output = tmp_path / "artifacts"
    archive = package(binary, manifest, output, target)
    receipt = json.loads((output / "manifest.json").read_text())
    assert receipt["binary_sha256"] == hashlib.sha256(binary.read_bytes()).hexdigest()
    assert receipt["archive_sha256"] == hashlib.sha256(archive.read_bytes()).hexdigest()
    assert receipt["architecture"] == "aarch64"
    assert receipt["version"] == "0.1.0"
    assert receipt["platform"] == target
    assert archive.name.startswith(f"azteccraft-0.1.0-{target}-aarch64.")
    if target == "linux":
        with tarfile.open(archive) as tar:
            member = tar.getmember("azteccraft/azteccraft")
            payload = tar.extractfile(member)
            assert payload is not None
            assert payload.read() == binary.read_bytes()
            assert member.mode == 0o755
            assert member.uid == member.gid == 0
            assert member.uname == member.gname == ""
    else:
        with zipfile.ZipFile(archive) as zip_file:
            if target == "windows":
                assert zip_file.read("azteccraft/azteccraft.exe") == binary.read_bytes()
            else:
                name = "AztecCraft.app/Contents/MacOS/azteccraft"
                assert zip_file.read(name) == binary.read_bytes()
                mode = zip_file.getinfo(name).external_attr >> 16
                assert stat.S_IMODE(mode) == 0o755
                info = plistlib.loads(zip_file.read("AztecCraft.app/Contents/Info.plist"))
                assert info["CFBundleExecutable"] == "azteccraft"
                assert info["CFBundleVersion"] == info["CFBundleShortVersionString"] == "0.1.0"
                assert info["CFBundleInfoDictionaryVersion"] == "6.0"
                assert info["CFBundlePackageType"] == "APPL"
                assert info["NSHighResolutionCapable"] is True
                assert info["LSUIElement"] is False
                assert zip_file.namelist() == [
                    "AztecCraft.app/Contents/",
                    "AztecCraft.app/Contents/Info.plist",
                    "AztecCraft.app/Contents/MacOS/",
                    name,
                ]


def test_existing_output_and_symlink_are_never_overwritten(tmp_path: Path) -> None:
    binary = binary_fixture(tmp_path, "linux")
    manifest = manifest_fixture(tmp_path)
    output = tmp_path / "artifacts"
    output.mkdir()
    sentinel = output / "keep"
    sentinel.write_bytes(b"original")
    with pytest.raises(FileExistsError):
        package(binary, manifest, output, "linux")
    assert sentinel.read_bytes() == b"original"
    assert list(output.iterdir()) == [sentinel]
    link = tmp_path / "link"
    try:
        link.symlink_to(output, target_is_directory=True)
    except OSError:
        pytest.skip("host cannot create symlinks")
    with pytest.raises(FileExistsError):
        package(binary, manifest, link, "linux")
    assert sentinel.read_bytes() == b"original"


@pytest.mark.parametrize("version", ["../bad", "1.2", "01.2.3"])
def test_invalid_version_fails_before_creating_output(tmp_path: Path, version: str) -> None:
    binary = binary_fixture(tmp_path, "macos")
    manifest = manifest_fixture(tmp_path, version)
    output = tmp_path / "artifacts"
    with pytest.raises(ValueError, match="semantic version"):
        package(binary, manifest, output, "macos")
    assert not output.exists()


def test_wrong_binary_platform_is_rejected_without_output(tmp_path: Path) -> None:
    binary = binary_fixture(tmp_path, "linux")
    manifest = manifest_fixture(tmp_path)
    output = tmp_path / "artifacts"
    with pytest.raises(ValueError, match="format"):
        package(binary, manifest, output, "macos")
    assert not output.exists()
