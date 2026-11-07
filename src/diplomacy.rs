//! Directed hostility, mutual treaties and negotiated full/none shared sight.

//! Treaty vocabulary follows the local EE2 headers/manual. AI negotiation

//! thresholds below are independent deterministic gameplay policy.

use crate::{defs::*, game::*, map::*};

use core::fmt::Write;

pub const WAR: u8 = 0;
pub const NEUTRAL: u8 = 1;
pub const ALLIED: u8 = 2;

pub const DIP_MAGIC: u32 = 0x3150_4944; // DIP1

pub const DIP_VERSION: u32 = 1;

pub const DIP_LOGS: usize = 12;

// Header, player/visibility metadata, byte matrices, five u32 matrices, log count,

// fixed 12x16-byte events, and the four genuine exploration maps.

pub const DIP_BYTES: usize = 8 + 12 + 64 + 320 + 4 + DIP_LOGS * 16 + 4 * NN;

#[derive(Clone, Copy, Default)]

pub struct DipEvent {
    pub tick: u32,
    pub from: u8,
    pub to: u8,
    pub kind: u8,
    pub reason: u8,
    pub value: u32,
    pub resource: u32,
}

pub struct Diplomacy {
    pub relation: [u8; 16],
    pub offers: [u8; 16],
    pub offer_vision: [u8; 16],
    pub sharing: [u8; 16],

    pub offer_tick: [u32; 16],
    pub cooldown: [u32; 16],
    pub gifts: [u32; 16],
    pub neutral_since: [u32; 16],
    pub war_tick: [u32; 16],

    pub visibility: u8,
    pub log: Vec<DipEvent>,
}

impl Default for Diplomacy {
    fn default() -> Self {
        let mut relation = [WAR; 16];
        for t in 0..4 {
            relation[t * 4 + t] = ALLIED;
        }
        Self {
            relation,
            offers: [0; 16],
            offer_vision: [0; 16],
            sharing: [0; 16],
            offer_tick: [0; 16],
            cooldown: [0; 16],
            gifts: [0; 16],
            neutral_since: [0; 16],
            war_tick: [0; 16],
            visibility: 0,
            log: Vec::with_capacity(DIP_LOGS),
        }
    }
}

impl Game {
    #[inline]
    pub fn relation(&self, from: u8, to: u8) -> u8 {
        self.diplomacy.relation[from as usize * 4 + to as usize]
    }

    #[inline]
    pub fn hostile(&self, from: u8, to: u8) -> bool {
        from != to && self.relation(from, to) == WAR
    }

    #[inline]
    pub fn allied(&self, from: u8, to: u8) -> bool {
        from == to || (self.relation(from, to) == ALLIED && self.relation(to, from) == ALLIED)
    }

    #[inline]
    pub fn shares_to(&self, donor: u8, viewer: u8) -> bool {
        donor != viewer
            && self.allied(donor, viewer)
            && self.diplomacy.sharing[donor as usize * 4 + viewer as usize] == 1
    }

    pub fn real_fog(&self, team: u8, tile: usize) -> u8 {
        if self.bench > 0 {
            return 255;
        }
        let mut v = self.team_vis[team as usize * NN + tile];

        for p in 0..self.nplayers {
            if self.shares_to(p as u8, team) {
                v = v.max(self.team_vis[p * NN + tile]);
            }
        }
        v
    }

    pub fn footprint_explored(&self, team: u8, def:DefId, tx: i32, ty: i32) -> bool {
        self.footprint_known(team,def,tx,ty,0)
    }

    pub(crate) fn display_visibility(&self) -> u8 {
        #[cfg(feature="ee2-native13")]
        if self.network_observer(self.local) { return 2; }
        match (self.diplomacy.visibility, self.fog_inverted) {
            (2, true) => 1,
            (_, true) => 2,
            (mode, false) => mode,
        }
    }

    pub fn refresh_display_fog(&mut self) {
        if self.bench > 0 {
            self.vis.fill(255);
            return;
        }

        // Copy our real map, then combine only active, explicitly granted ally sight.

        let start = self.local as usize * NN;
        self.vis.copy_from_slice(&self.team_vis[start..start + NN]);

        for p in 0..self.nplayers {
            if self.shares_to(p as u8, self.local) {
                for i in 0..NN {
                    self.vis[i] = self.vis[i].max(self.team_vis[p * NN + i]);
                }
            }
        }

        let visibility = self.display_visibility();
        if visibility == 1 {
            for v in &mut self.vis {
                *v = (*v).max(128);
            }
        }

        if visibility == 2 {
            self.vis.fill(255);
        }
    }

