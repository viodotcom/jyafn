import click
import platform
import re
import os
import semver
import tempfile
import shutil
import ctypes
import json

from urllib.request import urlretrieve


def platform_extension() -> str:
    match platform.system():
        case "Linux":
            return "so"
        case "Darwin":
            return "dylib"
        case "Windows":
            return "dll"
        case p:
            click.echo(f"platform {p!r} not supported")
            exit(1)


def alternatives() -> list[str]:
    from_env = os.getenv("JYAFN_PATH", os.path.expanduser("~/.jyafn/extensions"))
    if from_env == "":
        # This pesky edge case!
        from_env = os.path.expanduser("~/.jyafn/extensions")
    return from_env.split(",")


@click.group(help="Manages jyafn extension")
def ext():
    pass


@ext.command(help="Dowloads an extension from an URL")
@click.option(
    "--force",
    "-f",
    is_flag=True,
    help="forces the substition of the extension if it exists",
)
@click.argument("origin")
def get(force, origin):
    extension = platform_extension()
    install_to = alternatives()[0]
    os.makedirs(install_to, exist_ok=True)

    click.echo(f"Downloading {origin}...")
    with tempfile.NamedTemporaryFile() as temp:
        urlretrieve(origin, temp.name)

        # This runs arbitrary code downloaded from the internet. I hope you know what you
        # are doing.
        so = ctypes.cdll.LoadLibrary(temp.name)

        if not hasattr(so, "extension_init"):
            click.echo("error: extension is missing symbol `extension_init`")
            exit(1)
        so.extension_init.restype = ctypes.c_char_p

        manifest = json.loads(so.extension_init())
        name = manifest["metadata"]["name"]
        version = manifest["metadata"]["version"]

        if not re.match(r"[a-z][a-z0-9_]*", name):
            click.echo(f"error: invalid extension name {name!r}.")
            click.echo(
                "hint: extension names should contain only lowercase letters and digits "
                "and should start with a letter"
            )
            exit(1)

        try:
            semver.parse_version_info(version)
        except ValueError:
            click.echo(f"error: version {version!r} not a valid semantic version")
            exit(1)

        click.echo(f"Collected {name}-{version}.{extension}")

        target_path = f"{install_to}/{name}-{version}.{extension}"
        if os.path.exists(target_path) and not force:
            click.echo(f"error: extension {name}-{version} is already installed")
            click.echo("hint: pass -f to force reinstall")
            exit(1)

        shutil.copy(temp.name, target_path)

    click.echo(f"You now have {name}-{version} installed")


@ext.command(help="Lists installed extensions")
def ls() -> None:
    extension = platform_extension()
    found = set()

    for path in alternatives():
        for item in os.listdir(path):
            if os.path.isfile(f"{path}/{item}"):
                try:
                    filename, file_ext = item.rsplit(".", 1)
                except ValueError:
                    continue

                if file_ext != extension:
                    continue

                try:
                    name, version = filename.rsplit("-", 1)
                except ValueError:
                    continue

                if not re.match(r"[a-z][a-z0-9_]*", name):
                    continue

                try:
                    semver.parse_version_info(version)
                except ValueError:
                    continue

                found.add((name, version))

    for name, version in sorted(found):
        print(name, version)


@ext.command(help="Removes installed extensions")
@click.option(
    "--force",
    "-f",
    is_flag=True,
    help="suppresses prompt for permission to delete extension",
)
@click.option(
    "--version",
    "-v",
    help="the semantic version requirements to remove. Defaults to `*` (everything).",
)
@click.argument("name")
def rm(force, version, name):
    extension = platform_extension()
    remove_from = alternatives()[0]
    candidates = []

    if not re.match(r"[a-z][a-z0-9_]*", name):
        click.echo(f"error: invalid extension name {name!r}.")
        click.echo(
            "hint: extension names should contain only lowercase letters and digits "
            "and should start with a letter"
        )
        exit(1)

    for item in os.listdir(remove_from):
        if os.path.isfile(f"{remove_from}/{item}"):
            try:
                filename, file_ext = item.rsplit(".", 1)
            except ValueError:
                continue

            if file_ext != extension:
                continue

            candidate_name, candidate_version = filename.rsplit("-", 1)

            if candidate_name != name:
                continue

            try:
                semver.parse_version_info(candidate_version)
            except ValueError:
                continue

            if version is not None and not semver.match(candidate_version, version):
                continue

            candidates.append(candidate_version)

    if not force:
        match len(candidates):
            case 0:
                click.echo("no extension found for removal")
                return
            case 1:
                print(
                    f"Are you sure you want to remove extension {name!r}?",
                    end=" (y/n) ",
                )
            case _:
                head, tail = candidates[:-1], candidates[-1]
                print(
                    f"Are you sure you want to remove extension {name!r}, versions {head.join(', ')} "
                    f"and {tail}?",
                    end=" (y/n) ",
                )

        match input():
            case "y":
                ...
            case "n":
                click.echo("aborting operation")
                exit(1)
            case invalid:
                click.echo(f"error: invalid option {invalid!r}")
                exit(1)

    # Do the thing!
    for candidate in candidates:
        target_path = f"{remove_from}/{name}-{candidate}.{extension}"
        click.echo(f"removing {target_path}...")
        os.remove(target_path)
