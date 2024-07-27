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
        print("error:", str(e))
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
        print(colorful_json)
    else:
        print(json.dumps(obj))


def show_text_with_less(text: str) -> None:
    if sys.stdout.isatty():
        print(text)
    else:
        process = subprocess.Popen(["less"], stdin=subprocess.PIPE)
        try:
            process.communicate(input=text.encode())
        except Exception as e:
            print(f"An error occurred: {e}")


@click.group(help="Manages connections to jyafn servers")
def cloud():
    pass


@cloud.command(help="Shows current profiles")
def profile_ls():
    def mutate(servers: dict[str, Any]) -> dict[str, Any]:
        print_json(servers)
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


@cloud.command(help="Gets a manifest from a server")
@click.option("--profile", default="default")
@click.argument("path")
def manifest(profile: str, path: str) -> None:
    server = load_server(profile)
    try:
        print_json(json.loads(server.get_manifest(path).to_json()))
    except Exception as e:
        print("error:", str(e).strip())
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
        print("error:", str(e).strip())
        exit(1)


@cloud.command(help="Gets the current version for a function from a server")
@click.option("--profile", default="default")
@click.argument("path")
def version(profile: str, path: str) -> None:
    server = load_server(profile)
    try:
        print_json(server.get_version(path))
    except Exception as e:
        print("error:", str(e).strip())
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
        print("error:", str(e).strip())
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
            print(to_show)
    except Exception as e:
        print("error:", str(e).strip())
        exit(1)


@cloud.command(help="Gets the resource usage for a given server")
@click.option("--profile", default="default")
def usage(profile: str) -> None:
    server = load_server(profile)
    try:
        print_json(server.get_usage())
    except Exception as e:
        print("error:", str(e).strip())
        exit(1)
