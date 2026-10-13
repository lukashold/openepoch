//! Player commands as data.
//!
//! Every order a player gives becomes one small record. Single-player applies it
//! on the spot; in lockstep multiplayer it is serialised into `outbox`, sent to
//! the peer, and both sides apply it on the same future tick. `apply` is the
//! only way input reaches the simulation, and it only reads simulation state,
//! so both peers stay bit-identical.

use crate::defs::*;
use crate::game::*;
use crate::map::*;

pub const C_MOVE: u8 = 1;
pub const C_AMOVE: u8 = 2;
pub const C_ATTACK: u8 = 3;
pub const C_WORK: u8 = 4;
pub const C_GATHER: u8 = 5;
pub const C_BOARD: u8 = 6;
pub const C_UNLOAD: u8 = 7;
pub const C_RALLY: u8 = 8;
pub const C_STOP: u8 = 9;
pub const C_PLACE: u8 = 10;
pub const C_TRAIN: u8 = 11;
pub const C_CANCEL: u8 = 12;
pub const C_DELETE: u8 = 13;

const HEAD: usize = 14; // kind, count, a, b, c

impl Game {
    /// Issue a command as the local player. `hs` are entity handles.
    pub fn submit(&mut self, kind: u8, a: f32, b: f32, c: i32, hs: &[u32]) {
        if !self.net {
            let team = self.local;
            self.apply(team, kind, a, b, c, hs);
            return;
        }
        // big selections are split so the count fits a byte
        let mut chunks = hs.chunks(250);
        let mut first = true;
        loop {
            let part = chunks.next();
            if part.is_none() && !first {
                break;
            }
            first = false;
            let part = part.unwrap_or(&[]);
            self.outbox.push(kind);
            self.outbox.push(part.len() as u8);
            self.outbox.extend_from_slice(&a.to_le_bytes());
            self.outbox.extend_from_slice(&b.to_le_bytes());
            self.outbox.extend_from_slice(&c.to_le_bytes());
            for h in part {
                self.outbox.extend_from_slice(&h.to_le_bytes());
            }
        }
    }

    /// Apply every command in `inbox` on behalf of `team`.
    pub fn net_apply(&mut self, team: u8) {
        let buf = std::mem::take(&mut self.inbox);
        let mut o = 0;
        let mut hs: Vec<u32> = Vec::new();
        while o + HEAD <= buf.len() {
            let (kind, n) = (buf[o], buf[o + 1] as usize);
            let f = |k: usize| [buf[o + k], buf[o + k + 1], buf[o + k + 2], buf[o + k + 3]];
            let (a, b, c) = (f32::from_le_bytes(f(2)), f32::from_le_bytes(f(6)), i32::from_le_bytes(f(10)));
            o += HEAD;
            if o + n * 4 > buf.len() {
                break;
            }
            hs.clear();
            for k in 0..n {
                hs.push(u32::from_le_bytes([buf[o + k * 4], buf[o + k * 4 + 1], buf[o + k * 4 + 2], buf[o + k * 4 + 3]]));
            }
            o += n * 4;
            self.apply(team, kind, a, b, c, &hs);
        }
        self.inbox = buf;
    }

    pub fn apply(&mut self, team: u8, kind: u8, a: f32, b: f32, c: i32, hs: &[u32]) {
        if !a.is_finite() || !b.is_finite() || team as usize >= self.nplayers {
            return;
        }
        // never trust the wire: only our own living entities take orders
        let ids: Vec<usize> = hs.iter().filter_map(|&h| self.get(h)).filter(|&i| self.ents[i].team == team).collect();
        let target = self.get(c as u32);
        match kind {
            C_MOVE => self.order_move(&ids, a, b, false),
            C_AMOVE => self.order_move(&ids, a, b, true),
            C_ATTACK => {
                if let Some(j) = target.filter(|&j| self.ents[j].team != team) {
                    self.order_attack(&ids, j);
                }
            }
            C_WORK => {
                if let Some(j) = target.filter(|&j| self.ents[j].team == team && DEFS[self.ents[j].def as usize].building) {
                    self.order_building(&ids, j);
                }
            }
            C_GATHER => {
                if c >= 0 && (c as usize) < NN && self.map.res[c as usize] != R_NONE {
                    self.order_gather(&ids, c as usize);
                }
            }
            C_BOARD => {
                if let Some(j) = target.filter(|&j| self.ents[j].team == team) {
                    self.order_board(&ids, j);
                }
            }
            C_UNLOAD => self.order_unload(&ids, a, b),
            C_RALLY => {
                for &i in &ids {
                    if DEFS[self.ents[i].def as usize].building {
                        self.ents[i].rx = a.clamp(1.0, N as f32 - 1.0);
                        self.ents[i].ry = b.clamp(1.0, N as f32 - 1.0);
                    }
                }
            }
            C_STOP => self.order_stop(&ids),
            C_PLACE => {
                if (0..DEFS.len() as i32).contains(&c) {
                    let bi = self.place(c as u8, a as i32, b as i32, team);
                    if bi != NOIDX {
                        self.order_building(&ids, bi);
                    }
                }
            }
            C_TRAIN => {
                if let (Some(&i), 0..=255) = (ids.first(), c) {
                    self.train(i, c as u8);
                }
            }
            C_CANCEL => {
                if let Some(&i) = ids.first() {
                    self.cancel(i);
                }
            }
            C_DELETE => {
                for &i in &ids {
                    self.ents[i].hp = 0.0;
                }
            }
            _ => {}
        }
    }
}
