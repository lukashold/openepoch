#!/usr/bin/env python3
"""Serve the portable game on loopback with optional direct LAN signaling."""
from __future__ import annotations

import argparse
import errno
import functools
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import socket
import sys
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from urllib.error import URLError
from urllib.parse import quote, unquote, urlsplit, urlunsplit
from urllib.request import urlopen
import webbrowser

from lan_signaling import LAN_PATH, LAN_PORT, LanServer, SignalingError, Tickets, local_request, private_ipv4
from lan_directory import DIRECTORY_PORT, RoomDirectory

HOST = "127.0.0.1"
STATUS_PATH = "/__openepoch__/status"
SERVER_VERSION = 5
WASM_FILES = ("openepoch.wasm", "openepoch-ee2.wasm", "openepoch-ee2-up15.wasm")
PUBLIC_START_PAGE = "/?rules=ee2&build=native56"
PRIVATE_START_PAGE = "/death-valley2-private.html?rules=ee2"
# The local entry needs its own engine, authored map and presentation media.
# Source-only/public packages retain the normal map/mode selection entry.
PRIVATE_ENTRY_FILES = (
    "death-valley2-private.html",
    "main-death-valley2-private.js",
    "openepoch-ee2-death-valley2.wasm",
    "authored-rectangular-game.js",
    "authored-rectangular-view.js",
    "authored-cliff-terrain.js",
    "authored-cliff-lightmap.js",
    "authored-rectangular-layout.js",
    "dv2-original-renderer.js",
    "dv2-citizen-work-presentation.mjs",
    "local-original/models/manifest.json",
    "local-original/models/manifest-civ8.json",
    "local-original/death-valley2-terrain/manifest.json",
    "local-original/death-valley2-terrain/world.w8in",
    "local-original/death-valley2-terrain/world0.dvp1",
    "local-original/death-valley2-terrain/animal-manager.anm",
    "local-original/death-valley2-terrain/material-2.bin",
    "local-original/death-valley2-terrain/lightmaps.r8",
    "local-original/citizen-work-events/manifest.json",
    "local-original/citizen-work-events/source-metadata.json",
)


def default_start_page(root):
    """Prefer the current local game only when its required files are present."""
    root = root.resolve()
    try:
        for name in PRIVATE_ENTRY_FILES:
            target = (root / name).resolve()
            target.relative_to(root)
            if not target.is_file() or not target.stat().st_size:
                return PUBLIC_START_PAGE
    except (OSError, ValueError):
        return PUBLIC_START_PAGE
    return PRIVATE_START_PAGE


def start_page(root, value=None):
    """Resolve an optional existing HTML entry beneath the same game root."""
    if value is None:
        return default_start_page(root)
    invalid = "--page braucht eine relative HTML-Datei im Spielordner"
    if not value or value != value.strip() or "\\" in value or any(ord(c) < 32 or ord(c) == 127 for c in value):
        raise ValueError(invalid)
    try:
        parts = urlsplit(value)
        if parts.scheme or parts.netloc or not parts.path or parts.path.startswith("/"):
            raise ValueError(invalid)
        path = unquote(parts.path, encoding="utf-8", errors="strict")
        if ":" in path or "\\" in path or "%" in path or any(part in ("", ".", "..") for part in path.split("/")):
            raise ValueError(invalid)
        target = (root / path).resolve()
        target.relative_to(root.resolve())
        if target.suffix.lower() != ".html" or not target.is_file():
            raise ValueError(invalid)
    except (OSError, UnicodeError, ValueError):
        raise ValueError(invalid) from None
    return "/" + urlunsplit(("", "", quote(path, safe="/"), parts.query, parts.fragment))


class GameServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = os.name != "nt"
    request_queue_size = 64

    def server_bind(self):
        if os.name == "nt" and hasattr(socket, "SO_EXCLUSIVEADDRUSE"):
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
        super().server_bind()


class GameHandler(SimpleHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    timeout = 20

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
        ".mp4": "video/mp4",
    }

    def end_headers(self):
        self.send_header("Cache-Control", "no-store" if self.path == LAN_PATH else "no-cache")
        self.send_header("X-Content-Type-Options", "nosniff")
        if self.command == "POST" and self.path == LAN_PATH:
            self.send_header("Connection", "close")
        if getattr(self, "video_response", False):
            self.send_header("Accept-Ranges", "bytes")
        super().end_headers()

    def do_POST(self):
        # Rejected signaling requests can still have an unread body.
        self.close_connection = True
        if self.path == LAN_PATH:
            local_request(self)
        else:
            self.send_error(404)

    def send_head(self):
        self.video_response = False
        self.range_remaining = None
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
        if target.suffix.lower() == ".mp4" and target.is_file():
            self.video_response = True
            interval = re.fullmatch(r"bytes=([0-9]{0,20})-([0-9]{0,20})", self.headers.get("Range", ""))
            if interval and any(interval.groups()):
                return self.video_range(target, *interval.groups())
        return super().send_head()

    def video_range(self, target, first, last):
        try:
            source = target.open("rb")
        except OSError:
            self.send_error(404, "Video unavailable")
            return None
        try:
            stat = os.fstat(source.fileno())
            length = stat.st_size
            modified = self.date_time_string(stat.st_mtime)
            if self.headers.get("If-Range", modified) != modified:
                source.close()
                return super().send_head()
            start = int(first) if first else max(0, length - int(last))
            end = min(int(last), length - 1) if first and last else length - 1
            if start > end:
                source.close()
                self.send_response(416)
                self.send_header("Content-Range", f"bytes */{length}")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return None
            source.seek(start)
            self.range_remaining = end - start + 1
            self.send_response(206)
            self.send_header("Content-Type", "video/mp4")
            self.send_header("Content-Range", f"bytes {start}-{end}/{length}")
            self.send_header("Content-Length", str(self.range_remaining))
            self.send_header("Last-Modified", modified)
            self.end_headers()
            return source
        except BaseException:
            source.close()
            raise

    def copyfile(self, source, outputfile):
        if self.range_remaining is None:
            return super().copyfile(source, outputfile)
        remaining = self.range_remaining
        while remaining:
            block = source.read(min(remaining, 64 * 1024))
            if not block:
                break
            outputfile.write(block)
            remaining -= len(block)

    def list_directory(self, path):
        self.send_error(404, "No directory listing")
        return None

    def log_message(self, fmt, *args):
        if self.server.verbose or (len(args) > 1 and str(args[1]).isdigit() and int(args[1]) >= 400):
            super().log_message(fmt, *args)


