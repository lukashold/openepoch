"""Optional, bounded signaling for direct WebRTC links on a trusted IPv4 LAN."""

import base64
import binascii
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import ipaddress
import json
import os
import re
import secrets
import socket
import threading
import time


LAN_PATH = '/__openepoch__/lan'
LAN_PORT = 8126
MAX_BODY = 135168
MAX_CODE = 131072
MAX_TICKETS = 24
TICKET_AGE = 600
CLAIM_AGE = 60
TOKEN = re.compile(r'[A-Za-z0-9_-]{32}')
ROOM = re.compile(r'[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}')
CODE = re.compile(r'[A-Za-z0-9_-]{1,131072}')
HUGE10_LAN_PROFILES = {
    'pangea-huge10-checkpoint-transfer-1': ('pangea-huge10', 'huge10'),
    'pangea-huge10-native-naval-1': ('pangea-huge10-naval', 'huge10Naval'),
    'pangea-huge10-native-pursuit-1': ('pangea-huge10-pursuit', 'huge10Pursuit'),
    'pangea-huge10-native-anchor-1': ('pangea-huge10-anchor', 'huge10Anchor'),
}
HUGE10_PROFILE = re.compile(r'(?:' + '|'.join(HUGE10_LAN_PROFILES) + r'):[0-9a-f]{64}')
PRIVATE_NETWORKS = tuple(map(ipaddress.IPv4Network, ('10.0.0.0/8', '172.16.0.0/12', '192.168.0.0/16')))


class SignalingError(Exception):
    def __init__(self, status, reason):
        super().__init__(reason)
        self.status = status


def private_ipv4(value):
    try:
        address = ipaddress.IPv4Address(value) if isinstance(value, str) else None
    except ipaddress.AddressValueError:
        address = None
    if address is None or str(address) != value or not any(address in network for network in PRIVATE_NETWORKS):
        raise SignalingError(400, 'private-address')
    return value


def token(value):
    if not isinstance(value, str) or TOKEN.fullmatch(value) is None:
        raise SignalingError(400, 'token')
    return value


def description(value, kind):
    if not isinstance(value, str) or CODE.fullmatch(value) is None:
        raise SignalingError(400, 'description')
    try:
        body = json.loads(base64.urlsafe_b64decode(value + '=' * (-len(value) % 4)))
        valid = (isinstance(body, dict) and set(body) == {'type', 'sdp'} and body['type'] == kind
                 and isinstance(body['sdp'], str) and body['sdp'].startswith('v=0\r\n'))
    except (ValueError, UnicodeError, binascii.Error, RecursionError):
        valid = False
    if not valid:
        raise SignalingError(400, 'description')
    return value


def lan_profile(value):
    if value is None:
        return {}
    if not isinstance(value, str) or HUGE10_PROFILE.fullmatch(value) is None:
        raise SignalingError(400, 'room')
    return {'profile': value}


def seat_assignment(request):
    room, seat, seats = request.get('room'), request.get('seat'), request.get('seats')
    profile = lan_profile(request.get('profile'))
    if (not isinstance(room, str) or ROOM.fullmatch(room) is None or type(seat) is not int
            or type(seats) is not int or not 1 <= seat < seats <= 4 or (profile and (seat, seats) != (1, 2))):
        raise SignalingError(400, 'room')
    return {'room': room, 'seat': seat, 'seats': seats, **profile}


