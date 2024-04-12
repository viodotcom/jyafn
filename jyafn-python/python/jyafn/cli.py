from http.server import BaseHTTPRequestHandler, HTTPServer

import click
from click_default_group import DefaultGroup
import jyafn as fn

from .describe import describe, describe_graph


@click.group(cls=DefaultGroup, default="run", default_if_no_args=True)
def main():
    pass


@main.command(help="Describes a function or a graph")
@click.option(
    "--graph",
    is_flag=True,
    help="print info only for the graph and not for the compiled function",
)
@click.argument("file")
def desc(graph, file):
    if not graph:
        click.echo(describe(file).strip())
    else:
        click.echo(describe_graph(fn.read_graph(file)).strip())


@main.command(help="Runs an input against a given file")
@click.argument("file")
@click.argument("input")
def run(file, input):
    try:
        click.echo(fn.read_fn(file).eval_json(input, pretty=True))
    except Exception as e:
        click.echo(e)
        click.echo(f"hint: try `jyafn desc {file}` for help on this function")


@main.command(
    help="Spawns a simple http.server that will serve a single function. "
    "WARNING: this is for development purposes only!"
)
@click.option("--port", default=8080, help="the address of the server")
@click.option("--address", default="", help="the TCP port of the server")
@click.argument("file")
def serve(port, address, file):
    click.echo(f"Reading and compiling {file}... ", nl=False)
    func = fn.read_fn(file)
    click.echo("done!")

    class Server(BaseHTTPRequestHandler):
        def do_POST(self):
            content_length = int(self.headers["Content-Length"])
            post_data = self.rfile.read(content_length)

            try:
                output = func.eval_json(post_data.decode("utf-8"), pretty=True)
                self.send_response(200)
                self.send_header("Content-type", "application/json")
                self.end_headers()
            except Exception as e:
                output = str(e)
                output += f"\nhint: try `jyafn desc {file}` for help on this function"
                self.send_response(400)
                self.send_header("Content-type", "text/plain")
                self.end_headers()

            self.wfile.write(output.encode("utf-8"))

    httpd = HTTPServer((address, port), Server)
    try:
        click.echo(f"Starting httpd on {address}:{port}")
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        httpd.server_close()
        click.echo("Stopping httpd...")
