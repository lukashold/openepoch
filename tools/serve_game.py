#!/usr/bin/env python3
"""Serve the portable game on loopback using only Python's standard library."""
from __future__ import annotations

import argparse
import errno
import functools
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import socket
import sys
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from urllib.error import URLError
from urllib.parse import urlsplit
from urllib.request import urlopen
import webbrowser

HOST = "127.0.0.1"
STATUS_PATH = "/__openepoch__/status"
SERVER_VERSION = 2
WASM_FILES = ("openepoch.wasm", "openepoch-ee2.wasm", "openepoch-ee2-up15.wasm")


class GameServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = os.name != "nt"

    def server_bind(self):
        if os.name == "nt" and hasattr(socket, "SO_EXCLUSIVEADDRUSE"):
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
        super().server_bind()


class GameHandler(SimpleHTTPRequestHandler):
    extensions_map = {
        **SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript; charset=utf-8",
        ".mjs": "text/javascript; charset=utf-8",
        ".json": "application/json; charset=utf-8",
        ".html": "text/html; charset=utf-8",
        ".css": "text/css; charset=utf-8",
        ".png": "image/png",
        ".ogg": "audio/ogg",
        ".wav": "audio/wav",
        ".mp3": "audio/mpeg",
    }

    def end_headers(self):
        self.send_header("Cache-Control", "no-cache")
        self.send_header("X-Content-Type-Options", "nosniff")
        super().end_headers()

    def send_head(self):
        port = self.server.server_address[1]
        if self.headers.get("Host", "") not in {
            f"127.0.0.1:{port}", f"localhost:{port}"
        }:
            self.send_error(403, "Only local game requests are accepted")
            return None
        if urlsplit(self.path).path == STATUS_PATH:
            data = json.dumps(self.server.game_status).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            return io.BytesIO(data)
        # SimpleHTTPRequestHandler already normalizes URL traversal. Also keep
        # symlinks from exposing anything outside the selected docs directory.
        target = Path(self.translate_path(self.path)).resolve()
        try:
            contained = os.path.normcase(os.path.commonpath([str(target), str(self.server.game_root)])) == os.path.normcase(str(self.server.game_root))
        except ValueError:
            contained = False
        if not contained:
            self.send_error(403, "Outside game directory")
            return None
        return super().send_head()

    def list_directory(self, path):
        self.send_error(404, "No directory listing")
        return None

    def log_message(self, fmt, *args):
        if self.server.verbose or (len(args) > 1 and str(args[1]).isdigit() and int(args[1]) >= 400):
            super().log_message(fmt, *args)


def game_identity(root):
    hashes = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in WASM_FILES}
    return {
        "application": "openepoch",
        "serverVersion": SERVER_VERSION,
        "rootToken": hashlib.sha256(str(root).encode("utf-8")).hexdigest()[:16],
        # Preserve the previous BASE-reference status field for existing tools.
        "wasmSha256": hashes["openepoch-ee2.wasm"],
        "wasmSha256ByFile": hashes,
        "originalAssets": (root / "local-original/models/manifest.json").is_file(),
    }


def same_server(port, identity):
    try:
        with urlopen(f"http://{HOST}:{port}{STATUS_PATH}", timeout=0.35) as response:
            data = json.loads(response.read(8192))
        return all(data.get(key) == identity[key] for key in ("application", "serverVersion", "rootToken", "wasmSha256", "wasmSha256ByFile"))
    except (OSError, URLError, ValueError, AttributeError):
        return False


def bind_server(root, port, strict, verbose):
    identity = game_identity(root)
    candidates = [port] if strict or port == 0 else list(range(port, min(65536, port + 11))) + [0]
    for candidate in candidates:
        try:
            handler = functools.partial(GameHandler, directory=str(root))
            server = GameServer((HOST, candidate), handler)
            server.game_root, server.game_status, server.verbose = root, identity, verbose
            return server, server.server_address[1], False
        except OSError as error:
            if error.errno not in (errno.EADDRINUSE, errno.EACCES) and getattr(error, "winerror", None) not in (10048, 10013):
                raise
            if candidate and same_server(candidate, identity):
                return None, candidate, True
            if strict:
                raise
    raise OSError("No free local game port")


def open_game(url):
    try:
        opened = webbrowser.open(url, new=2)
    except (OSError, webbrowser.Error):
        opened = False
    if not opened:
        print("Browser nicht automatisch geoeffnet. Oeffne diese Adresse: " + url, flush=True)


def main(argv=None):
    if sys.version_info < (3, 8):
        print("OpenEpoch benoetigt Python3.8 oder neuer.", file=sys.stderr)
        return 2
    parser = argparse.ArgumentParser(description="OpenEpoch lokal starten (nur127.0.0.1).")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1] / "docs")
    parser.add_argument("--port", type=int, default=8125)
    parser.add_argument("--strict-port", action="store_true", help="Bei belegtem Port keinen anderen waehlen")
    parser.add_argument("--no-browser", action="store_true")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(argv)
    if not 0 <= args.port <= 65535:
        parser.error("Port muss zwischen0 und65535 liegen")
    root = args.root.expanduser().resolve()
    for name in ("index.html", *WASM_FILES):
        if not (root / name).is_file():
            print("Spielpaket unvollstaendig: " + str(root / name), file=sys.stderr)
            return 2
    try:
        server, port, reused = bind_server(root, args.port, args.strict_port, args.verbose)
    except OSError as error:
        print("Lokaler Spielserver konnte nicht starten: " + str(error), file=sys.stderr)
        return 2
    url = f"http://{HOST}:{port}/?rules=ee2&build=native10"
    print("READY " + json.dumps({"url": url, "host": HOST, "port": port, "reused": reused}), flush=True)
    if port != args.port and args.port:
        print(f"Port{args.port} war belegt. Dieses Mal wird Port{port} verwendet; gespeicherte Spiele im Browser sind an den Port gebunden.", flush=True)
    if reused:
        print("Der Spielserver fuer diesen Ordner laeuft bereits.", flush=True)
        if not args.no_browser:
            open_game(url)
        return 0
    print("Spiel: " + url, flush=True)
    print("Dieses Fenster geoeffnet lassen. Ctrl+C beendet den Server.", flush=True)
    if not server.game_status["originalAssets"]:
        print("Hinweis: Originalmedien fehlen. Das Vollpaket enthaelt docs/local-original.", flush=True)
    timer = None
    if not args.no_browser:
        timer = threading.Timer(0.15, open_game, args=(url,))
        timer.daemon = True
        timer.start()

    def stop(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGINT, stop)
    if hasattr(signal, "SIGBREAK"):
        signal.signal(signal.SIGBREAK, stop)
    try:
        with server:
            try:
                server.serve_forever(poll_interval=0.2)
            except KeyboardInterrupt:
                print("Spielserver beendet.", flush=True)
    finally:
        if timer:
            timer.cancel()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
