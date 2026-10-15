from __future__ import annotations

import hashlib
import json
import plistlib
import stat
import struct
import tarfile
import zipfile
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest
from package import (
    ATOMIC_NOTICES,
    FONT_NOTICES,
    NOTICE_LIMIT,
    PROJECT_NOTICES,
    Platform,
    capture_atomic_notices,
    package,
)


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
    notices = directory / "vendor" / "craft-atomic"
    notices.mkdir(parents=True)
    for name in ATOMIC_NOTICES:
        (notices / name).write_bytes(f"fixture notice {name}\n".encode())
    for name in PROJECT_NOTICES:
        (directory / name).write_bytes(f"fixture project notice {name}\n".encode())
    fonts = directory / "licenses" / "egui-default-fonts"
    fonts.mkdir(parents=True)
    for name in FONT_NOTICES:
        (fonts / name).write_bytes(f"fixture font notice {name}\n".encode())
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
    notice_root = (
        "AztecCraft.app/Contents/Resources/licenses/craft-atomic"
        if target == "macos"
        else "azteccraft/licenses/craft-atomic"
    )
    for name in ATOMIC_NOTICES:
        expected = (manifest.parent / "vendor" / "craft-atomic" / name).read_bytes()
        assert (output / notice_root / name).read_bytes() == expected
    project_root = notice_root.rsplit("/", 1)[0]
    for name in FONT_NOTICES:
        expected = (manifest.parent / "licenses" / "egui-default-fonts" / name).read_bytes()
        assert (output / project_root / "egui-default-fonts" / name).read_bytes() == expected
    for name in PROJECT_NOTICES:
        expected = (manifest.parent / name).read_bytes()
        assert (output / project_root / name).read_bytes() == expected
    if target == "linux":
        with tarfile.open(archive) as tar:
            for name in PROJECT_NOTICES:
                notice = tar.extractfile(f"{project_root}/{name}")
                assert notice is not None
                assert notice.read() == (manifest.parent / name).read_bytes()
            for name in ATOMIC_NOTICES:
                notice = tar.extractfile(f"{notice_root}/{name}")
                assert notice is not None
                assert notice.read() == (output / notice_root / name).read_bytes()
            member = tar.getmember("azteccraft/azteccraft")
            payload = tar.extractfile(member)
            assert payload is not None
            assert payload.read() == binary.read_bytes()
            assert member.mode == 0o755
            assert member.uid == member.gid == 0
            assert member.uname == member.gname == ""
    else:
        with zipfile.ZipFile(archive) as zip_file:
            for name in PROJECT_NOTICES:
                assert (
                    zip_file.read(f"{project_root}/{name}") == (manifest.parent / name).read_bytes()
                )
            for name in ATOMIC_NOTICES:
                assert (
                    zip_file.read(f"{notice_root}/{name}")
                    == (output / notice_root / name).read_bytes()
                )
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
                    "AztecCraft.app/Contents/Resources/",
                    "AztecCraft.app/Contents/Resources/licenses/",
                    f"{project_root}/LICENSE-APACHE",
                    f"{project_root}/LICENSE-MIT",
                    f"{project_root}/NOTICE",
                    f"{notice_root}/",
                    f"{notice_root}/ATTRIBUTION.md",
                    f"{notice_root}/LICENSE-APACHE",
                    f"{notice_root}/LICENSE-MIT",
                    f"{notice_root}/NOTICE",
                    f"{project_root}/egui-default-fonts/",
                    *[f"{project_root}/egui-default-fonts/{n}" for n in sorted(FONT_NOTICES)],
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


@pytest.mark.parametrize("name", ATOMIC_NOTICES)
@pytest.mark.parametrize("failure", ["missing", "symlink", "directory", "oversize"])
def test_invalid_notice_fails_before_creating_output(
    tmp_path: Path, name: str, failure: str
) -> None:
    binary = binary_fixture(tmp_path, "linux")
    manifest = manifest_fixture(tmp_path)
    notice = manifest.parent / "vendor" / "craft-atomic" / name
    notice.unlink()
    if failure == "symlink":
        try:
            notice.symlink_to(manifest)
        except OSError:
            pytest.skip("host cannot create symlinks")
    elif failure == "directory":
        notice.mkdir()
    elif failure == "oversize":
        notice.write_bytes(b"x" * (NOTICE_LIMIT + 1))
    output = tmp_path / "artifacts"
    with pytest.raises((ValueError, FileNotFoundError)):
        package(binary, manifest, output, "linux")
    assert not output.exists()