    fn dip_event(&mut self, from: u8, to: u8, kind: u8, reason: u8, value: u32, resource: u32) {
        if self.diplomacy.log.len() == DIP_LOGS {
            self.diplomacy.log.remove(0);
        }

        self.diplomacy.log.push(DipEvent {
            tick: self.tick,
            from,
            to,
            kind,
            reason,
            value,
            resource,
        });
    }

    pub fn diplomacy_valid(
        &self,
        from: u8,
        to: u32,
        action: u32,
        resource: u32,
        amount: f32,
    ) -> bool {
        if self.bench > 0
            || from as usize >= self.nplayers
            || to as usize >= self.nplayers
            || to == from as u32
            || action > 7
            || !amount.is_finite()
            || amount.fract() != 0.0
            || amount < 0.0
            || !self.players[from as usize].alive
            || !self.players[to as usize].alive
        {
            return false;
        }

        let to = to as u8;
        let i = from as usize * 4 + to as usize;
        let j = to as usize * 4 + from as usize;

        if action != 5 && action != 1 && resource != 0 {
            return false;
        }

        if action != 5 && action != 7 && amount != 0.0 {
            return false;
        }

        #[cfg(not(feature = "ee2-native13"))]
        let alliance_allowed = true;
        #[cfg(feature = "ee2-native13")]
        let alliance_allowed = self.sole_alliances_allowed();
        match action {
            0 | 1 => {
                self.diplomacy.offers[i] == 0
                    && self.tick >= self.diplomacy.cooldown[i]
                    && if action == 0 {
                        self.relation(from, to) == WAR || self.relation(to, from) == WAR
                    } else {
                        alliance_allowed && resource <= 1
                            && self.relation(from, to) == NEUTRAL
                            && self.relation(to, from) == NEUTRAL
                    }
            }

            2 => self.relation(from, to) != WAR,

            3 | 4 => {
                self.diplomacy.offers[j] != 0
                    && (self.diplomacy.offers[j] <= 2 || self.allied(from, to))
                    && (action == 4 || self.diplomacy.offers[j] != 2 || alliance_allowed)
            }

            5 => {
                resource < crate::economy::RESOURCE_COUNT as u32
                    && amount >= 1.0
                    && amount <= 999999.0
                    && crate::economy::unlocked(
                        resource as usize,
                        self.players[from as usize].epoch,
                    )
                    && crate::economy::unlocked(resource as usize, self.players[to as usize].epoch)
                    && self.resource(from, resource as usize) >= amount
                    && self.resource(to, resource as usize) + amount <= 999999.0
            }

            6 => self.diplomacy.offers[i] != 0,

            7 => {
                amount <= 1.0
                    && self.allied(from, to)
                    && self.diplomacy.sharing[i] != (amount as u8)
                    && self.diplomacy.offers[i] == 0
                    && self.tick >= self.diplomacy.cooldown[i]
            }

            _ => false,
        }
    }

    pub(crate) fn cancel_nonhostile_orders(&mut self) {
        for i in 0..self.high {
            if !self.ents[i].alive {
                continue;
            }
            let team = self.ents[i].team;

            if let Some(j) = self.get(self.ents[i].tgt) {
                if !self.hostile(team, self.ents[j].team) {
                    self.ents[i].tgt = NONE;
                }
            }

            if self.ents[i].order == O_ATTACK {
                let valid = self
                    .get(self.ents[i].otgt)
                    .map_or(false, |j| self.hostile(team, self.ents[j].team));
                if !valid {
                    self.ents[i].order = O_IDLE;
                    self.ents[i].otgt = NONE;
                    self.ents[i].tgt = NONE;
                    self.ents[i].ox = self.ents[i].x;
                    self.ents[i].oy = self.ents[i].y;
                }
            }
        }
    }