class Tickets:
    def __init__(self, clock=time.monotonic):
        self.clock = clock
        self.lock = threading.Lock()
        self.entries = {}

    def assignment(self, key, owner):
        with self.lock:
            entry = self.entries.get(key)
            if (not entry or entry['expires'] <= self.clock() or 'claim' in entry
                    or not secrets.compare_digest(owner, entry['owner'])):
                raise SignalingError(404, 'ticket-expired')
            return entry['assignment'].copy()

    def discard(self, key, owner):
        with self.lock:
            entry = self.entries.get(key)
            if entry and secrets.compare_digest(owner, entry['owner']):
                del self.entries[key]

    def handle(self, request, local):
        action = request.get('action')
        with self.lock:
            now = self.clock()
            self.entries = {key: entry for key, entry in self.entries.items() if entry['expires'] > now}
            if local and action == 'publish':
                if len(self.entries) >= MAX_TICKETS:
                    raise SignalingError(429, 'room-limit')
                offer = description(request.get('offer'), 'offer')
                key, owner = secrets.token_urlsafe(24), secrets.token_urlsafe(24)
                self.entries[key] = {'owner': owner, 'offer': offer, 'expires': now + TICKET_AGE,
                                     'assignment': seat_assignment(request)}
                return {'ticket': key, 'owner': owner}
            if (local and action not in ('read', 'close')) or (not local and action not in ('claim', 'answer', 'release')):
                raise SignalingError(400, 'action')
            key = token(request.get('ticket'))
            entry = self.entries.get(key)
            if entry is None:
                raise SignalingError(404, 'ticket-expired')
            if local:
                if not secrets.compare_digest(token(request.get('owner')), entry['owner']):
                    raise SignalingError(404, 'ticket-expired')
                if action == 'close':
                    del self.entries[key]
                    return {}
                return {'answer': entry.get('answer')}
            if action == 'claim':
                if lan_profile(request.get('profile')).get('profile') != entry['assignment'].get('profile'):
                    raise SignalingError(404, 'ticket-expired')
                if 'answer' in entry or entry.get('claimUntil', 0) > now:
                    raise SignalingError(409, 'seat-claimed')
                claim = secrets.token_urlsafe(24)
                entry.update(claim=claim, claimUntil=now + CLAIM_AGE)
                return {'offer': entry['offer'], 'claim': claim, **entry['assignment']}
            claim = token(request.get('claim'))
            if not secrets.compare_digest(claim, entry.get('claim', '')) or entry.get('claimUntil', 0) <= now:
                raise SignalingError(409, 'seat-claimed')
            if action == 'release':
                if 'answer' not in entry:
                    del entry['claim'], entry['claimUntil']
                return {}
            answer = description(request.get('answer'), 'answer')
            if 'answer' in entry and entry['answer'] != answer:
                raise SignalingError(409, 'seat-claimed')
            entry['answer'] = answer
            return {}


def read_request(handler):
    length = handler.headers.get('Content-Length', '')
    if (handler.headers.get('Transfer-Encoding') or len(handler.headers.get_all('Content-Length', [])) != 1
            or re.fullmatch(r'[0-9]{1,6}', length) is None or not 0 < int(length) <= MAX_BODY):
        raise SignalingError(413, 'request-size')
    if handler.headers.get_content_type() != 'application/json':
        raise SignalingError(415, 'content-type')
    try:
        body = handler.rfile.read(int(length))
        request = json.loads(body)
    except (ValueError, UnicodeError, RecursionError):
        raise SignalingError(400, 'request') from None
    if len(body) != int(length) or not isinstance(request, dict):
        raise SignalingError(400, 'request')
    return request


def send_result(handler, status, result):
    body = json.dumps(result, separators=(',', ':')).encode('utf-8')
    handler.send_response(status)
    handler.send_header('Content-Type', 'application/json')
    handler.send_header('Content-Length', str(len(body)))
    handler.end_headers()
    handler.wfile.write(body)


def forward_http(address, fields):
    # Only this fixed protocol endpoint can be contacted. A numeric private
    # address, direct HTTP connection, and no redirect or proxy handling are required.
    conn = http.client.HTTPConnection(address, LAN_PORT, timeout=3)
    try:
        conn.request('POST', LAN_PATH, json.dumps(fields), {'Content-Type': 'application/json'})
        response = conn.getresponse()
        if response.status not in (200, 400, 403, 404, 409, 413, 415, 429):
            raise SignalingError(502, 'host-unreachable')
        result = json.loads(response.read(MAX_BODY + 1))
        if not isinstance(result, dict):
            raise SignalingError(502, 'host-unreachable')
        if response.status != 200:
            reason = result.get('error')
            accepted = ('ticket-expired', 'seat-claimed', 'room-expired', 'room-full', 'room-limit', 'join-expired')
            raise SignalingError(response.status, reason if reason in accepted else 'host-unreachable')
        return result
    except (OSError, ValueError, UnicodeError, http.client.HTTPException, RecursionError):
        raise SignalingError(502, 'host-unreachable') from None
    finally:
        conn.close()


