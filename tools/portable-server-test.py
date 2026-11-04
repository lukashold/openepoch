"""Independent real-HTTP checks for the portable launcher (Python standard library).

Run with Python3 on the test host. This does not start a browser, modify game
assets, install anything, or claim a macOS launch when executed on Windows.
"""
from __future__ import annotations

import hashlib
import http.client
import json
import os
from pathlib import Path
import queue
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.error
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PROJECT = Path(__file__).resolve().parent.parent
SERVER = PROJECT / 'tools/serve_game.py'
DOCS = PROJECT / 'docs'
TIMEOUT = 12


class RunningServer:
    def __init__(self, root, port=0, strict=False):
        command = [sys.executable, '-u', str(SERVER), '--root', str(root), '--port', str(port), '--no-browser']
        if strict:
            command.append('--strict-port')
        options = {'creationflags': subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == 'nt' else {}
        self.process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                        text=True, encoding='utf-8', errors='replace', **options)
        self.lines = []
        self.output = queue.Queue()
        self.ready = None
        self.closed = False
        def read():
            for line in self.process.stdout:
                self.lines.append(line.rstrip())
                self.output.put(line)
            self.output.put(None)
        self.reader = threading.Thread(target=read, daemon=True)
        self.reader.start()

    def wait_ready(self):
        deadline = time.monotonic() + TIMEOUT
        while time.monotonic() < deadline:
            try:
                line = self.output.get(timeout=max(.01, deadline-time.monotonic()))
            except queue.Empty:
                break
            if line is None:
                break
            if line.startswith('READY '):
                self.ready = json.loads(line[6:])
                parsed = urllib.parse.urlsplit(self.ready['url'])
                if parsed.scheme != 'http' or parsed.hostname != '127.0.0.1' or parsed.port != self.ready['port']:
                    raise AssertionError('READY does not point at its literal loopback listener: '+line)
                if self.ready['host'] != '127.0.0.1' or not 1 <= self.ready['port'] <= 65535:
                    raise AssertionError('Invalid bind metadata: '+line)
                self.base = 'http://127.0.0.1:'+str(self.ready['port'])
                return self.ready
        raise AssertionError('Server did not become ready: '+'\n'.join(self.lines[-15:]))

    def close(self, expect_clean=False):
        if self.closed:
            return
        self.closed = True
        if self.process.poll() is None:
            try:
                self.process.send_signal(signal.CTRL_BREAK_EVENT if os.name == 'nt' else signal.SIGINT)
                self.process.wait(timeout=8)
            except (subprocess.TimeoutExpired, OSError):
                self.process.kill()
                self.process.wait(timeout=4)
                if expect_clean:
                    raise AssertionError('Graceful interrupt did not stop server: '+'\n'.join(self.lines[-10:]))
        self.reader.join(timeout=2)
        self.process.stdout.close()
        if expect_clean and self.process.returncode != 0:
            raise AssertionError('Interrupt exited uncleanly: '+str(self.process.returncode)+'\n'+'\n'.join(self.lines[-10:]))


class ForeignHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = b'{"application":"different-application"}'
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *args):
        pass


class ForeignServer(ThreadingHTTPServer):
    allow_reuse_address = False
    daemon_threads = True
    def server_bind(self):
        if os.name == 'nt' and hasattr(socket, 'SO_EXCLUSIVEADDRUSE'):
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
        super().server_bind()


class PortableServerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not SERVER.is_file():
            raise AssertionError('Expected portable server '+str(SERVER))

    def setUp(self):
        self.servers = []
        scratch = PROJECT.parents[1]/'work/portable-server-tests'
        scratch.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix='OpenEpoch portable test ', dir=scratch)
        assert Path(self.temporary.name).resolve().parent == scratch.resolve()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)/'Spiel mit Leerzeichen und Umlaut ä'
        self.root.mkdir()
        self.make_fixture(self.root, b'fixture-one')
        self.addCleanup(self.close_servers)

    @staticmethod
    def make_fixture(root, body):
        (root/'index.html').write_bytes(b'<!doctype html><title>portable fixture</title>'+body)
        for name in ('openepoch.wasm', 'openepoch-ee2.wasm'):
            (root/name).write_bytes(b'\x00asm\x01\x00\x00\x00')
        (root/'fixture.json').write_text(json.dumps({'message': 'ä / portable'}), encoding='utf-8')

    def launch(self, root=None, port=0, strict=False):
        server = RunningServer(root or self.root, port, strict)
        self.servers.append(server)
        server.wait_ready()
        return server

    def close_servers(self):
        for server in reversed(self.servers):
            server.close()

    def request(self, server, path='/', method='GET'):
        url = urllib.parse.urljoin(server.base+'/', path)
        with urllib.request.urlopen(urllib.request.Request(url, method=method), timeout=4) as response:
            return response.status, response.headers, response.read()

    def status(self, server):
        code, headers, body = self.request(server, '/__openepoch__/status')
        self.assertEqual(code, 200)
        self.assertEqual(headers.get_content_type(), 'application/json')
        value = json.loads(body)
        self.assertEqual(value['application'], 'openepoch')
        self.assertEqual(value['serverVersion'], 1)
        self.assertRegex(value['rootToken'], r'^[0-9a-f]{16}$')
        return value

    def test_bound_before_ready_and_root_with_spaces(self):
        server = self.launch()
        self.assertFalse(server.ready['reused'])
        self.assertEqual(self.request(server, '/?rules=ee2')[2], (self.root/'index.html').read_bytes())
        self.assertEqual(json.loads(self.request(server, '/fixture.json')[2])['message'], 'ä / portable')
        status = self.status(server)
        self.assertEqual(status['wasmSha256'], hashlib.sha256((self.root/'openepoch-ee2.wasm').read_bytes()).hexdigest())
        # This is another local loopback address, never an external connection.
        # A wildcard 0.0.0.0 listener would prevent the exclusive second bind.
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as check:
            if os.name == 'nt':
                check.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
            check.bind(('127.0.0.2', server.ready['port']))

    def test_real_http_wasm_json_png_mp3_and_pcm(self):
        server = self.launch(DOCS)
        for filename in ('openepoch.wasm', 'openepoch-ee2.wasm'):
            code, headers, body = self.request(server, '/'+filename+'?portable-test=1')
            self.assertEqual(code, 200)
            self.assertEqual(headers.get_content_type(), 'application/wasm')
            self.assertEqual(body, (DOCS/filename).read_bytes())
            self.assertTrue(body.startswith(b'\x00asm'))
            _, head, empty = self.request(server, '/'+filename, method='HEAD')
            self.assertEqual(int(head['Content-Length']), len(body))
            self.assertEqual(empty, b'')
        models_path = DOCS/'local-original/models/manifest.json'
        audio_path = DOCS/'local-original/audio/manifest.json'
        textures_path = DOCS/'local-original/textures.json'
        for path in (models_path, audio_path, textures_path):
            _, headers, body = self.request(server, '/'+path.relative_to(DOCS).as_posix())
            self.assertEqual(headers.get_content_type(), 'application/json')
            self.assertEqual(json.loads(body), json.loads(path.read_text(encoding='utf-8')))
        models = json.loads(models_path.read_text(encoding='utf-8'))
        first = models['models'][0]
        mesh = json.loads(self.request(server, first['url'])[2])
        self.assertTrue(mesh['parts'])
        texture = next(part['textureUrl'] for part in mesh['parts'] if part.get('textureUrl'))
        _, headers, png = self.request(server, texture)
        self.assertEqual(headers.get_content_type(), 'image/png')
        self.assertTrue(png.startswith(b'\x89PNG\r\n\x1a\n'))
        audio = json.loads(audio_path.read_text(encoding='utf-8'))
        mp3 = audio['voices']['8']['select'][0]
        _, headers, body = self.request(server, '/local-original/audio/'+urllib.parse.quote(mp3))
        self.assertEqual(headers.get_content_type(), 'audio/mpeg')
        self.assertEqual(hashlib.sha256(body).hexdigest(), audio['assets'][mp3.lower()]['sha256'])
        pcm = next(record for record in audio['assets'].values() if record.get('derived_from'))
        _, headers, body = self.request(server, '/local-original/audio/'+urllib.parse.quote(pcm['file']))
        self.assertIn(headers.get_content_type(), ('audio/wav', 'audio/x-wav', 'audio/vnd.wave', 'audio/wave'))
        self.assertEqual(hashlib.sha256(body).hexdigest(), pcm['sha256'])
        self.assertTrue(body.startswith(b'RIFF') and body[8:12] == b'WAVE')

    def test_identical_root_reuses_existing_listener(self):
        first = self.launch()
        original = self.status(first)
        second = self.launch(self.root, first.ready['port'])
        self.assertTrue(second.ready['reused'])
        self.assertEqual(second.ready['port'], first.ready['port'])
        self.assertEqual(self.status(second)['rootToken'], original['rootToken'])
        self.assertIsNone(first.process.poll())
        second.process.wait(timeout=4)
        self.assertEqual(second.process.returncode, 0)
        self.assertEqual(self.request(first)[2], (self.root/'index.html').read_bytes())

    def test_other_root_uses_fallback_and_preserves_original(self):
        first = self.launch()
        root2 = self.root.parent/'Anderes Spiel'
        root2.mkdir()
        self.make_fixture(root2, b'fixture-two')
        second = self.launch(root2, first.ready['port'])
        self.assertFalse(second.ready['reused'])
        self.assertNotEqual(second.ready['port'], first.ready['port'])
        self.assertNotEqual(self.status(second)['rootToken'], self.status(first)['rootToken'])
        self.assertEqual(self.request(second)[2], (root2/'index.html').read_bytes())
        self.assertEqual(self.request(first)[2], (self.root/'index.html').read_bytes())

    def test_foreign_application_not_reused_and_strict_conflict_fails(self):
        foreign = ForeignServer(('127.0.0.1', 0), ForeignHandler)
        threading.Thread(target=foreign.serve_forever, daemon=True).start()
        self.addCleanup(foreign.server_close)
        self.addCleanup(foreign.shutdown)
        port = foreign.server_port
        fallback = self.launch(port=port)
        self.assertFalse(fallback.ready['reused'])
        self.assertNotEqual(fallback.ready['port'], port)
        strict = RunningServer(self.root, port, True)
        self.servers.append(strict)
        strict.process.wait(timeout=TIMEOUT)
        strict.reader.join(timeout=2)
        self.assertNotEqual(strict.process.returncode, 0)
        self.assertFalse(any(line.startswith('READY ') for line in strict.lines))

    def test_all_eleven_candidates_occupied_uses_ephemeral(self):
        occupied = []
        for _ in range(20):
            first = ForeignServer(('127.0.0.1', 0), ForeignHandler)
            occupied = [first]
            if first.server_port > 65525:
                first.server_close()
                continue
            try:
                for port in range(first.server_port+1, first.server_port+11):
                    occupied.append(ForeignServer(('127.0.0.1', port), ForeignHandler))
                break
            except OSError:
                for item in occupied:
                    item.server_close()
                occupied = []
        self.assertEqual(len(occupied), 11, 'Could not reserve a safe, consecutive local-only test range')
        for item in occupied:
            threading.Thread(target=lambda s=item:s.serve_forever(poll_interval=.02), daemon=True).start()
            self.addCleanup(item.server_close)
            self.addCleanup(item.shutdown)
        server = self.launch(port=occupied[0].server_port)
        self.assertFalse(server.ready['reused'])
        self.assertNotIn(server.ready['port'], [item.server_port for item in occupied])
        self.assertEqual(self.request(server)[2], (self.root/'index.html').read_bytes())

    def test_untrusted_host_and_directory_listing_are_denied(self):
        (self.root/'no-index').mkdir()
        server = self.launch()
        connection = http.client.HTTPConnection('127.0.0.1', server.ready['port'], timeout=4)
        self.addCleanup(connection.close)
        connection.request('GET', '/', headers={'Host': 'untrusted.example'})
        response = connection.getresponse()
        self.assertEqual(response.status, 403)
        response.read()
        with self.assertRaises(urllib.error.HTTPError) as caught:
            self.request(server, '/no-index/')
        self.assertEqual(caught.exception.code, 404)
        caught.exception.close()

    def test_traversal_cannot_read_outside_document_root(self):
        secret = self.root.parent/'portable-private.txt'
        secret.write_bytes(b'OUTSIDE-DOCROOT-NEVER-SERVED')
        server = self.launch()
        for path in ('/%2e%2e/portable-private.txt', '/../portable-private.txt', '/%2f..%2fportable-private.txt'):
            try:
                body = self.request(server, path)[2]
            except urllib.error.HTTPError as error:
                self.assertIn(error.code, (400, 403, 404))
                error.close()
            else:
                self.assertNotIn(secret.read_bytes(), body)

    def test_graceful_shutdown_releases_actual_port(self):
        server = self.launch()
        port = server.ready['port']
        self.status(server)
        server.close(expect_clean=True)
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as check:
            check.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            check.bind(('127.0.0.1', port))
        with self.assertRaises(OSError):
            socket.create_connection(('127.0.0.1', port), timeout=.4)


if __name__ == '__main__':
    print('Portable server checks on '+sys.platform+'; no browser or macOS execution claim.', flush=True)
    unittest.main(verbosity=2)