@pytest.mark.parametrize("failure", ["unreadable", "mid-read"])
def test_notice_io_error_fails_before_creating_output(tmp_path: Path, failure: str) -> None:
    binary = binary_fixture(tmp_path, "linux")
    manifest = manifest_fixture(tmp_path)
    notice = manifest.parent / "vendor" / "craft-atomic" / ATOMIC_NOTICES[-1]
    output = tmp_path / "artifacts"
    original_open = Path.open
    with notice.open("rb") as source:
        reader = MagicMock()
        reader.__enter__.return_value.fileno.return_value = source.fileno()
        reader.__enter__.return_value.read.side_effect = OSError("injected read failure")

        def failing_open(path: Path, mode: str = "r") -> object:
            if path == notice:
                if failure == "unreadable":
                    raise PermissionError("injected unreadable notice")
                return reader
            return original_open(path, mode)

        with patch.object(Path, "open", new=failing_open), pytest.raises(OSError):
            package(binary, manifest, output, "linux")
    assert not output.exists()


@pytest.mark.parametrize("target", ["macos", "linux", "windows"])
def test_package_uses_captured_notices_without_reopening(tmp_path: Path, target: Platform) -> None:
    binary = binary_fixture(tmp_path, target)
    project = tmp_path / "selected-project"
    project.mkdir()
    manifest = manifest_fixture(project)
    expected = capture_atomic_notices(manifest)

    def capture_then_change(selected_manifest: Path) -> tuple[tuple[str, bytes], ...]:
        captured = capture_atomic_notices(selected_manifest)
        for name, _ in captured:
            (project / "vendor" / "craft-atomic" / name).write_bytes(b"changed after capture")
        return captured

    output = tmp_path / "artifacts"
    with patch("package.capture_atomic_notices", side_effect=capture_then_change):
        archive = package(binary, manifest, output, target)
    notice_root = (
        "AztecCraft.app/Contents/Resources/licenses/craft-atomic"
        if target == "macos"
        else "azteccraft/licenses/craft-atomic"
    )
    for name, content in expected:
        assert (output / notice_root / name).read_bytes() == content
    if target == "linux":
        with tarfile.open(archive) as tar:
            for name, content in expected:
                notice = tar.extractfile(f"{notice_root}/{name}")
                assert notice is not None
                assert notice.read() == content
    else:
        with zipfile.ZipFile(archive) as zip_file:
            for name, content in expected:
                assert zip_file.read(f"{notice_root}/{name}") == content


def test_all_four_notices_at_size_limit_are_captured(tmp_path: Path) -> None:
    manifest = manifest_fixture(tmp_path)
    for name in ATOMIC_NOTICES:
        (tmp_path / "vendor" / "craft-atomic" / name).write_bytes(b"x" * NOTICE_LIMIT)
    assert capture_atomic_notices(manifest) == tuple(
        (name, b"x" * NOTICE_LIMIT) for name in ATOMIC_NOTICES
    )


@pytest.mark.parametrize("name", PROJECT_NOTICES)
def test_missing_project_notice_fails_before_creating_output(tmp_path: Path, name: str) -> None:
    binary = binary_fixture(tmp_path, "linux")
    manifest = manifest_fixture(tmp_path)
    (manifest.parent / name).unlink()
    output = tmp_path / "artifacts"
    with pytest.raises(FileNotFoundError):
        package(binary, manifest, output, "linux")
    assert not output.exists()


@pytest.mark.parametrize("name", FONT_NOTICES)
def test_missing_font_notice_fails_before_creating_output(tmp_path: Path, name: str) -> None:
    binary = binary_fixture(tmp_path, "linux")
    manifest = manifest_fixture(tmp_path)
    (manifest.parent / "licenses" / "egui-default-fonts" / name).unlink()
    output = tmp_path / "artifacts"
    with pytest.raises(FileNotFoundError):
        package(binary, manifest, output, "linux")
    assert not output.exists()


def test_project_notice_symlink_error_names_the_notice(tmp_path: Path) -> None:
    binary = binary_fixture(tmp_path, "linux")
    manifest = manifest_fixture(tmp_path)
    notice = manifest.parent / "NOTICE"
    notice.unlink()
    try:
        notice.symlink_to(manifest)
    except OSError:
        pytest.skip("host cannot create symlinks")
    with pytest.raises(ValueError, match=r"license notice NOTICE must be a regular file"):
        package(binary, manifest, tmp_path / "artifacts", "linux")
