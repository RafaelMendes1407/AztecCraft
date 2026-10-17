from __future__ import annotations

import zipfile
from pathlib import Path
from unittest.mock import patch

import pytest
from package import Platform, package
from smoke_check import smoke_check
from test_package import binary_fixture, manifest_fixture


def make_archive(tmp_path: Path, target: Platform) -> Path:
    with patch("package.ad_hoc_sign"):
        return package(
            binary_fixture(tmp_path, target), manifest_fixture(tmp_path), tmp_path / "o", target
        )


@pytest.mark.parametrize("target", ["macos", "linux", "windows"])
def test_packaged_binary_architecture_and_mode_pass(tmp_path: Path, target: Platform) -> None:
    smoke_check(make_archive(tmp_path, target), target, "aarch64", run=False)


def test_wrong_architecture_fails(tmp_path: Path) -> None:
    archive = make_archive(tmp_path, "linux")
    with pytest.raises(ValueError, match="expected x86_64"):
        smoke_check(archive, "linux", "x86_64", run=False)


def test_zip_member_without_executable_bit_fails(tmp_path: Path) -> None:
    archive = make_archive(tmp_path, "macos")
    broken = tmp_path / "broken.zip"
    with zipfile.ZipFile(archive) as source, zipfile.ZipFile(broken, "w") as destination:
        for info in source.infolist():
            if info.filename.endswith("MacOS/azteccraft"):
                info.external_attr = 0o100644 << 16
            destination.writestr(info, source.read(info))
    with pytest.raises(ValueError, match="not marked executable"):
        smoke_check(broken, "macos", "aarch64", run=False)