def forward(request):
    address = private_ipv4(request.get('address'))
    action = request.get('action')
    fields = {'action': action, 'ticket': token(request.get('ticket'))}
    if action == 'claim':
        fields.update(lan_profile(request.get('profile')))
    if action in ('answer', 'release'):
        fields['claim'] = token(request.get('claim'))
    if action == 'answer':
        fields['answer'] = description(request.get('answer'), 'answer')
    result = forward_http(address, fields)
    if action == 'claim':
        assignment = seat_assignment(result)
        if assignment.get('profile') != fields.get('profile'):
            raise SignalingError(404, 'ticket-expired')
        return {'offer': description(result.get('offer'), 'offer'), 'claim': token(result.get('claim')), **assignment}
    return {}


def local_request(handler):
    from lan_directory import LOCAL_ACTIONS, REMOTE_ACTIONS, forward_directory

    handler.connection.settimeout(6)
    address, tickets = handler.server.lan_address, handler.server.lan_tickets
    host = handler.headers.get('Host', '')
    port = handler.server.server_address[1]
    try:
        if host not in {f'127.0.0.1:{port}', f'localhost:{port}'} or handler.headers.get('Origin') != f'http://{host}':
            raise SignalingError(403, 'origin')
        request = read_request(handler)
        action = request.get('action')
        if action == 'status':
            result = {'available': True, 'host': address, 'browse': 1}
            profile = lan_profile(request.get('profile')).get('profile')
            if profile:
                result[HUGE10_LAN_PROFILES[profile.partition(':')[0]][1]] = 1
        elif action in LOCAL_ACTIONS:
            result = handler.server.lan_directory.local(request)
        elif action in REMOTE_ACTIONS:
            result = forward_directory(request)
        elif action in ('claim', 'answer', 'release'):
            result = forward(request)
        elif tickets:
            result = tickets.handle(request, local=True)
        else:
            raise SignalingError(403, 'host-disabled')
        send_result(handler, 200, result)
    except SignalingError as error:
        send_result(handler, error.status, {'error': str(error)})
    except OSError:
        handler.close_connection = True


class LanHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        from lan_directory import REMOTE_ACTIONS

        try:
            private_ipv4(self.client_address[0])
            if (self.path != LAN_PATH or self.headers.get('Host') != f'{self.server.server_address[0]}:{LAN_PORT}'
                    or self.headers.get('Origin') is not None):
                raise SignalingError(403, 'endpoint')
            request = read_request(self)
            if request.get('action') in REMOTE_ACTIONS:
                result = self.server.directory.remote(request, self.client_address[0])
            else:
                result = self.server.tickets.handle(request, local=False)
            send_result(self, 200, result)
        except SignalingError as error:
            send_result(self, error.status, {'error': str(error)})
        except OSError:
            self.close_connection = True

    def end_headers(self):
        self.send_header('Cache-Control', 'no-store')
        self.send_header('X-Content-Type-Options', 'nosniff')
        super().end_headers()

    def log_message(self, *_args):
        pass


class LanServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = os.name != 'nt'

    def __init__(self, address, tickets, directory):
        self.tickets = tickets
        self.directory = directory
        self.slots = threading.BoundedSemaphore(16)
        super().__init__((private_ipv4(address), LAN_PORT), LanHandler)

    def server_bind(self):
        if os.name == 'nt' and hasattr(socket, 'SO_EXCLUSIVEADDRUSE'):
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
        super().server_bind()

    def process_request(self, request, client_address):
        if not self.slots.acquire(blocking=False):
            self.shutdown_request(request)
            return
        request.settimeout(3)
        try:
            super().process_request(request, client_address)
        except BaseException:
            self.slots.release()
            raise

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            self.slots.release()
