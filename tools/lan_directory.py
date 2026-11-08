"""Explicit LAN advertisements and per-seat host approval for OpenEpoch rooms."""

import json
import errno
import secrets
import socket
import threading
import time

from lan_signaling import ROOM, SignalingError, forward_http, lan_profile, private_ipv4, token


DIRECTORY_PORT = 8127
MAGIC = b'OpenEpoch-LAN-1\0'
ADVERTISEMENT_PERIOD = 4
ROOM_LEASE = 15
SCAN_LEASE = 8
ADVERT_AGE = 2 * ADVERTISEMENT_PERIOD
REQUEST_AGE = 60
APPROVAL_AGE = 40
MAX_DATAGRAM = 640
MAX_ROOMS = 32
MAX_REQUESTS = 12
LOCAL_ACTIONS = ('browse-start', 'browse-read', 'browse-stop', 'room-open', 'room-update', 'room-close', 'room-approve', 'room-reject')
REMOTE_ACTIONS = ('join-request', 'join-status', 'join-cancel')


def room_id(value):
    if not isinstance(value, str) or ROOM.fullmatch(value) is None:
        raise SignalingError(400, 'room')
    return value


def label(value):
    if not isinstance(value, str) or not 1 <= len(value.strip()) <= 40 or not value.isprintable():
        raise SignalingError(400, 'room-name')
    return value.strip()


def number(value, minimum, maximum):
    if type(value) is not int or not minimum <= value <= maximum:
        raise SignalingError(400, 'room')
    return value


def room_details(data):
    if not isinstance(data, dict):
        raise SignalingError(400, 'room')
    profile = lan_profile(data.get('profile'))
    seats = number(data.get('seats'), 2, 2 if profile else 4)
    ready = number(data.get('ready'), 1, seats)
    open_seats = number(data.get('open'), 0, seats - ready)
    mode = data.get('mode')
    if mode not in (('pangea-huge10',) if profile else ('ordinary', 'shared', 'grouped')):
        raise SignalingError(400, 'room')
    last_epoch = 14 if profile else 15
    epoch, final_epoch = number(data.get('epoch'), -1, last_epoch), number(data.get('finalEpoch'), -1, last_epoch)
    if 0 <= final_epoch < epoch:
        raise SignalingError(400, 'room')
    tick = data.get('tick')
    return {'room': room_id(data.get('room')), 'name': label(data.get('name')), 'seats': seats, 'ready': ready,
            'open': open_seats, 'mode': mode, 'climate': number(data.get('climate'), 0, 0 if profile else 3), 'epoch': epoch,
            'finalEpoch': final_epoch, 'ai': number(data.get('ai'), 8, 8) if profile else number(data.get('ai'), 0, 2),
            'tick': None if tick is None else number(tick, 1 if profile else 0, 4294967294 if profile else 4294967295),
            **profile}


