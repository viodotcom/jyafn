"""
Implements the `jyafn cloud` CLI utility.
"""

import click
import os
import yaml
import json
import subprocess
import sys
import jyafn as fn

from .. import cloud as jyafn_cloud

from typing import Any, Callable
from pygments import highlight, lexers, formatters


def with_servers(func: Callable[[dict[str, Any]], dict[str, Any]]) -> None:
    os.makedirs(os.path.expanduser("~/.jyafn"), exist_ok=True)

    try:
        with open(os.path.expanduser("~/.jyafn/servers.yaml")) as f:
            servers = yaml.safe_load(f)
    except FileNotFoundError:
        servers = {}

    try:
        servers = func(servers)
    except Exception as e:
        click.echo("error: " + str(e), file=sys.stderr)
        exit(1)

    with open(os.path.expanduser("~/.jyafn/servers.yaml"), "w") as f:
        yaml.safe_dump(servers, f)


def load_server(profile: str) -> jyafn_cloud.Server:
    return jyafn_cloud.Server.from_file(profile=profile)


def print_json(obj: Any) -> None:
    if sys.stdout.isatty():
        formatted_json = json.dumps(obj, indent=4)
        colorful_json = highlight(
            formatted_json, lexers.JsonLexer(), formatters.TerminalFormatter()
        )
        click.echo(colorful_json)
    else:
        click.echo(json.dumps(obj))


def show_text_with_less(text: str) -> None:
    if sys.stdout.isatty():
        click.echo(text)
    else:
        process = subprocess.Popen(["less"], stdin=subprocess.PIPE)
        try:
            process.communicate(input=text.encode())
        except Exception as e:
            click.echo(f"An error occurred: {e}")


@click.group(help="Manages connections to jyafn servers")
def cloud():
    pass


@cloud.command(help="Shows current profiles")
@click.option("--full", is_flag=True)
def profile_ls(full: bool):
    def mutate(servers: dict[str, Any]) -> dict[str, Any]:
        if full:
            print_json(servers)
        else:
            print_json(list(servers.keys()))
        return servers

    with_servers(mutate)


@cloud.command(help="Adds a profile")
@click.argument("profile")
@click.argument("host")
@click.option("--token", default=None)
def profile_add(profile: str, host: str, token: str):
    def mutate(servers: dict[str, Any]) -> dict[str, Any]:
        if profile in servers:
            raise Exception(f"profile `{profile}` already exists")
        servers[profile] = {"host": host, "token": token}
        return servers

    with_servers(mutate)


@cloud.command(help="Removes a profile")
@click.argument("profile")
def profile_rm(profile: str):
    def mutate(servers: dict[str, Any]) -> dict[str, Any]:
        if profile not in servers:
            raise Exception(f"profile `{profile}` does not exist")
        del servers[profile]
        return servers

    with_servers(mutate)


@cloud.command(help="Pings the server")
@click.option("--profile", default="default")
def ping(profile: str) -> None:
    server = load_server(profile)
    try:
        server.ping()
        click.echo("ping!")
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Gets a manifest from a server")
@click.option("--profile", default="default")
@click.argument("path")
def manifest(profile: str, path: str) -> None:
    server = load_server(profile)
    try:
        print_json(json.loads(server.get_manifest(path).to_json()))
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Lists all manifests residing in a server")
@click.option("--profile", default="default")
@click.option("--full", is_flag=True)
def manifest_ls(profile: str, full: bool) -> None:
    server = load_server(profile)
    try:
        if full:
            print_json(
                [json.loads(manifest.to_json()) for manifest in server.get_manifests()]
            )
        else:
            print_json([manifest.path for manifest in server.get_manifests()])
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Posts a manifest to a server")
@click.option("--profile", default="default")
@click.argument("path")
def manifest_post(profile: str, path: str) -> None:
    server = load_server(profile)
    try:
        with open(path) as manifest:
            print_json(
                {
                    "deploy_token": server.post_manifest(
                        jyafn_cloud.Manifest.from_json(manifest.read())
                    )
                }
            )
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Gets the current version for a function from a server")
@click.option("--profile", default="default")
@click.argument("path")
def version(profile: str, path: str) -> None:
    server = load_server(profile)
    try:
        print_json(server.get_version(path))
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Downloads the current version of a function from a server")
@click.option("--profile", default="default")
@click.argument("path")
@click.option("--output", "-o", default=None)
def download(profile: str, path: str, output: str | None) -> None:
    server = load_server(profile)
    try:
        if output is None:
            output = path.split("/")[-1] + ".jyafn"

        if output != "-":
            with open(output, "wb") as out:
                out.write(server.get_version_artifact(path))
        else:
            sys.stdout.write(server.get_version_artifact(path))
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Puts a new version for a function into a server")
@click.option("--profile", default="default")
@click.option("--deploy-token")
@click.argument("path")
@click.argument("filename")
def version_put(profile: str, deploy_token: str, path: str, filename: str) -> None:
    server = load_server(profile)
    try:
        print_json(
            server.put_version(path, fn.read_graph(filename), deploy_token=deploy_token)
        )
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Gets the logs for a function from a server")
@click.option("--profile", default="default")
@click.option("--less", default=True)
@click.argument("path")
def logs(profile: str, less: bool, path: str) -> None:
    server = load_server(profile)
    try:
        to_show = str(server.get_logs(path)).strip()
        if less:
            show_text_with_less(to_show)
        else:
            click.echo(to_show)
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)


@cloud.command(help="Gets the resource usage for a given server")
@click.option("--profile", default="default")
def usage(profile: str) -> None:
    server = load_server(profile)
    try:
        print_json(server.get_usage())
    except Exception as e:
        click.echo("error: " + str(e).strip(), file=sys.stderr)
        exit(1)