def game_identity(root, lan_address=None):
    hashes = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in WASM_FILES}
    return {
        "application": "openepoch",
        "serverVersion": SERVER_VERSION,
        "rootToken": hashlib.sha256(str(root).encode("utf-8")).hexdigest()[:16],
        # Preserve the previous BASE-reference status field for existing tools.
        "wasmSha256": hashes["openepoch-ee2.wasm"],
        "wasmSha256ByFile": hashes,
        "originalAssets": (root / "local-original/models/manifest.json").is_file(),
        "lanAddress": lan_address,
    }


def same_server(port, identity):
    try:
        with urlopen(f"http://{HOST}:{port}{STATUS_PATH}", timeout=0.35) as response:
            data = json.loads(response.read(8192))
        return all(data.get(key) == identity[key] for key in ("application", "serverVersion", "rootToken", "wasmSha256", "wasmSha256ByFile", "lanAddress"))
    except (OSError, URLError, ValueError, AttributeError):
        return False


def bind_server(root, port, strict, verbose, lan_address=None):
    identity = game_identity(root, lan_address)
    candidates = [port] if strict or port == 0 else list(range(port, min(65536, port + 11))) + [0]
    for candidate in candidates:
        try:
            handler = functools.partial(GameHandler, directory=str(root))
            server = GameServer((HOST, candidate), handler)
            server.game_root, server.game_status, server.verbose = root, identity, verbose
            server.lan_address, server.lan_tickets = None, None
            server.lan_directory = RoomDirectory(None, None)
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
    parser = argparse.ArgumentParser(description="OpenEpoch lokal starten (127.0.0.1).")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1] / "docs")
    parser.add_argument("--port", type=int, default=8125)
    parser.add_argument("--strict-port", action="store_true", help="Bei belegtem Port keinen anderen waehlen")
    parser.add_argument("--no-browser", action="store_true")
    parser.add_argument("--page", metavar="RELATIVE-HTML", help="Bestimmte HTML-Seite im Spielordner oeffnen (z.B. index.html?rules=ee2)")
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--lan", metavar="PRIVATE-IPV4", help="LAN-Einladungen auf dieser privaten IPv4-Adresse zulassen")
    args = parser.parse_args(argv)
    if not 0 <= args.port <= 65535:
        parser.error("Port muss zwischen0 und65535 liegen")
    if args.lan:
        try:
            private_ipv4(args.lan)
        except SignalingError:
            parser.error("--lan braucht die private IPv4-Adresse dieses Rechners")
    root = args.root.expanduser().resolve()
    for name in ("index.html", *WASM_FILES):
        if not (root / name).is_file():
            print("Spielpaket unvollstaendig: " + str(root / name), file=sys.stderr)
            return 2
    try:
        # LAN games use the existing map/multiplayer menu unless an entry is
        # explicitly chosen; the current private map has its own local mode.
        page = PUBLIC_START_PAGE if args.lan and args.page is None else start_page(root, args.page)
    except ValueError as error:
        parser.error(str(error))
    try:
        server, port, reused = bind_server(root, args.port, args.strict_port, args.verbose, args.lan)
    except OSError as error:
        print("Lokaler Spielserver konnte nicht starten: " + str(error), file=sys.stderr)
        return 2
    lan = None
    if args.lan and not reused:
        try:
            server.lan_tickets = Tickets()
            server.lan_directory = RoomDirectory(args.lan, server.lan_tickets)
            lan = LanServer(args.lan, server.lan_tickets, server.lan_directory)
            server.lan_address = args.lan
        except OSError as error:
            server.lan_directory.close()
            server.server_close()
            print(f"LAN-Einladungen auf {args.lan}:{LAN_PORT} konnten nicht starten: {error}", file=sys.stderr)
            return 2
    url = f"http://{HOST}:{port}{page}"
    print("READY " + json.dumps({"url": url, "host": HOST, "port": port, "reused": reused}), flush=True)
    if port != args.port and args.port:
        print(f"Port{args.port} war belegt. Dieses Mal wird Port{port} verwendet; gespeicherte Spiele im Browser sind an den Port gebunden.", flush=True)
    if reused:
        print("Der Spielserver fuer diesen Ordner laeuft bereits.", flush=True)
        if not args.no_browser:
            open_game(url)
        return 0
    print("Spiel: " + url, flush=True)
    if lan:
        print(f"LAN-Einladungen im vertrauten lokalen Netz: {args.lan}:{LAN_PORT}", flush=True)
        print(f"Optionale LAN-Raumliste: UDP {DIRECTORY_PORT}", flush=True)
        threading.Thread(target=lan.serve_forever, kwargs={"poll_interval": 0.2}, daemon=True).start()
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
        server.lan_directory.close()
        if lan:
            lan.shutdown()
            lan.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
