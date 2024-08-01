"""
This module defines the utilities to interface with the `jyafn-server` implemented in this
repository. Use this module to upload and manage jyafn function in the server.
"""

from __future__ import annotations

import jyafn as fn
import os
import yaml
import json
import requests
import inspect
import typing
import types
import string
import random

from typing import Optional, Callable, Any, Iterator
from dataclasses import dataclass


def generate_deploy_token() -> str:
    """
    Generates a deploy token to be used in manifests. The format is irrelevant as long as
    the generated token is not an empty string. This is just one strategy.
    """
    return "".join(random.choices(string.ascii_uppercase + string.digits, k=12))


@dataclass
class Server:
    """The server is a client to a jyafn-server instance."""

    host: str
    """
    The HTTP endpoint to connect to. This should be of the form `http://<domain>`,
    without trailing `/`.
    """
    token: Optional[str] = None
    """
    An optional bearer token to be used as authentication to the server. If the server is
    not protected by authentication, leave this as `None`.
    """

    @staticmethod
    def from_env() -> Server:
        """
        Loads the server from the environment variables `JYAFNCLOUD_HOST` and
        `JYAFNCLOUD_TOKEN`.
        """
        return Server(
            host=os.environ["JYAFNCLOUD_HOST"],
            token=os.environ.get("JYAFNCLOUD_TOKEN"),
        )

    @staticmethod
    def from_file(
        path: str = "~/.jyafn/servers.yaml",
        profile: Optional[str] = None,
    ) -> Server:
        """
        Loads the server from a file containing credentials for servers. The default file
        is `"~/.jyafn/servers.yaml`. A profile is a key inside the YAML corresponding to
        the actual credentials. If no profile is set,the `default` profile will be used.
        """
        if profile is None:
            profile = os.environ.get("JYAFNCLOUD_PROFILE", "default")

        with open(os.path.expanduser(path)) as f:
            servers = yaml.safe_load(f)
            server_def = servers[profile]
            return Server(
                host=server_def["host"],
                token=server_def["token"],
            )

    @staticmethod
    def load() -> Server:
        """
        Default loading: tries to load the server from environment variables. If that
        fails, tries to load the serevr from the default file with the default profile.
        """
        try:
            return Server.from_env()
        except Exception:
            pass

        try:
            return Server.from_file()
        except Exception:
            pass

        raise Exception("all load methods failed")

    def ping(self) -> None:
        """
        Pings the host to test connectivity. If a connection could not be established,
        raises an exception.
        """
        response = requests.get(f"{self.host}/health")
        if response.status_code >= 400:
            raise Exception(response.text)

    def post_manifest(self, manifest: Manifest) -> str:
        """
        Posts a manifest to the server, returning the defined deploy token. This method
        will mutate the supplied manifest instance to generate a deploy token if one has
        not yet been defined.
        """
        manifest._maybe_generate_deploy_token()
        response = requests.post(
            f"{self.host}/manifest",
            headers={
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
            },
            data=manifest.to_json(),
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        return manifest.deploy_token

    def get_manifest(self, path: str) -> Manifest:
        """Gets the manifest associated with the supplied path."""
        response = requests.get(
            f"{self.host}/manifest/{path}",
            headers={
                "Authorization": f"Bearer {self.token}",
            },
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        outcome = response.json()

        return Manifest(
            outcome["path"],
            fn.Layout.from_json(json.dumps(outcome["input_layout"])),
            fn.Layout.from_json(json.dumps(outcome["output_layout"])),
            deploy_token=outcome["deploy_token"],
        )

    def get_manifests(self) -> list[Manifest]:
        """Lists all the manifests currently runing in the sever."""
        response = requests.get(
            f"{self.host}/manifest",
            headers={
                "Authorization": f"Bearer {self.token}",
            },
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        outcome = response.json()

        return [
            Manifest(
                entry["path"],
                fn.Layout.from_json(json.dumps(entry["input_layout"])),
                fn.Layout.from_json(json.dumps(entry["output_layout"])),
                deploy_token=entry["deploy_token"],
            )
            for entry in outcome
        ]

    def delete_manifest(self, path: str, *, deploy_token: str) -> None:
        """Deletes the manifest associated with the supplied path."""
        response = requests.delete(
            f"{self.host}/manifest/{path}",
            headers={
                "Authorization": f"Bearer {self.token}",
                "X-Deploy-Token": deploy_token,
            },
        )

        if response.status_code >= 400:
            raise Exception(response.text)

    def put_version(
        self, path: str, func: fn.Function | fn.Graph, *, deploy_token: str
    ) -> None:
        """
        Puts a new version of a function to a given path. This path must have been
        initialized before with a `post_manifest` call.
        """
        response = requests.put(
            f"{self.host}/version/{path}",
            headers={
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/octet-stream",
                "X-Deploy-Token": deploy_token,
            },
            data=func.dump(),
        )

        if response.status_code >= 400:
            raise Exception(response.text)

    def get_version(self, path: str) -> dict[str, Any]:
        """
        Gets information on the current version of a function associated with the supplied
        path.
        """
        response = requests.get(
            f"{self.host}/version/{path}",
            headers={
                "Authorization": f"Bearer {self.token}",
            },
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        return response.json()

    def get_version_artifact(self, path: str) -> bytes:
        """
        Downloads the currently running jyafn function at the supplied path.
        """
        response = requests.get(
            f"{self.host}/version-artifact/{path}",
            headers={
                "Authorization": f"Bearer {self.token}",
            },
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        return response.content

    def call(self, path: str, **input) -> Any:
        """Calls the function associated with the supplied path, given the arguments."""
        response = requests.post(
            f"{self.host}/fn/{path}",
            headers={
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
            },
            data=json.dumps(input),
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        return response.json()

    def get_logs(self, path: str) -> Logs:
        """
        Gets the logs associated with a given path. These logs are transient and may be
        flushed from time to time.
        """
        response = requests.get(
            f"{self.host}/logs/{path}",
            headers={
                "Authorization": f"Bearer {self.token}",
            },
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        return Logs(response.json() or [])

    def get_usage(self) -> dict[str, Any]:
        """Gets statistics on resource usage on the server."""
        response = requests.get(
            f"{self.host}/usage",
            headers={
                "Authorization": f"Bearer {self.token}",
            },
        )

        if response.status_code >= 400:
            raise Exception(response.text)

        return response.json()


@dataclass
class Logs:
    """
    Logs for deployment events in the server associated with a given path. See
    `Server.get_logs` for more information.
    """

    entries: list[dict[str, str]]
    """The entries associated in this log collection."""

    def __iter__(self) -> Iterator:
        return iter(self.entries)

    def __str__(self) -> str:
        buf = ""
        for entry in self.entries:
            buf += f"{entry['timestamp']}\t{entry['level'].upper():<7}\t{entry['message']}\n"
        return buf

    def show(self) -> None:
        """
        Prints the log to the screen. You can also get a string representation of the logs
        by calling `str(logs)`.
        """
        print(self)


@dataclass
class Manifest:
    """
    A manifest represents a systems-leve contract for functions. It defines a _maximum_
    input layout and a _minimum_ output layout, so that callers can have some guarantees
    on the data they expect to get from calls.
    """

    path: str
    """The path this manifest applies to."""
    input_layout: fn.Layout
    """
    The maximum input layout. Functions can choose to not use fields from this layout, but
    cannot add extra fields.
    """
    output_layout: fn.Layout
    """
    The minimum output layout. Functions can choose to send extra fields, but need to
    supply all fields in this layout.
    """
    deploy_token: Optional[str] = None
    """
    The deploy token is a safety mechanism to prevent unwanted changed to the server. This
    token has to be supplied every time a new function version is sent and every time this
    manifest has to be updated or deleted. 
    """

    def _maybe_generate_deploy_token(self) -> None:
        """Generates a deploy token if this manifest dones't yet have one."""
        if self.deploy_token is None:
            self.deploy_token = generate_deploy_token()

    def with_deploy_token(self, deploy_token: str) -> Manifest:
        """Creates a new manifest instance with the supplied deploy token."""
        return Manifest(self.path, self.input_layout, self.output_layout, deploy_token)

    @staticmethod
    def from_json(s: str) -> Manifest:
        obj: dict[str, Any] = json.loads(s)
        return Manifest(
            path=obj["path"],
            input_layout=fn.Layout.from_json(json.dumps(obj["input_layout"])),
            output_layout=fn.Layout.from_json(json.dumps(obj["output_layout"])),
            deploy_token=obj.get("deploy_token", None),
        )

    def to_json(self) -> str:
        """Generates a JSON string representation of this manifest."""
        input_json = self.input_layout.to_json()
        output_json = self.output_layout.to_json()

        return '{{"path":{},"deploy_token":{},"input_layout":{},"output_layout":{}}}'.format(
            json.dumps(self.path),
            json.dumps(self.deploy_token),
            input_json,
            output_json,
        )

    @staticmethod
    def for_prototype(path: str, prototype: fn.Function | fn.Graph) -> Manifest:
        """
        Generates a manifest for a given jyafn function or graph. This takes the
        prototype's input layout to he the input layout of the manifest and the
        prototype's output layout to be the output layout of the manifest.
        """
        return Manifest(path, prototype.input_layout(), prototype.output_layout())


def _layout_from_annotation(a: Any) -> fn.Layout:
    """Generates a layout from an annotation in an input position."""
    match a:
        case types.GenericAlias():
            return typing.get_origin(a).make_layout(typing.get_args(a))
        case _:
            return a.make_layout(())


def _ret_layout_from_annotation(a: Any) -> fn.Layout:
    """Generates a layout from an annotation in a return postition."""
    match a:
        case inspect._empty:
            raise Exception("Return annotation cannot be empty")
        case types.GenericAlias():
            origin = typing.get_origin(a)
            return origin.make_layout(typing.get_args(a))
        case None:
            return fn.unit.make_layout(())
        case _:
            return a.make_layout(())


def manifest(path: str) -> Callable[[Callable], Manifest]:
    """
    The manifest decorator. Use this decorator on a dummy function to define it as a
    manifest, much the same way as you would do to create a `jyafn.func`. The body of the
    function will be ignored and therefore can be empty. Example:
    ```python
    @jyafn_cloud.manifest(path="foo")
    def manifest(a: fn.scalar, b: fn.scalar) -> fn.scalar:
        pass
    ```
    This will create the desired manifest and assign it to the variable `manifest`.
    """

    def manifest_(prototype: Callable) -> Manifest:
        signature = inspect.signature(prototype)
        input_layout = fn.Layout.struct_of(
            {
                arg: _layout_from_annotation(param.annotation)
                for arg, param in signature.parameters.items()
            }
        )
        output_layout = _ret_layout_from_annotation(signature.return_annotation)

        return Manifest(path, input_layout, output_layout)

    return manifest_
