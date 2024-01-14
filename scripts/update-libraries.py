#!/usr/bin/env python3

import argparse
import dataclasses
import itertools
import shutil
import subprocess
import tempfile
import tomllib
from pathlib import Path
from typing import Literal, Self

_PROJECT_DIR = (Path(__file__) / "../..").resolve()


@dataclasses.dataclass(slots=True, kw_only=True)
class CargoPackage:
    @dataclasses.dataclass(slots=True, kw_only=True)
    class BuilderMeta:
        enable: bool = False
        repo: str | None = None
        include_patterns: list[str] = dataclasses.field(default_factory=list)
        exclude_patterns: list[str] = dataclasses.field(default_factory=list)

    cargo_path: Path
    name: str
    version: str
    links: str | None
    builder_meta: BuilderMeta

    @classmethod
    def load(cls, cargo_path: Path) -> Self:
        with cargo_path.open("rb") as fp:
            metadata = tomllib.load(fp)

        package = metadata["package"]
        builder_meta = cls.BuilderMeta()

        try:
            builder_meta_data = package["metadata"]["aws-c-builder"]
        except KeyError:
            pass
        else:
            # if aws-c-builder is present the default value for enable is True!
            builder_meta.enable = bool(builder_meta_data.get("enable", True))
            builder_meta.repo = builder_meta_data.get("repo")
            try:
                builder_meta.include_patterns = builder_meta_data["include_patterns"]
            except KeyError:
                pass
            try:
                builder_meta.exclude_patterns = builder_meta_data["exclude_patterns"]
            except KeyError:
                pass

        return cls(
            cargo_path=cargo_path,
            name=package["name"],
            version=package["version"],
            links=package.get("links"),
            builder_meta=builder_meta,
        )

    def get_repo_url(self) -> str:
        return self.builder_meta.repo or f"https://github.com/awslabs/{self.links}.git"

    def get_repo_tag(self) -> str:
        _, _, repo_tag = self.version.partition("+")
        assert repo_tag, "missing repo tag"
        return repo_tag


_DEFAULT_INCLUDE_PATTERNS: list[str] = [
    "cmake/",
    "CMakeLists.txt",
    "include/",
    "LICENSE",
    "source/",
]


def _apply_package_code(package: CargoPackage, temp_dir: Path) -> None:
    assert package.links

    repo_tag = package.get_repo_tag()
    subprocess.run(
        [
            "git",
            "-c",
            "advice.detachedHead=false",
            "clone",
            "--quiet",
            f"--branch={repo_tag}",
            "--depth=1",
            "--",
            package.get_repo_url(),
            str(temp_dir),
        ],
        check=True,
        cwd=_PROJECT_DIR,
    )

    lib_dir = package.cargo_path.parent / package.links
    if lib_dir.exists():
        shutil.rmtree(lib_dir)

    for pattern in itertools.chain(
        _DEFAULT_INCLUDE_PATTERNS, package.builder_meta.include_patterns
    ):
        for path in temp_dir.glob(pattern):
            rel_path = path.relative_to(temp_dir)
            shutil.move(path, lib_dir / rel_path)

    for pattern in package.builder_meta.exclude_patterns:
        for path in lib_dir.glob(pattern):
            if path.is_dir():
                shutil.rmtree(path)
            else:
                path.unlink()


def _check_package_update(package: CargoPackage) -> None:
    current_tag = package.get_repo_tag()
    tags = _list_version_tags(package.get_repo_url())
    newest_tag = tags[0]

    if current_tag != newest_tag:
        print(f"Package {package.name} can be updated to {newest_tag}")


def _list_version_tags(repo_url: str) -> list[str]:
    proc = subprocess.run(
        ["git", "ls-remote", "--tags", "--refs", "--sort=-v:refname", repo_url],
        stdout=subprocess.PIPE,
        check=True,
        cwd=_PROJECT_DIR,
        encoding="utf-8",
    )
    tags: list[str] = []
    for line in proc.stdout.splitlines():
        _oid, _, ref = line.partition("\t")
        assert ref.startswith("refs/tags/")
        tags.append(ref[len("refs/tags/") :])

    return tags


def _arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(required=True)

    parser_apply = subparsers.add_parser("apply")
    parser_apply.set_defaults(op="apply")

    parser_check = subparsers.add_parser("check")
    parser_check.set_defaults(op="check")

    return parser


def _parse_args() -> argparse.Namespace:
    parser = _arg_parser()
    return parser.parse_args()


def main() -> None:
    ns = _parse_args()
    op: Literal["apply"] | Literal["check"] = ns.op

    for package_path in (_PROJECT_DIR / "bindings").iterdir():
        if not package_path.is_dir():
            continue

        try:
            package = CargoPackage.load(package_path / "Cargo.toml")
            if not package.builder_meta.enable:
                continue

            print(f"package: {package.name}")
            if op == "apply":
                with tempfile.TemporaryDirectory() as temp_dir:
                    _apply_package_code(package, Path(temp_dir))
            else:
                _check_package_update(package)
        except Exception:
            print(f"Error in package {package_path.name}")
            raise


if __name__ == "__main__":
    main()