    fn declare_war(&mut self, from: u8, to: u8) {
        let i = from as usize * 4 + to as usize;
        let j = to as usize * 4 + from as usize;
        if self.allied(from, to) {
            self.diplomacy.relation[j] = NEUTRAL;
            self.diplomacy.neutral_since[j] = self.tick;
        }
        self.diplomacy.relation[i] = WAR;
        self.diplomacy.war_tick[i] = self.tick;
        self.diplomacy.offers[i] = 0;
        self.diplomacy.offers[j] = 0;
        self.diplomacy.sharing[i] = 0;
        self.diplomacy.sharing[j] = 0;
        self.diplomacy.gifts[i] = 0;
        self.diplomacy.gifts[j] = 0;
        self.dip_event(from, to, 2, 0, 0, 0);
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn clear_sole_diplomacy_offers(&mut self) {
        self.diplomacy.offers.fill(0);
        self.diplomacy.offer_vision.fill(0);
        self.diplomacy.offer_tick.fill(0);
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn force_sole_survivor_war(&mut self, active: u8) {
        self.clear_sole_diplomacy_offers();
        for from in 0..self.nplayers {
            for to in from + 1..self.nplayers {
                let pair = (1 << from) | (1 << to);
                if active & pair != pair { continue; }
                for (a, b) in [(from as u8, to as u8), (to as u8, from as u8)] {
                    if !self.hostile(a, b) { self.declare_war(a, b); }
                }
            }
        }
        self.prune_territory_offers();
        self.cancel_nonhostile_orders();
        self.refresh_display_fog();
    }

    pub fn apply_diplomacy(
        &mut self,
        from: u8,
        to: u32,
        action: u32,
        resource: u32,
        amount: f32,
    ) -> bool {
        if !self.diplomacy_valid(from, to, action, resource, amount) {
            return false;
        }

        let to = to as u8;
        let i = from as usize * 4 + to as usize;
        let j = to as usize * 4 + from as usize;

        match action {
            0 | 1 => {
                self.diplomacy.offers[i] = action as u8 + 1;
                self.diplomacy.offer_vision[i] = if action == 1 { resource as u8 } else { 0 };
                self.diplomacy.offer_tick[i] = self.tick;
                self.diplomacy.cooldown[i] = self.tick.saturating_add(100);
                self.dip_event(from, to, action as u8, 0, 0, 0);
            }

            2 => {
                self.declare_war(from, to);
            }

            3 => {
                let offer = self.diplomacy.offers[j];
                if offer <= 2 {
                    let relation = if offer == 1 { NEUTRAL } else { ALLIED };
                    self.diplomacy.relation[i] = relation;
                    self.diplomacy.relation[j] = relation;
                    self.diplomacy.neutral_since[i] = self.tick;
                    self.diplomacy.neutral_since[j] = self.tick;
                    let sharing = if offer == 2 {
                        self.diplomacy.offer_vision[j]
                    } else {
                        0
                    };
                    self.diplomacy.sharing[i] = sharing;
                    self.diplomacy.sharing[j] = sharing;
                } else {
                    let sharing = if offer == 3 { 1 } else { 0 };
                    self.diplomacy.sharing[i] = sharing;
                    self.diplomacy.sharing[j] = sharing;
                }
                self.diplomacy.offers[i] = 0;
                self.diplomacy.offers[j] = 0;
                self.dip_event(from, to, 3, offer, 0, 0);
            }

            4 => {
                self.diplomacy.offers[j] = 0;
                self.dip_event(from, to, 4, 0, 0, 0);
            }

            5 => {
                self.add_resource(from, resource as usize, -amount);
                self.add_resource(to, resource as usize, amount);
                self.diplomacy.gifts[i] = self.diplomacy.gifts[i].saturating_add(amount as u32);
                self.dip_event(from, to, 5, 0, amount as u32, resource);
            }

            6 => {
                self.diplomacy.offers[i] = 0;
                self.dip_event(from, to, 6, 0, 0, 0);
            }

            7 => {
                self.diplomacy.offers[i] = if amount == 1.0 { 3 } else { 4 };
                self.diplomacy.offer_tick[i] = self.tick;
                self.diplomacy.cooldown[i] = self.tick.saturating_add(100);
                self.dip_event(from, to, 7, 0, amount as u32, 0);
            }

            _ => return false,
        }

        self.prune_territory_offers();
        self.cancel_nonhostile_orders();
        self.refresh_display_fog();
        true
    }

    pub fn update_diplomacy(&mut self) {
        // Fixed iteration order, timestamps and team IDs; no RNG or local-seat input.

        for from in 0..self.nplayers {
            for to in 0..self.nplayers {
                if from == to {
                    continue;
                }
                let i = from * 4 + to;
                let offer = self.diplomacy.offers[i];

                if offer != 0 {
                    if !self.players[from].alive
                        || !self.players[to].alive
                        || self.tick.saturating_sub(self.diplomacy.offer_tick[i]) >= 1200
                    {
                        self.diplomacy.offers[i] = 0;
                        self.dip_event(from as u8, to as u8, 8, 0, 0, 0);
                        continue;
                    }

                    let delay = 60 + 10 * ((from + to) % 3) as u32;

                    if self.players[to].ai
                        && self.tick.saturating_sub(self.diplomacy.offer_tick[i]) >= delay
                    {
                        let common = (0..self.nplayers).any(|p| {
                            p != from
                                && p != to
                                && self.players[p].alive
                                && self.hostile(from as u8, p as u8)
                                && self.hostile(to as u8, p as u8)
                        });

                        let has_ally = |t: usize| {
                            (0..self.nplayers).any(|p| {
                                p != t && self.players[p].alive && self.allied(t as u8, p as u8)
                            })
                        };

                        let reason = if offer >= 3 {
                            if self.allied(from as u8, to as u8) {
                                0
                            } else {
                                2
                            }
                        } else if offer == 1 {
                            if self.tick < 1200 && self.diplomacy.gifts[i] < 100 {
                                1
                            } else {
                                0
                            }
                        } else if self.relation(from as u8, to as u8) != NEUTRAL
                            || self.relation(to as u8, from as u8) != NEUTRAL
                        {
                            2
                        } else if self.tick.saturating_sub(
                            self.diplomacy.neutral_since[i]
                                .max(self.diplomacy.neutral_since[to * 4 + from]),
                        ) < 600
                        {
                            3
                        } else if self.diplomacy.gifts[i] < 200 {
                            4
                        } else if !common && self.diplomacy.gifts[i] < 400 {
                            5
                        } else if has_ally(from) || has_ally(to) {
                            6
                        } else {
                            0
                        };

                        if reason == 0 {
                            self.apply_diplomacy(to as u8, from as u32, 3, 0, 0.0);
                        } else {
                            self.diplomacy.offers[i] = 0;
                            self.dip_event(to as u8, from as u8, 4, reason, 0, 0);
                        }
                    }
                }

                // A unilateral declaration can provoke a separate AI declaration. Humans

                // remain free to keep their own outgoing stance neutral.

                if self.players[to].ai
                    && self.hostile(from as u8, to as u8)
                    && self.relation(to as u8, from as u8) == NEUTRAL
                    && self.tick.saturating_sub(self.diplomacy.war_tick[i]) >= 40
                {
                    self.apply_diplomacy(to as u8, from as u32, 2, 0, 0.0);
                }
            }
        }
    }

    pub fn append_diplomacy_ui(&self, s: &mut String) {
        let me = self.local;
        let _ = write!(
            s,
            ",\"diplomacy\":{{\"local\":{},\"visibility\":{},\"players\":[",
            me, self.display_visibility()
        );

        for p in 0..self.nplayers {
            let i = me as usize * 4 + p;
            let j = p * 4 + me as usize;
            let _=write!(s,"{}{{\"team\":{},\"alive\":{},\"ai\":{},\"relation\":{},\"theirRelation\":{},\"incoming\":{},\"outgoing\":{},\"incomingAllianceVision\":{},\"outgoingAllianceVision\":{},\"sharedVision\":{},\"sharingVision\":{},\"cooldown\":{:.1}}}",if p>0{","}else{""},p,self.players[p].alive,self.players[p].ai,self.diplomacy.relation[i],self.diplomacy.relation[j],self.diplomacy.offers[j],self.diplomacy.offers[i],self.diplomacy.offer_vision[j]!=0,self.diplomacy.offer_vision[i]!=0,self.shares_to(p as u8,me),self.shares_to(me,p as u8),self.diplomacy.cooldown[i].saturating_sub(self.tick) as f32*DT);
            s.pop();
            self.append_territory_player_ui(s, me, p as u8);
            s.push('}');
        }

        s.push_str("],\"log\":[");
        // Merge only presentation records. The two bounded simulation logs
        // retain their original order, hashes and byte-for-byte save layout.
        // Same-tick ties are DIP before TER, then original insertion index.
        let mut rows = [(0u32, 0u8, 0usize); DIP_LOGS + 12];
        let mut count = 0;
        for (index, e) in self.diplomacy.log.iter().take(DIP_LOGS).enumerate() {
            if e.from == me || e.to == me {
                rows[count] = (e.tick, 0, index);
                count += 1;
            }
        }
        for (index, e) in self.territory.log.iter().take(12).enumerate() {
            if e.from == me || e.to == me {
                rows[count] = (e.tick, 1, index);
                count += 1;
            }
        }
        rows[..count].sort_unstable();
        for (number, &(_, kind, index)) in rows[..count].iter().enumerate() {
            if number > 0 {
                s.push(',');
            }
            if kind == 0 {
                Self::append_diplomacy_log(s, &self.diplomacy.log[index]);
            } else {
                Self::append_territory_log(s, &self.territory.log[index]);
            }
        }
        s.push(']');
        self.append_territory_ui(s);
        s.push('}');
    }

    fn append_diplomacy_log(s: &mut String, e: &DipEvent) {
        let verb = match e.kind {
            0 => "bietet Frieden an",
            1 => "bietet eine Allianz an",
            2 => "erklärt Krieg",
            3 => match e.reason {
                1 => "akzeptiert Frieden",
                2 => "akzeptiert die Allianz",
                3 => "akzeptiert gemeinsame Sicht",
                _ => "akzeptiert Sichttrennung",
            },
            4 => "lehnt das Angebot ab",
            5 => "sendet Tribut",
            6 => "zieht das Angebot zurück",
            7 => {
                if e.value == 1 {
                    "bietet gemeinsame Sicht an"
                } else {
                    "bietet Sichttrennung an"
                }
            }
            _ => "Angebot abgelaufen",
        };

        let _ = write!(
            s,
            "\"Spieler {} → Spieler {}: {}",
            e.from + 1,
            e.to + 1,
            verb
        );

        if e.kind == 5 {
            let _ = write!(
                s,
                " ({} {})",
                e.value,
                crate::economy::RESOURCE_NAMES[e.resource as usize]
            );
        }

        let why = match e.reason {
            1 if e.kind == 4 => "; früher Spielbeginn: erst nach 60 Sekunden oder 100 Tribut",
            2 if e.kind == 4 => "; erst gegenseitigen Frieden schließen",
            3 if e.kind == 4 => "; mindestens 30 Sekunden Frieden nötig",
            4 if e.kind == 4 => "; mindestens 200 Tribut nötig",
            5 if e.kind == 4 => "; kein gemeinsamer Gegner: alternativ 400 Tribut nötig",
            6 if e.kind == 4 => "; bereits ein Bündnispartner",
            _ => "",
        };
        s.push_str(why);
        s.push('"');
    }

    pub fn write_diplomacy(&self, b: &mut Vec<u8>) {
        for v in [
            DIP_MAGIC,
            DIP_VERSION,
            self.humans,
            self.nplayers as u32,
            self.diplomacy.visibility as u32,
        ] {
            b.extend_from_slice(&v.to_le_bytes());
        }

        for v in [
            &self.diplomacy.relation,
            &self.diplomacy.offers,
            &self.diplomacy.offer_vision,
            &self.diplomacy.sharing,
        ] {
            b.extend_from_slice(v);
        }

        for a in [
            &self.diplomacy.offer_tick,
            &self.diplomacy.cooldown,
            &self.diplomacy.gifts,
            &self.diplomacy.neutral_since,
            &self.diplomacy.war_tick,
        ] {
            for v in a {
                b.extend_from_slice(&v.to_le_bytes());
            }
        }

        b.extend_from_slice(&(self.diplomacy.log.len() as u32).to_le_bytes());

        for k in 0..DIP_LOGS {
            let e = self.diplomacy.log.get(k).copied().unwrap_or_default();
            b.extend_from_slice(&e.tick.to_le_bytes());
            b.extend_from_slice(&[e.from, e.to, e.kind, e.reason]);
            b.extend_from_slice(&e.value.to_le_bytes());
            b.extend_from_slice(&e.resource.to_le_bytes());
        }

        b.extend_from_slice(&self.team_vis);
    }

    pub fn read_diplomacy(&mut self, b: &[u8]) -> Option<()> {
        if b.len() != DIP_BYTES {
            return None;
        }
        let u = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());

        if u(0) != DIP_MAGIC
            || u(4) != DIP_VERSION
            || !(1..=2).contains(&u(8))
            || !(2..=4).contains(&u(12))
            || u(12) != (u(8) + self.n_ai).clamp(2, 4)
            || u(16) > 2
        {
            return None;
        }

        self.humans = u(8);
        self.nplayers = u(12) as usize;
        self.diplomacy.visibility = u(16) as u8;

        self.diplomacy.relation.copy_from_slice(&b[20..36]);
        self.diplomacy.offers.copy_from_slice(&b[36..52]);
        self.diplomacy.offer_vision.copy_from_slice(&b[52..68]);
        self.diplomacy.sharing.copy_from_slice(&b[68..84]);

        let d = &mut self.diplomacy;
        let mut o = 84;
        for a in [
            &mut d.offer_tick,
            &mut d.cooldown,
            &mut d.gifts,
            &mut d.neutral_since,
            &mut d.war_tick,
        ] {
            for v in a {
                *v = u(o);
                o += 4;
            }
        }

        let count = u(o) as usize;
        o += 4;
        if count > DIP_LOGS {
            return None;
        }
        d.log.clear();

        for k in 0..DIP_LOGS {
            let e = DipEvent {
                tick: u(o),
                from: b[o + 4],
                to: b[o + 5],
                kind: b[o + 6],
                reason: b[o + 7],
                value: u(o + 8),
                resource: u(o + 12),
            };
            o += 16;
            if k < count {
                if e.tick > self.tick
                    || e.from as usize >= self.nplayers
                    || e.to as usize >= self.nplayers
                    || e.kind > 8
                    || e.reason > 6
                    || e.resource as usize >= crate::economy::RESOURCE_COUNT
                {
                    return None;
                }
                d.log.push(e);
            }
        }

        for a in 0..4 {
            for c in 0..4 {
                let i = a * 4 + c;
                let j = c * 4 + a;
                if d.relation[i] > ALLIED
                    || d.offers[i] > 4
                    || d.offer_vision[i] > 1
                    || d.sharing[i] > 1
                    || d.offer_tick[i] > self.tick
                    || d.neutral_since[i] > self.tick
                    || d.war_tick[i] > self.tick
                    || (a == c
                        && (d.relation[i] != ALLIED || d.offers[i] != 0 || d.sharing[i] != 0))
                    || (d.relation[i] == ALLIED && d.relation[j] != ALLIED)
                    || (d.sharing[i] != d.sharing[j])
                    || (d.sharing[i] != 0 && d.relation[i] != ALLIED)
                    || (d.offers[i] >= 3 && d.relation[i] != ALLIED)
                {
                    return None;
                }
            }
        }

        self.team_vis.copy_from_slice(&b[o..]);
        if self
            .team_vis
            .iter()
            .any(|&v| v != 0 && v != 128 && v != 255)
        {
            return None;
        }
        self.refresh_display_fog();
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn humans() -> Game {
        Game::new(7, 0, 0, 2, 0)
    }
    fn peace(g: &mut Game) {
        assert!(g.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
    }
    fn alliance(g: &mut Game, vision: u32) {
        peace(g);
        g.tick += 100;
        assert!(g.apply_diplomacy(0, 1, 1, vision, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
    }
    #[test]
    fn directed_war_mutual_treaties_and_cancelled_orders() {
        let mut g = humans();
        peace(&mut g);
        assert_eq!(g.relation(0, 1), NEUTRAL);
        assert_eq!(g.relation(1, 0), NEUTRAL);
        assert!(g.apply_diplomacy(0, 1, 2, 0, 0.0));
        assert!(g.hostile(0, 1));
        assert!(!g.hostile(1, 0));
        let enemy = g.ents.iter().position(|e| e.alive && e.team == 1).unwrap();
        g.ents[1].order = O_ATTACK;
        g.ents[1].otgt = g.h(enemy);
        g.ents[1].tgt = g.h(enemy);
        g.tick = 100;
        assert!(g.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
        assert_eq!(g.ents[1].order, O_IDLE);
        assert_eq!(g.ents[1].tgt, NONE);
    }
    #[test]
    fn vision_is_mutual_negotiated_and_revocable() {
        let mut g = humans();
        let j = g
            .ents
            .iter()
            .position(|e| e.alive && e.team == 1 && !DEFS[e.def as usize].building)
            .unwrap();
        alliance(&mut g, 0);
        assert!(!g.visible(j));
        assert!(!g.shares_to(0, 1));
        g.tick += 100;
        assert!(g.apply_diplomacy(0, 1, 7, 0, 1.0));
        assert_eq!(g.diplomacy.offers[1], 3);
        assert!(!g.visible(j));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
        assert!(g.shares_to(0, 1) && g.shares_to(1, 0));
        assert!(g.visible(j));
        g.tick += 100;
        assert!(g.apply_diplomacy(0, 1, 7, 0, 0.0));
        assert!(g.visible(j));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
        assert!(!g.visible(j));
        assert!(g.apply_diplomacy(0, 1, 2, 0, 0.0));
        assert!(!g.shares_to(0, 1) && !g.shares_to(1, 0));
        assert_eq!(g.relation(1, 0), NEUTRAL);
    }
    #[test]
    fn explored_mode_hides_undiscovered_buildings_and_reveal_does_not_target() {
        let mut g = humans();
        let j = g
            .ents
            .iter()
            .position(|e| e.alive && e.team == 1 && DEFS[e.def as usize].building)
            .unwrap();
        let t = tile_of(g.ents[j].x, g.ents[j].y);
        assert_eq!(g.real_fog(0, t), 0);
        g.diplomacy.visibility = 1;
        g.refresh_display_fog();
        assert_eq!(g.vis[t], 128);
        assert!(!g.visible(j));
        g.diplomacy.visibility = 2;
        g.refresh_display_fog();
        assert!(g.visible(j));
        assert!(!g.can_see(0, j));
        assert!(g.footprint_explored(
            0,
            TC,
            (g.ents[j].x - 1.0) as i32,
            (g.ents[j].y - 1.0) as i32
        ));
    }
    #[test]
    fn tribute_is_atomic_and_strictly_validated() {
        let mut g = humans();
        let a = g.resource(0, 0);
        let b = g.resource(1, 0);
        for amount in [f32::NAN, f32::INFINITY, -1.0, 0.0, 1.5, 1000000.0] {
            assert!(!g.apply_diplomacy(0, 1, 5, 0, amount));
        }
        assert!(!g.apply_diplomacy(0, 256, 5, 0, 1.0));
        assert!(!g.apply_diplomacy(0, 1, 5, 9, 1.0));
        assert!(g.apply_diplomacy(0, 1, 5, 0, 50.0));
        assert_eq!(g.resource(0, 0), a - 50.0);
        assert_eq!(g.resource(1, 0), b + 50.0);
        g.players[1].res[0] = 999990.0;
        let before = g.resource(0, 0);
        assert!(!g.apply_diplomacy(0, 1, 5, 0, 50.0));
        assert_eq!(g.resource(0, 0), before);
        assert_eq!(g.resource(1, 0), 999990.0);
    }
    #[test]
    fn ai_negotiation_has_delay_price_and_neutral_period() {
        let mut g = Game::new(7, 1, 0, 1, 0);
        assert!(g.apply_diplomacy(0, 1, 0, 0, 0.0));
        for t in 1..81 {
            g.tick = t;
            g.update_diplomacy();
        }
        assert_eq!(g.relation(0, 1), WAR);
        assert_eq!(g.diplomacy.offers[1], 0);
        assert_eq!(g.diplomacy.log.last().unwrap().reason, 1);
        g.tick = 100;
        assert!(g.apply_diplomacy(0, 1, 5, 0, 100.0));
        assert!(g.apply_diplomacy(0, 1, 0, 0, 0.0));
        for t in 101..181 {
            g.tick = t;
            g.update_diplomacy();
        }
        assert_eq!(g.relation(0, 1), NEUTRAL);
        g.tick = 200;
        assert!(g.apply_diplomacy(0, 1, 1, 1, 0.0));
        for t in 201..281 {
            g.tick = t;
            g.update_diplomacy();
        }
        assert_eq!(g.diplomacy.log.last().unwrap().reason, 3);
        g.players[0].res[0] = 1000.0;
        assert!(g.apply_diplomacy(0, 1, 5, 0, 300.0));
        g.tick = 800;
        assert!(g.apply_diplomacy(0, 1, 1, 1, 0.0));
        for t in 801..881 {
            g.tick = t;
            g.update_diplomacy();
        }
        assert!(g.allied(0, 1));
        assert!(g.shares_to(0, 1) && g.shares_to(1, 0));
    }
    #[test]
    fn save_extension_roundtrip_and_legacy_migration() {
        let mut g = humans();
        alliance(&mut g, 1);
        g.diplomacy.visibility = 1;
        g.refresh_display_fog();
        g.save();
        let bytes = g.savebuf.clone();
        let loaded = Game::load(&bytes).unwrap();
        assert_eq!(g.state_hash(), loaded.state_hash());
        assert_eq!(loaded.nplayers, 2);
        assert!(loaded.shares_to(0, 1));
        #[cfg(feature="ee2-up15")]
        let bytes=crate::native_attack_clock::split_save(&bytes).unwrap().0.to_vec();
        #[cfg(feature="ee2-reference")]
        let bytes=crate::city_ground::split_save(&bytes).unwrap().0.to_vec();
        let start = bytes.len() - DIP_BYTES - crate::territory::TERR_BYTES;
        assert_eq!(
            u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap()),
            DIP_MAGIC
        );
        assert_eq!(
            bytes.len() - start,
            DIP_BYTES + crate::territory::TERR_BYTES
        );
        let mut bad = bytes.clone();
        bad[start + 21] = 3;
        assert!(Game::load(&bad).is_none());
        let old = &bytes[..start];
        #[cfg(feature="ee2-up15")]
        {assert!(Game::load(old).is_none());return;}
        let legacy = Game::load(old).unwrap();
        assert!(legacy.hostile(0, 1));
        assert_eq!(legacy.diplomacy.visibility, 0);
    }
    #[test]
    fn lockstep_is_independent_of_local_seat_and_display() {
        let mut a = Game::new(31, 1, 0, 2, 0);
        let mut b = Game::new(31, 1, 0, 2, 1);
        assert_eq!(a.state_hash(), b.state_hash());
        a.submit(crate::cmd::C_DIPLOMACY, 100.0, 0.0, 1 | (5 << 8), &[]);
        let command = a.outbox.clone();
        a.inbox = command.clone();
        b.inbox = command;
        a.net_apply(0);
        b.net_apply(0);
        for _ in 0..160 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.state_hash(), b.state_hash());
        b.diplomacy.visibility = 2;
        b.refresh_display_fog();
        for _ in 0..80 {
            a.tick();
            b.tick();
        }
        for t in 0..3 {
            for r in 0..crate::economy::RESOURCE_COUNT {
                assert_eq!(a.resource(t, r), b.resource(t, r));
            }
        }
        for (x, y) in a.ents[..a.high].iter().zip(&b.ents[..b.high]) {
            assert_eq!(x.hp, y.hp);
            assert_eq!(x.x, y.x);
            assert_eq!(x.y, y.y);
        }
    }
}

#[cfg(test)]
mod chronological_log_tests {
    use super::*;
    use crate::territory::TerritoryEvent;
    #[test]
    fn diplomacy_and_territory_ui_logs_are_chronological_stable_and_read_only() {
        let mut g = Game::new(7, 2, 0, 2, 0);
        g.tick = 40;
        g.diplomacy.log = vec![
            DipEvent {
                tick: 10,
                from: 0,
                to: 1,
                kind: 0,
                ..DipEvent::default()
            },
            DipEvent {
                tick: 30,
                from: 0,
                to: 1,
                kind: 2,
                ..DipEvent::default()
            },
            DipEvent {
                tick: 40,
                from: 0,
                to: 1,
                kind: 5,
                value: 77,
                resource: 1,
                ..DipEvent::default()
            },
            DipEvent {
                tick: 25,
                from: 2,
                to: 3,
                kind: 6,
                ..DipEvent::default()
            },
        ];
        g.territory.log = vec![
            TerritoryEvent {
                tick: 20,
                from: 1,
                to: 0,
                kind: 2,
                give: 0,
                want: 1,
                ..TerritoryEvent::default()
            },
            TerritoryEvent {
                tick: 30,
                from: 0,
                to: 1,
                kind: 0,
                give: 2,
                want: NONE,
                ..TerritoryEvent::default()
            },
        ];
        let hash = g.state_hash();
        g.save();
        let saved = g.savebuf.clone();
        let mut output = String::new();
        g.append_diplomacy_ui(&mut output);
        let peace = output.find("bietet Frieden an").unwrap();
        let swap = output.find("nimmt den Gebietstausch an").unwrap();
        let war = output.find("erklärt Krieg").unwrap();
        let gift = output.find("verschenkt ein Territorium").unwrap();
        let tribute = output.find("77 Holz").unwrap();
        assert!(peace < swap && swap < war && war < gift && gift < tribute);
        assert!(!output.contains("zieht das Angebot zurück"));
        assert_eq!(g.state_hash(), hash);
        g.save();
        assert_eq!(g.savebuf, saved);
    }
    #[test]
    fn ui_merges_all_twenty_four_bounded_records() {
        let mut g = Game::new(7, 0, 0, 2, 0);
        g.tick = 24;
        for k in 0..12 {
            g.diplomacy.log.push(DipEvent {
                tick: k * 2,
                from: 0,
                to: 1,
                kind: 2,
                ..DipEvent::default()
            });
            g.territory.log.push(TerritoryEvent {
                tick: k * 2 + 1,
                from: 0,
                to: 1,
                kind: 4,
                give: 0,
                want: 1,
                ..TerritoryEvent::default()
            });
        }
        let mut output = String::new();
        g.append_diplomacy_ui(&mut output);
        let log = output
            .split("\"log\":[")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap();
        assert_eq!(log.matches("\"Spieler ").count(), 24);
        let rows: Vec<_> = log.split("\",\"").collect();
        assert_eq!(rows.len(), 24);
        for (i, row) in rows.iter().enumerate() {
            assert!(row.contains(if i % 2 == 0 {
                "erklärt Krieg"
            } else {
                "zieht den Gebietstausch zurück"
            }));
        }
    }
}