def datagram(details, goodbye=False):
    fields = {'room': details['room'], 'closed': True} if goodbye else details
    if goodbye:
        fields.update(lan_profile(details.get('profile')))
    raw = MAGIC + json.dumps(fields, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
    assert len(raw) <= MAX_DATAGRAM
    return raw


def forward_directory(request):
    address = private_ipv4(request.get('address'))
    action = request['action']
    fields = {'action': action, 'room': room_id(request.get('room')), **lan_profile(request.get('profile'))}
    if action == 'join-request':
        fields['name'] = label(request.get('name'))
    else:
        fields.update(request=token(request.get('request')), guest=token(request.get('guest')))
    result = forward_http(address, fields)
    if action == 'join-request':
        return {'request': token(result.get('request')), 'guest': token(result.get('guest'))}
    if action == 'join-cancel':
        return {}
    state = result.get('state')
    if state == 'approved':
        return {'state': state, 'ticket': token(result.get('ticket'))}
    if state in ('pending', 'rejected'):
        return {'state': state}
    raise SignalingError(502, 'host-unreachable')


class BrowserScan:
    def __init__(self, clock, profile=None):
        self.clock = clock
        self.profile = profile
        self.key = secrets.token_urlsafe(24)
        self.started = clock()
        self.expires = self.started + SCAN_LEASE
        self.lock = threading.Lock()
        self.rows = {}
        self.stop = threading.Event()
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        try:
            self.socket.bind(('', DIRECTORY_PORT))
            self.socket.settimeout(0.25)
        except OSError:
            self.socket.close()
            raise SignalingError(503, 'browse-unavailable') from None
        self.thread = threading.Thread(target=self.listen, daemon=True)
        self.thread.start()

    def listen(self):
        try:
            while not self.stop.is_set():
                now = self.clock()
                with self.lock:
                    if now >= self.expires:
                        return
                    self.rows = {key: value for key, value in self.rows.items() if value[0] >= now}
                try:
                    raw, (address, _) = self.socket.recvfrom(MAX_DATAGRAM + 1)
                except socket.timeout:
                    continue
                except OSError as error:
                    if error.errno == errno.EMSGSIZE or getattr(error, 'winerror', None) == 10040:
                        continue
                    return
                if len(raw) > MAX_DATAGRAM or not raw.startswith(MAGIC):
                    continue
                try:
                    private_ipv4(address)
                    body = json.loads(raw[len(MAGIC):])
                    if not isinstance(body, dict):
                        continue
                    if lan_profile(body.get('profile')).get('profile') != self.profile:
                        continue
                    if body.get('closed') is True:
                        with self.lock:
                            self.rows.pop((address, room_id(body.get('room'))), None)
                        continue
                    details = room_details(body)
                except (SignalingError, ValueError, RecursionError):
                    continue
                key = (address, details['room'])
                with self.lock:
                    if key in self.rows or len(self.rows) < MAX_ROOMS:
                        self.rows[key] = (self.clock() + ADVERT_AGE, {**details, 'address': address})
        finally:
            self.socket.close()
            self.stop.set()

    def read(self):
        with self.lock:
            now = self.clock()
            if self.stop.is_set() or now >= self.expires:
                raise SignalingError(404, 'browse-expired')
            self.expires = now + SCAN_LEASE
            rooms = [row for expires, row in self.rows.values() if expires >= now]
            return {'rooms': sorted(rooms, key=lambda row: (row['name'].casefold(), row['address'], row['room'])),
                    'waiting': now - self.started < ADVERTISEMENT_PERIOD + 0.5}

    def close(self):
        self.stop.set()
        self.thread.join()


class RoomDirectory:
    def __init__(self, address, tickets, clock=time.monotonic, emit=None):
        self.address = address
        self.tickets = tickets
        self.clock = clock
        self.emit = emit or self.broadcast
        self.lock = threading.Lock()
        self.current = None
        self.scan = None
        self.stopped = threading.Event()
        self.advertiser = None
        if address:
            self.advertiser = threading.Thread(target=self.tick, daemon=True)
            self.advertiser.start()

    def broadcast(self, packet):
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as outgoing:
            outgoing.bind((self.address, 0))
            outgoing.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
            outgoing.sendto(packet, ('255.255.255.255', DIRECTORY_PORT))

    def announce(self, room, goodbye=False):
        if not goodbye:
            room['nextAd'] = self.clock() + ADVERTISEMENT_PERIOD
        try:
            self.emit(datagram(room['details'], goodbye))
        except OSError:
            pass

    def tick(self):
        while not self.stopped.wait(0.25):
            with self.lock:
                now = self.clock()
                self.prune(now)
                if self.current and now >= self.current['nextAd']:
                    self.announce(self.current)

    def revoke(self, entry):
        if 'ticket' in entry:
            self.tickets.discard(entry['ticket'], entry['ticketOwner'])

    def drop_room(self):
        if self.current:
            self.announce(self.current, goodbye=True)
            for entry in self.current['requests'].values():
                self.revoke(entry)
            self.current = None

    def prune(self, now):
        if not self.current:
            return
        if self.current['expires'] <= now:
            self.drop_room()
            return
        entries = self.current['requests']
        for key, entry in tuple(entries.items()):
            if entry['expires'] <= now:
                self.revoke(entry)
                del entries[key]

    def browse(self, action, request):
        if action == 'browse-start':
            if self.scan and not self.scan.stop.is_set():
                raise SignalingError(409, 'browse-in-use')
            self.scan = BrowserScan(self.clock, lan_profile(request.get('profile')).get('profile'))
            return {'scan': self.scan.key}
        if not self.scan or not secrets.compare_digest(token(request.get('scan')), self.scan.key):
            raise SignalingError(404, 'browse-expired')
        if action == 'browse-read':
            return self.scan.read()
        self.scan.close()
        self.scan = None
        return {}

    def local(self, request):
        action = request.get('action')
        with self.lock:
            if action in ('browse-start', 'browse-read', 'browse-stop'):
                return self.browse(action, request)
            if not self.address:
                raise SignalingError(403, 'host-disabled')
            now = self.clock()
            self.prune(now)
            if action == 'room-open':
                if self.current:
                    raise SignalingError(409, 'room-already-listed')
                room = {'owner': secrets.token_urlsafe(24), 'details': room_details(request.get('details')),
                        'expires': now + ROOM_LEASE, 'requests': {}}
                self.current = room
                self.announce(room)
                return {'owner': room['owner']}
            room = self.current
            if not room or not secrets.compare_digest(token(request.get('owner')), room['owner']):
                raise SignalingError(404, 'room-expired')
            if action == 'room-close':
                self.drop_room()
                return {}
            if action == 'room-update':
                details = room_details(request.get('details'))
                if any(details.get(field) != room['details'].get(field) for field in ('room', 'seats', 'profile')):
                    raise SignalingError(400, 'room')
                room['expires'] = now + ROOM_LEASE
                if details != room['details']:
                    room['details'] = details
                    self.announce(room)
                return {'pending': [{'request': key, 'name': entry['name'], 'address': entry['address']}
                                    for key, entry in room['requests'].items() if entry['state'] == 'pending']}
            entry = room['requests'].get(token(request.get('request')))
            if not entry or entry['state'] != 'pending':
                raise SignalingError(404, 'join-expired')
            if action == 'room-reject':
                entry['state'] = 'rejected'
                return {}
            if action != 'room-approve':
                raise SignalingError(400, 'action')
            ticket, owner = token(request.get('ticket')), token(request.get('ticketOwner'))
            assignment = self.tickets.assignment(ticket, owner)
            if any(assignment.get(field) != room['details'].get(field) for field in ('room', 'seats', 'profile')):
                raise SignalingError(400, 'room')
            for other in room['requests'].values():
                if other.get('seat') == assignment['seat'] or other.get('ticket') == ticket:
                    raise SignalingError(409, 'seat-claimed')
            entry.update(state='approved', seat=assignment['seat'], ticket=ticket, ticketOwner=owner,
                         expires=now + APPROVAL_AGE)
            return {}

    def remote(self, request, address):
        action = request.get('action')
        with self.lock:
            now = self.clock()
            self.prune(now)
            room = self.current
            if (not room or room['details']['room'] != room_id(request.get('room'))
                    or room['details'].get('profile') != lan_profile(request.get('profile')).get('profile')):
                raise SignalingError(404, 'room-expired')
            entries = room['requests']
            if action == 'join-request':
                if not room['details']['open']:
                    raise SignalingError(409, 'room-full')
                if len(entries) >= MAX_REQUESTS:
                    raise SignalingError(429, 'room-limit')
                key, guest = secrets.token_urlsafe(24), secrets.token_urlsafe(24)
                entries[key] = {'secret': guest, 'state': 'pending', 'name': label(request.get('name')),
                                'address': address, 'expires': now + REQUEST_AGE}
                return {'request': key, 'guest': guest}
            key = token(request.get('request'))
            entry = entries.get(key)
            if (not entry or entry['address'] != address
                    or not secrets.compare_digest(token(request.get('guest')), entry['secret'])):
                raise SignalingError(404, 'join-expired')
            if action == 'join-cancel':
                self.revoke(entry)
                del entries[key]
                return {}
            result = {'state': entry['state']}
            if entry['state'] == 'approved':
                result['ticket'] = entry['ticket']
            return result

    def close(self):
        self.stopped.set()
        with self.lock:
            self.drop_room()
            if self.scan:
                self.scan.close()
        if self.advertiser:
            self.advertiser.join()
