//! Territory gifts and negotiated swaps. Original runtime evidence: 0071ddb0
//! transfers eligible owner objects, not only buildings; 0054bdc0 is the
//! immediate tribute path. Queue handling and AI pricing are explicit adapters.
use crate::{defs::*, game::*, map::*};
use core::fmt::Write;
pub const TERR_MAGIC: u32 = 0x3152_4554; // TER1
pub const TERR_BYTES: usize = 8 + 32 * 4 + 16 * 12 + 4 + 12 * 16;
pub const TERR_NONE: u8 = 255;
// TER1 stores actor handles. These low words cannot be entity indices, and
// preserve the incumbent in the same slot while there is no completed city.
const DISPUTED_CLAIM: u32 = 0x0000_ff00;
#[cfg(feature = "ee2-native13")]
pub(crate) const DISPUTED_PEER_SAVE_FLAG: u32 = 1 << 23;
#[cfg(feature = "ee2-native13")]
pub(crate) const DISPUTED_PEER_RESUME_FLAG: u32 = 1 << 7;

fn disputed_owner(claim: u32) -> Option<u8> {
    (DISPUTED_CLAIM..DISPUTED_CLAIM + 4).contains(&claim).then(|| (claim - DISPUTED_CLAIM) as u8)
}
#[cfg(feature = "ee2-native13")]
pub(crate) const TERR_GIFT_BATCH_ACTION: u32 = 5;
#[cfg(feature = "ee2-native13")]
pub(crate) const TERR_GIFT_BATCH_MAX: usize = 32;
#[cfg(not(feature = "ee2-reference"))]
const TERR_POLICY: &str = "Eigene Gebäude und Einheiten im Gebiet wechseln mitsamt Besatzung den Besitzer. Laufende Aufträge werden gestoppt und dem bisherigen Besitzer erstattet. Erforschte Technologien werden nicht übertragen.";
#[cfg(feature = "ee2-reference")]
const TERR_POLICY: &str = "Übertragbare eigene Gebäude und Einheiten im Gebiet wechseln mitsamt Besatzung den Besitzer. Baustellen, Wunder und Kronenführer bleiben beim bisherigen Besitzer. Laufende Aufträge der übergebenen Gebäude werden erstattet. Erforschte Technologien verbleiben beim bisherigen Besitzer.";
#[derive(Clone, Copy)]
pub struct TerritoryOffer {
    pub active: bool,
    pub give: u8,
    pub want: u8,
    pub tick: u32,
    pub cooldown: u32,
}
impl Default for TerritoryOffer {
    fn default() -> Self {
        Self {
            active: false,
            give: TERR_NONE,
            want: TERR_NONE,
            tick: 0,
            cooldown: 0,
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct TerritoryEvent {
    pub tick: u32,
    pub from: u8,
    pub to: u8,
    pub kind: u8,
    pub reason: u8,
    pub give: u32,
    pub want: u32,
}
pub struct TerritoryDiplomacy {
    pub claims: [u32; 32],
    pub offers: [TerritoryOffer; 16],
    pub log: Vec<TerritoryEvent>,
}
impl Default for TerritoryDiplomacy {
    fn default() -> Self {
        Self {
            claims: [NONE; 32],
            offers: [TerritoryOffer::default(); 16],
            log: Vec::with_capacity(12),
        }
    }
}
impl Game {
    pub(crate) fn disputed_territory_rules(&self) -> bool {
        #[cfg(feature = "ee2-native13")]
        { return self.bench == 0 && (!self.net || self.network_disputed_territories); }
        #[cfg(not(feature = "ee2-native13"))]
        false
    }

    pub(crate) fn territory_is_disputed(&self, territory: usize) -> bool {
        disputed_owner(self.territory.claims[territory]).is_some()
    }

    pub(crate) fn territory_has_foreign_dispute(&self) -> bool {
        self.territory.claims.iter().filter_map(|&claim| disputed_owner(claim)).any(|owner| owner != self.local)
    }

    fn territory_seat_active(&self, team: u8) -> bool {
        let seat = usize::from(team);
        seat < self.nplayers && self.players[seat].alive
    }

    pub(crate) fn valid_territory_claim(&self, territory: usize, claim: u32) -> bool {
        claim == NONE || (territory < self.map.tseeds.len() &&
            if let Some(team) = disputed_owner(claim) {
                self.disputed_territory_rules() && self.territory_seat_active(team)
            } else {
                self.get(claim).is_some_and(|i| self.valid_territory_anchor(&self.ents[i], territory))
            })
    }

    /// New acquisition is separate from retaining an existing city anchor.
    /// BASE territory.ddf requires CityCenter1 for adjacent acquisition and
    /// CityCenter1+Fortress1 for isolated acquisition, but only CityCenter1
    /// to retain ownership. 00718cc0 separates Construction (+C) from completed
    /// counts (+8), and 007194b0 reads only completed counts. 00723ba0 builds
    /// right/down cell-edge neighbor pairs, and 007217e0 filters same-player
    /// ownership. Simultaneous rounds and lowest-ID bootstrap tie-break are adapters.
    fn territory_control_ready(e: &Ent) -> bool {
        e.alive && e.hp > 0.0 && e.built >= 1.0
    }
    #[cfg(feature = "ee2-native13")]
    fn has_colonization_mill(&self, team: u8, territory: usize) -> bool {
        self.ents[..self.high].iter().any(|e| Self::territory_control_ready(e) && e.team == team
            && matches!(e.def, MILL | WORKSHOP) && self.map.terr[tile_of(e.x, e.y)] as usize == territory)
    }
    pub(crate) fn valid_territory_anchor(&self, e: &Ent, territory: usize) -> bool {
        if e.hp <= 0.0 || self.map.terr[tile_of(e.x, e.y)] as usize != territory { return false; }
        #[cfg(feature = "ee2-native13")]
        if self.koth_territory().is_some_and(|(id, _)| id == territory) {
            return e.def == crate::koth64::HILL && e.team < self.nplayers as u8 && Self::territory_control_ready(e);
        }
        #[cfg(feature = "ee2-native13")]
        if e.def == BARRACKS && e.built >= 1.0 {
            return self.owns_completed_wonder(e.team, crate::wonders61::PARTHENON)
                && self.has_colonization_mill(e.team, territory);
        }
        e.def == TC
    }
    pub fn territory_adjacent_owned(&self, team: u8, id: usize) -> bool {
        let mask=self.map.territory_neighbors[id];
        self.towner.iter().enumerate().any(|(k,&o)|o==team && mask & (1<<k)!=0)
    }
    /// Counts include foundations; ready counts reflect the acquisition gate.
    pub fn territory_control_counts(&self, team:u8, id:usize)->[u32;4] {
        let mut counts=[0;4];
        for e in &self.ents[..self.high] {
            if !e.alive || e.hp<=0.0 || e.team!=team || self.map.terr[tile_of(e.x,e.y)] as usize!=id {continue;}
            let index=match e.def {TC=>0,FORTRESS=>1,_=>continue};
            counts[index]+=1;
            if Self::territory_control_ready(e) {counts[index+2]+=1;}
        }
        counts
    }
    pub fn territory_claim_status(&self,team:u8,id:usize)->u32 {
        if self.towner[id]==team {return 1;}
        #[cfg(feature = "ee2-native13")]
        if self.koth_territory().is_some_and(|(territory, _)| territory == id) { return 5; }
        let c=self.territory_control_counts(team,id);
        if c[0]==0 {return 0;}
        if c[2]==0 {return 2;}
        if self.towner.contains(&team) && !self.territory_adjacent_owned(team,id) {
            if c[1]==0 {return 3;}
            if c[3]==0 {return 4;}
        }
        2 // Eligible city; the next ownership reconstruction commits the claim.
    }
    /// Old layouts claimed every live city, including foundations. Preserve
    /// their acquired rights during migration rather than silently deleting land.
    pub fn seed_legacy_territory_claims(&mut self) {
        for i in 0..self.high {
            let e=self.ents[i];
            if e.alive && e.hp>0.0 && e.def==TC {
                let k=self.map.terr[tile_of(e.x,e.y)] as usize;
                if self.territory.claims[k]==NONE {self.territory.claims[k]=self.h(i);}
            }
        }
    }
    /// A live city handle anchors the claim. Transfer changes its owner, and a
    /// destroyed/reused handle cannot restore the former owner's border next tick.
    pub fn rebuild_territories(&mut self) {
        self.update_territories(false);
    }

    pub(crate) fn update_territories(&mut self, restoring: bool) {
        #[cfg(feature = "ee2-native13")]
        let hill = self.koth_territory();
        let disputed = self.disputed_territory_rules();
        let prior_claims = self.territory.claims;
        let mut deferred = [false; 32];
        let mut first = [NONE; 32];
        let mut first_by_team = [[NONE; 4]; 32];
        for i in 0..self.high {
            let e = &self.ents[i];
            if e.alive && e.hp > 0.0 && e.def == TC {
                let k = self.map.terr[tile_of(e.x, e.y)] as usize;
                if first[k] == NONE && (!cfg!(feature="ee2-reference") || Self::territory_control_ready(e)) {
                    first[k] = self.h(i);
                }
                if disputed && self.territory_seat_active(e.team) && Self::territory_control_ready(e) {
                    let candidate = &mut first_by_team[k][usize::from(e.team)];
                    if *candidate == NONE { *candidate = self.h(i); }
                }
            }
        }
        let mut owner = [TERR_NONE; 32];
        for k in 0..self.map.tseeds.len() {
            #[cfg(feature = "ee2-native13")]
            if let Some((_, index)) = hill.filter(|&(id, _)| id == k) {
                first[k] = NONE;
                let e = self.ents[index];
                self.territory.claims[k] = NONE;
                if self.valid_territory_anchor(&e, k) {
                    self.territory.claims[k] = self.h(index);
                    owner[k] = e.team;
                }
                continue;
            }
            let claim = self.territory.claims[k];
            if let Some(team) = disputed_owner(claim).filter(|_| restoring) {
                owner[k] = team;
                deferred[k] = true;
                continue;
            }
            let valid = self.get(claim).filter(|&i| self.valid_territory_anchor(&self.ents[i], k));
            let mut h = if valid.is_some() { claim } else if cfg!(feature="ee2-reference") {NONE} else { first[k] };
            #[cfg(feature = "ee2-native13")]
            if let (Some(anchor), Some(city)) = (valid, self.get(first[k])) {
                if self.ents[anchor].def == BARRACKS && self.ents[anchor].team == self.ents[city].team { h = first[k]; }
            }
            if disputed && claim != NONE {
                let incumbent = valid.map(|i| self.ents[i].team).or_else(|| disputed_owner(claim))
                    .unwrap_or(self.towner[k]);
                h = if !self.territory_seat_active(incumbent) {
                    NONE
                } else if self.get(h).is_some_and(|i| Self::territory_control_ready(&self.ents[i])) {
                    h
                } else {
                    let replacement = first_by_team[k][usize::from(incumbent)];
                    if replacement == NONE { DISPUTED_CLAIM + u32::from(incumbent) } else { replacement }
                };
            }
            self.territory.claims[k] = h;
            if let Some(i) = self.get(h) {
                owner[k] = self.ents[i].team;
            } else if let Some(incumbent) = disputed_owner(h) {
                owner[k] = incumbent;
            }
        }
        if cfg!(feature="ee2-reference") {
            // Use a fixed point of simultaneous acquisition rounds. Chains of
            // completed neighboring cities therefore do not depend on scan order.
            let mut forts=[0u8;32];
            for e in &self.ents[..self.high] {
                if e.def==FORTRESS && Self::territory_control_ready(e) {
                    forts[self.map.terr[tile_of(e.x,e.y)] as usize]|=1<<e.team;
                }
            }
            let colony = [NONE; 32];
            let colonies_by_team = [[NONE; 4]; 32];
            #[cfg(feature = "ee2-native13")]
            let (colony, colonies_by_team) = {
                let mut colony = colony;
                let mut colonies_by_team = colonies_by_team;
                let parthenon = self.completed_wonder_owners(crate::wonders61::PARTHENON);
                if parthenon != 0 {
                    let mut mills = [0u8; 32];
                    for e in &self.ents[..self.high] {
                        if matches!(e.def, MILL | WORKSHOP) && Self::territory_control_ready(e) {
                            mills[self.map.terr[tile_of(e.x, e.y)] as usize] |= 1 << e.team;
                        }
                    }
                    for (index, e) in self.ents[..self.high].iter().enumerate() {
                        if e.def != BARRACKS || !Self::territory_control_ready(e) || parthenon & (1 << e.team) == 0 { continue; }
                        let k = self.map.terr[tile_of(e.x, e.y)] as usize;
                        if mills[k] & (1 << e.team) == 0 { continue; }
                        if colony[k] == NONE { colony[k] = self.h(index); }
                        if disputed && self.territory_seat_active(e.team) {
                            let claim = &mut colonies_by_team[k][usize::from(e.team)];
                            if *claim == NONE { *claim = self.h(index); }
                        }
                    }
                }
                (colony, colonies_by_team)
            };
            if disputed {
                for k in 0..self.map.tseeds.len() {
                    if let Some(incumbent) = disputed_owner(self.territory.claims[k]).filter(|_| !deferred[k]) {
                        let claim = colonies_by_team[k][usize::from(incumbent)];
                        if claim != NONE { self.territory.claims[k] = claim; }
                    }
                }
            }
            for _ in 0..self.map.tseeds.len() {
                let mut added=[TERR_NONE;32];
                let mut anchors=[NONE;32];
                let mut changed=false;
                let mut bootstrap=[usize::MAX;4];
                for k in 0..self.map.tseeds.len() {
                    if deferred[k] || owner[k]!=TERR_NONE && !self.territory_is_disputed(k) {continue;}
                    let candidates = if disputed { &first_by_team[k][..] } else { core::slice::from_ref(&first[k]) };
                    for i in candidates.iter().filter_map(|&h| self.get(h)) {
                        let team=self.ents[i].team as usize;
                        if !owner.contains(&(team as u8)) && bootstrap[team]==usize::MAX {bootstrap[team]=k;}
                    }
                }
                for k in 0..self.map.tseeds.len() {
                    #[cfg(feature = "ee2-native13")]
                    if hill.is_some_and(|(id, _)| id == k) { continue; }
                    if deferred[k] || owner[k]!=TERR_NONE && !self.territory_is_disputed(k) {continue;}
                    let candidates = if disputed { &first_by_team[k][..] } else { core::slice::from_ref(&first[k]) };
                    for &candidate in candidates {
                        let Some(i) = self.get(candidate) else { continue; };
                        let team=self.ents[i].team;
                        let adjacent=owner.iter().enumerate().any(|(j,&o)|o==team && self.map.territory_neighbors[k]&(1<<j)!=0);
                        // 00719280 uses adjacentReq when this player owns no territory.
                        if adjacent || bootstrap[team as usize]==k || forts[k]&(1<<team)!=0 {
                            anchors[k]=candidate;added[k]=team;changed=true;break;
                        }
                    }
                    if added[k]==TERR_NONE {
                        let colony = if disputed {
                            colonies_by_team[k].iter().copied().find(|&claim| claim != NONE).unwrap_or(NONE)
                        } else {
                            colony[k]
                        };
                        let Some(i) = self.get(colony) else { continue; };
                        anchors[k]=colony;added[k]=self.ents[i].team;changed=true;
                    }
                }
                if !changed {break;}
                for k in 0..self.map.tseeds.len() {
                    if added[k] == TERR_NONE { continue; }
                    owner[k] = added[k];
                    // Native 0071D9B0 writes the new owner while preserving
                    // state 2. The next region update checks the new owner.
                    if self.territory_is_disputed(k) {
                        self.territory.claims[k] = DISPUTED_CLAIM + u32::from(added[k]);
                        deferred[k] = true;
                    } else {
                        self.territory.claims[k] = anchors[k];
                    }
                }
            }
        }
        if owner != self.towner {
            #[cfg(feature = "ee2-native13")]
            self.name_territory_owner_changes(&owner);
            self.towner = owner;
            self.terr_dirty = true;
        }
        if disputed && prior_claims.iter().enumerate().any(|(id, &claim)|
            disputed_owner(claim).is_some() != self.territory_is_disputed(id))
        {
            self.terr_dirty = true;
        }
        for t in 0..self.nplayers {
            let count = owner.iter().filter(|&&o| o as usize == t).count() as u32;
            #[cfg(feature = "ee2-native13")]
            if let Some(borders) = &mut self.diplomacy.borders {
                borders.territory_count_changed(t, self.players[t].terr, count);
            }
            self.players[t].terr = count;
        }
        #[cfg(feature = "ee2-native13")]
        self.prune_alliance_territories();
    }
    /// The original localized message explicitly voids existing alliances
    /// when the last territory is lost. Run after the entire atomic operation.
    pub fn void_landless_alliances(&mut self) {
        #[cfg(feature = "ee2-native13")]
        if self.diplomacy.fixed_teams.is_some() || self.diplomacy.borders.is_some() { return; }
        for a in 0..self.nplayers {
            if self.players[a].terr != 0 {
                continue;
            }
            for b in 0..self.nplayers {
                if a == b {
                    continue;
                }
                let i = a * 4 + b;
                let j = b * 4 + a;
                if self.allied(a as u8, b as u8) {
                    self.diplomacy.relation[i] = crate::diplomacy::NEUTRAL;
                    self.diplomacy.relation[j] = crate::diplomacy::NEUTRAL;
                    self.diplomacy.neutral_since[i] = self.tick;
                    self.diplomacy.neutral_since[j] = self.tick;
                    self.territory_event(a as u8, b as u8, 7, 0, NONE, NONE);
                }
                self.diplomacy.sharing[i] = 0;
                self.diplomacy.sharing[j] = 0;
                self.diplomacy.clear_offer(i);
                self.diplomacy.clear_offer(j);
                #[cfg(feature = "ee2-native13")]
                if let Some(timed) = &mut self.diplomacy.timed { timed.clear(a, b); }
            }
        }
        self.cancel_nonhostile_orders();
        self.refresh_display_fog();
    }
    pub fn territory_living(&self, team: u8) -> bool {
        (team as usize) < self.nplayers
            && self.players[team as usize].alive
            && self.ents[..self.high]
                .iter()
                .any(|e| e.alive && e.hp > 0.0 && e.team == team && DEFS[e.def as usize].building)
    }
    fn territory_owned(&self, team: u8, id: u32) -> bool {
        #[cfg(feature = "ee2-native13")]
        if self.koth_territory().is_some_and(|(territory, _)| territory == id as usize) { return false; }
        id < self.map.tseeds.len() as u32
            && self.towner[id as usize] == team
            && self
                .get(self.territory.claims[id as usize])
                .map_or(false, |i| {
                    let e = &self.ents[i];
                    e.team == team && self.valid_territory_anchor(e, id as usize)
                })
    }
    #[cfg(not(feature = "ee2-native13"))]
    fn territory_capitol_protected(&self, _id: u32) -> bool { false }

    fn territory_transferable(&self, team: u8, id: u32) -> bool {
        self.territory_owned(team, id) && !self.territory_capitol_protected(id)
    }
    #[cfg(feature = "ee2-native13")]
    pub(crate) fn territory_alliance_legs_valid(&self, proposer: u8, recipient: u8, terms: crate::alliance_territories::AllianceTerritories) -> bool {
        let (give, want) = (terms.give(), terms.want());
        // An alliance must retain land on both sides after the whole operation.
        self.territory_living(proposer) && self.territory_living(recipient)
            && self.players[proposer as usize].terr as usize + want.len() > give.len()
            && self.players[recipient as usize].terr as usize + give.len() > want.len()
            && give.iter().all(|&id| self.territory_transferable(proposer, u32::from(id)))
            && want.iter().all(|&id| self.territory_transferable(recipient, u32::from(id)))
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn transfer_alliance_territories(&mut self, proposer: u8, recipient: u8, terms: crate::alliance_territories::AllianceTerritories) {
        let mut objects = Vec::new();
        for (donor, new_owner, ids) in [(recipient, proposer, terms.want()), (proposer, recipient, terms.give())] {
            for &id in ids {
                for (i, actor) in self.ents[..self.high].iter().enumerate() {
                    if actor.alive && actor.hp > 0.0 && actor.team == donor && self.map.terr[tile_of(actor.x, actor.y)] == id
                        && self.territory_tribute_ready(i, usize::from(id)) {
                        objects.push((i, new_owner));
                    }
                }
            }
        }
        self.transfer_territory_objects(objects);
    }

    fn territory_offer_protected(&self, offer: TerritoryOffer) -> bool {
        self.territory_capitol_protected(u32::from(offer.give))
            || self.territory_capitol_protected(u32::from(offer.want))
    }

    pub fn territory_counts(&self, id: usize, owner: u8) -> [u32; 3] {
        self.territory_count_sets(id, owner, false).0
    }
    fn territory_count_sets(&self, id: usize, owner: u8, visible_only: bool) -> ([u32; 3], [u32; 3]) {
        let mut counts = [0; 3];
        let mut tribute = [0; 3];
        for (index, e) in self.ents[..self.high].iter().enumerate() {
            if !e.alive || e.hp <= 0.0 || e.team != owner
                || self.map.terr[tile_of(e.x, e.y)] as usize != id
                || (visible_only && !self.visible(index))
            {
                continue;
            }
            let category = if DEFS[e.def as usize].building { 0 } else { 1 };
            let crew = if visible_only { 0 } else { u32::from(e.ncargo) };
            counts[category] += 1;
            counts[2] += crew;
            if self.territory_tribute_ready(index, id) {
                tribute[category] += 1;
                tribute[2] += crew;
            }
        }
        (counts, tribute)
    }

    fn territory_tribute_ready(&self, index: usize, territory: usize) -> bool {
        let e = &self.ents[index];
        // 0071DF15 invokes 0054FED0 before changing the object's owner.
        // Imported pre-57 saves can retain a city foundation as the claim anchor.
        if cfg!(feature = "ee2-reference") && DEFS[e.def as usize].building && e.built < 1.0
            && self.territory.claims[territory] != self.h(index)
        {
            return false;
        }
        #[cfg(feature = "ee2-native13")]
        if crate::wonders61::is_wonder(e.def) || crate::leader58_defs::branch(e.def).is_some() {
            return false;
        }
        true
    }
    fn territory_offer_valid(&self, from: u8, to: u8, o: TerritoryOffer) -> bool {
        let index = from as usize * 4 + to as usize;
        o.active
            && self.territory_living(from)
            && self.territory_living(to)
            && !self.hostile(from, to)
            && !self.hostile(to, from)
            && o.give != o.want
            && self.territory_owned(from, o.give as u32)
            && self.territory_owned(to, o.want as u32)
            && !self.diplomacy_proposal_expired(o.tick, self.diplomacy.clock.territory[index])
    }
    pub fn territory_valid(&self, from: u8, to: u32, action: u32, give: u32, want: u32) -> bool {
        if self.bench > 0
            || to as usize >= self.nplayers
            || from as usize >= self.nplayers
            || to == from as u32
            || action > 4
            || !self.territory_living(from)
            || !self.territory_living(to as u8)
        {
            return false;
        }
        let to = to as u8;
        let i = from as usize * 4 + to as usize;
        let j = to as usize * 4 + from as usize;
        match action {
            0 => want == NONE && self.territory_transferable(from, give),
            1 => {
                !self.territory.offers[i].active
                    && self.diplomacy_cooldown_seconds(self.territory.offers[i].cooldown, self.diplomacy.clock.territory[i]) == 0.0
                    && !self.hostile(from, to)
                    && !self.hostile(to, from)
                    && give != want
                    && self.territory_transferable(from, give)
                    && self.territory_transferable(to, want)
            }
            2 => {
                give == NONE
                    && want == NONE
                    && self.territory_offer_valid(to, from, self.territory.offers[j])
                    && !self.territory_offer_protected(self.territory.offers[j])
            }
            3 => give == NONE && want == NONE && self.territory.offers[j].active,
            4 => give == NONE && want == NONE && self.territory.offers[i].active,
            _ => false,
        }
    }
    fn territory_event(&mut self, from: u8, to: u8, kind: u8, reason: u8, give: u32, want: u32) {
        if self.territory.log.len() == 12 {
            self.territory.log.remove(0);
        }
        self.territory.log.push(TerritoryEvent {
            tick: self.tick,
            from,
            to,
            kind,
            reason,
            give,
            want,
        });
    }
    /// Snapshot both directions before touching entities: no intermediate empty
    /// empire, duplicate object, transferred queue or acquired research bits.
    fn transfer_territories(&mut self, from: u8, to: u8, give: u8, want: u8) {
        let mut objects = Vec::new();
        for i in 0..self.high {
            let e = &self.ents[i];
            if !e.alive || e.hp <= 0.0 {
                continue;
            }
            let territory = self.map.terr[tile_of(e.x, e.y)];
            let new_team = if e.team == from && territory == give { to }
                else if want != TERR_NONE && e.team == to && territory == want { from }
                else { continue; };
            if self.territory_tribute_ready(i, usize::from(territory)) {
                objects.push((i, new_team));
            }
        }
        self.transfer_territory_objects(objects);
    }

    fn transfer_territory_objects(&mut self, objects: Vec<(usize, u8)>) {
        // Cancel and refund while each building still has its original owner.
        for &(i, _) in &objects {
            if DEFS[self.ents[i].def as usize].building {
                while self.ents[i].qn > 0 {
                    self.cancel(i);
                }
            }
        }
        for &(i, new_team) in &objects {
            #[cfg(feature = "ee2-native13")]
            { self.forget_capture_actor(i); self.mark_capture_gift(i); self.trade_interrupt(i); }
            #[cfg(feature = "ee2-native13")]
            self.retire_imperialism(i);
            #[cfg(feature="ee2-reference")]
            self.sync_city_ground_entry(i);
            let old = self.ents[i];
            let max_old = self.max_hp(old.team, old.def).max(0.01);
            let max_new = self.max_hp(new_team, old.def);
            let invalid_carry = old.def == WORKER
                && old.q[2] != R_NONE as QueueItem
                && !self.resource_unlocked(new_team, old.q[2] as u8);
            let e = &mut self.ents[i];
            e.team = new_team;
            // Match the normal return_cargo invalid-resource path.
            if invalid_carry {
                e.carry = 0.0;
                e.q[1] = 0;
                e.q[2] = 0;
            }

            e.hp = (old.hp / max_old * max_new).clamp(0.0, max_new);
            e.sel = false;
            e.order = O_IDLE;
            e.tgt = NONE;
            e.otgt = NONE;
            e.ox = e.x;
            e.oy = e.y;
            e.vx = 0.0;
            e.vy = 0.0;
            e.prev = 0;
            e.fkey = 0;
            e.fslot = u16::MAX;
            e.stuck = 0;
            e.rx = -1.0;
            e.ry = -1.0;
            if DEFS[e.def as usize].building {
                e.qt = 0.0;
                e.carry = 0.0;
            }
            #[cfg(feature="ee2-reference")]
            self.sync_city_ground_entry(i);
        }
        #[cfg(feature="ee2-native13")]
        self.air_transferred_assets(&objects);
        #[cfg(feature="ee2-native13")]
        self.reconcile_dock_owners();
        // Citizens already walking to a donated farm, ship, city or construction
        // site keep their ownership and stop their now-invalid old-owner job.
        for i in 0..self.high {
            if !self.ents[i].alive {
                continue;
            }
            let e = self.ents[i];
            if matches!(e.order, O_BUILD | O_BOARD | O_GATHER) && e.otgt != NONE {
                if let Some(j) = self.get(e.otgt) {
                    if self.ents[j].team != e.team {
                        let e = &mut self.ents[i];
                        e.order = O_IDLE;
                        e.otgt = NONE;
                        e.tgt = NONE;
                        e.ox = e.x;
                        e.oy = e.y;
                        e.vx = 0.0;
                        e.vy = 0.0;
                    }
                }
            }
        }
        self.cancel_nonhostile_orders();
        #[cfg(feature = "ee2-native13")]
        self.reconcile_capture_orders();
        self.build_grid();
        self.refresh_match_alive();
        self.release_inactive_territories();
        self.void_landless_alliances();
        self.prune_territory_offers();
        self.update_fog();
        self.terr_dirty = true;
    }
    pub fn apply_territory(
        &mut self,
        from: u8,
        to: u32,
        action: u32,
        give: u32,
        want: u32,
    ) -> bool {
        if !self.territory_valid(from, to, action, give, want) {
            return false;
        }
        let to = to as u8;
        let i = from as usize * 4 + to as usize;
        let j = to as usize * 4 + from as usize;
        match action {
            0 => {
                self.gift_territory(from, to, give);
            }
            1 => {
                self.territory.offers[i] = TerritoryOffer {
                    active: true,
                    give: give as u8,
                    want: want as u8,
                    tick: self.tick,
                    cooldown: self.tick.saturating_add(100),
                };
                self.diplomacy.clock.territory[i] = self.sim_elapsed();
                self.territory_event(from, to, 1, 0, give, want);
            }
            2 => {
                let o = self.territory.offers[j];
                self.territory.offers[j].active = false;
                self.territory.offers[i].active = false;
                self.territory_event(from, to, 2, 0, o.give as u32, o.want as u32);
                self.transfer_territories(to, from, o.give, o.want);
            }
            3 => {
                let o = self.territory.offers[j];
                self.territory.offers[j].active = false;
                self.territory_event(from, to, 3, 0, o.give as u32, o.want as u32);
            }
            4 => {
                let o = self.territory.offers[i];
                self.territory.offers[i].active = false;
                self.territory_event(from, to, 4, 0, o.give as u32, o.want as u32);
            }
            _ => return false,
        }
        true
    }

    fn gift_territory(&mut self, from: u8, to: u8, id: u32) {
        self.territory_event(from, to, 0, 0, id, NONE);
        self.transfer_territories(from, to, id as u8, TERR_NONE);
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn territory_gift_batch_valid(&self, from: u8, to: u32, ids: &[u32]) -> bool {
        if !(2..=TERR_GIFT_BATCH_MAX).contains(&ids.len()) ||
            !self.territory_valid(from, to, 0, ids[0], NONE) {
            return false;
        }
        let mut seen = 0u32;
        ids.iter().all(|&id| {
            if id >= TERR_GIFT_BATCH_MAX as u32 || !self.territory_transferable(from, id) {
                return false;
            }
            let bit = 1u32 << id;
            let new = seen & bit == 0;
            seen |= bit;
            new
        })
    }

    #[cfg(feature = "ee2-native13")]
    pub(crate) fn apply_territory_gift_batch(&mut self, from: u8, to: u32, ids: &[u32]) -> bool {
        if !self.territory_gift_batch_valid(from, to, ids) {
            return false;
        }
        // 00551273 walks one tribute list in order. Admit every ID first so a
        // stale selection or forged payload cannot transfer part of a gift.
        for &id in ids {
            self.gift_territory(from, to as u8, id);
        }
        true
    }
    pub fn prune_territory_offers(&mut self) {
        for a in 0..self.nplayers {
            for b in 0..self.nplayers {
                let i = a * 4 + b;
                let o = self.territory.offers[i];
                if !o.active {
                    continue;
                }
                let reason = if !self.territory_living(a as u8) || !self.territory_living(b as u8) {
                    3
                } else if self.hostile(a as u8, b as u8) || self.hostile(b as u8, a as u8) {
                    1
                } else if !self.territory_owned(a as u8, o.give as u32)
                    || !self.territory_owned(b as u8, o.want as u32)
                {
                    2
                } else if self.diplomacy_proposal_expired(o.tick, self.diplomacy.clock.territory[i]) {
                    4
                } else {
                    0
                };
                if reason != 0 || self.territory_offer_protected(o) {
                    self.territory.offers[i].active = false;
                    self.territory_event(
                        a as u8,
                        b as u8,
                        if reason == 4 { 5 } else { 6 },
                        reason,
                        o.give as u32,
                        o.want as u32,
                    );
                }
            }
        }
    }
    fn territory_value(&self, owner: u8, id:u8) -> f32 {
        let mut value = 100.0;
        for (index, e) in self.ents[..self.high].iter().enumerate() {
            if !e.alive || e.hp <= 0.0 || e.team != owner || self.map.terr[tile_of(e.x, e.y)] != id
                || !self.territory_tribute_ready(index, usize::from(id))
            {
                continue;
            }
            let cost: f32 = self.unit_cost(owner, e.def).iter().map(|&v| v as f32).sum();
            let health = (e.hp / self.max_hp(owner, e.def).max(0.01)).clamp(0.0, 1.0);
            value += cost.max(25.0)
                * (0.25 + 0.75 * health)
                * if DEFS[e.def as usize].building {
                    e.built.max(0.1)
                } else {
                    1.0
                };
            for k in 0..e.ncargo as usize {
                value += self
                    .unit_cost(owner, e.cargo[k])
                    .iter()
                    .map(|&v| v as f32)
                    .sum::<f32>()
                    * (e.cargo_hp[k] as f32 / 100.0).clamp(0.1, 1.0);
            }
        }
        value
    }
    #[cfg(feature = "ee2-native13")]
    pub(crate) fn alliance_territory_value_sufficient(&self, proposer: u8, recipient: u8, terms: crate::alliance_territories::AllianceTerritories) -> bool {
        let offered: f32 = terms.give().iter().map(|&id| self.territory_value(proposer, id)).sum();
        let requested: f32 = terms.want().iter().map(|&id| self.territory_value(recipient, id)).sum();
        offered + 0.001 >= requested * 1.1
    }
    pub fn update_territory_diplomacy(&mut self) {
        self.prune_territory_offers();
        for a in 0..self.nplayers {
            for b in 0..self.nplayers {
                let i = a * 4 + b;
                let o = self.territory.offers[i];
                if o.active && self.players[b].ai && self.diplomacy_delay_reached(o.tick, self.diplomacy.clock.territory[i], 80) {
                    let offered = self.territory_value(a as u8, o.give);
                    let requested = self.territory_value(b as u8, o.want);
                    let margin = if self.allied(a as u8, b as u8) {
                        1.0
                    } else {
                        1.1
                    };
                    if offered + 0.001 >= requested * margin {
                        self.apply_territory(b as u8, a as u32, 2, NONE, NONE);
                    } else {
                        self.territory.offers[i].active = false;
                        self.territory_event(b as u8, a as u8, 3, 5, o.give as u32, o.want as u32);
                    }
                }
            }
        }
    }
    pub fn append_territory_player_ui(&self, s: &mut String, local: u8, other: u8) {
        for (key, i) in [
            ("territoryIncoming", other as usize * 4 + local as usize),
            ("territoryOutgoing", local as usize * 4 + other as usize),
        ] {
            let o = self.territory.offers[i];
            let _ = write!(s, ",\"{}\":", key);
            if o.active {
                let expires = self.diplomacy_proposal_seconds(o.tick, self.diplomacy.clock.territory[i]);
                let _ = write!(
                    s,
                    "{{\"give\":{},\"want\":{},\"expires\":{:.1}{}}}",
                    o.give,
                    o.want,
                    expires,
                    if self.territory_offer_protected(o) { ",\"capitolProtected\":true" } else { "" }
                );
            } else {
                s.push_str("null");
            }
        }
        let row = local as usize * 4 + other as usize;
        let _ = write!(
            s,
            ",\"territoryCooldown\":{:.1}",
            self.diplomacy_cooldown_seconds(self.territory.offers[row].cooldown, self.diplomacy.clock.territory[row])
        );
    }
    pub fn append_territory_ui(&self, s: &mut String) {
        let mut known = [false; 32];
        let mut visible = [false; 32];
        let mut disclosed = [false; 32];
        for (i, &v) in self.vis.iter().enumerate() {
            if v > 0 {
                let territory = self.map.terr[i] as usize;
                known[territory] = true;
                visible[territory] |= v == 255;
            }
        }
        // An explicit offer discloses the two named parcels to its recipient.
        for p in 0..self.nplayers {
            let o = self.territory.offers[p * 4 + self.local as usize];
            if o.active {
                known[o.give as usize] = true;
                known[o.want as usize] = true;
                disclosed[o.give as usize] = true;
            }
        }
        let _ = write!(
            s,
            ",\"territoryPolicy\":\"{}\",\"territories\":[",
            TERR_POLICY
        );
        for id in 0..self.map.tseeds.len() {
            let actual = self.towner[id];
            let seen = known[id] || actual == self.local;
            let disputed = (actual == self.local || visible[id]) && self.territory_is_disputed(id);
            let owner = if seen { actual } else { TERR_NONE };
            let complete =
                seen && (actual == self.local || disclosed[id] || self.display_visibility() == 2);
            let (counts, tribute) = if seen { self.territory_count_sets(id, actual, !complete) } else { ([0; 3], [0; 3]) };
            let transferable = seen
                && actual != TERR_NONE
                && self.territory_living(actual)
                && self.territory_transferable(actual, id as u32);
            let capitol = seen && self.territory_capitol_protected(id as u32);
            #[cfg(feature = "ee2-native13")]
            let hill = self.koth_territory().is_some_and(|(territory, _)| territory == id);
            #[cfg(not(feature = "ee2-native13"))]
            let hill = false;
            let reason = if !seen {
                "Gebiet noch nicht erkundet"
            } else if hill {
                "Der Hügel kann nur erobert werden"
            } else if capitol {
                "Ein Kapitolgebiet kann nur erobert werden"
            } else if actual == TERR_NONE {
                "Kein Besitzer"
            } else if disputed {
                "Umkämpftes Gebiet"
            } else if !transferable {
                "Kein lebender Eigentümer mit Stadtzentrum"
            } else {
                ""
            };
            let (x, y) = self.map.tseeds[id];
            let control=if seen {self.territory_control_counts(self.local,id)}else{[0;4]};
            let claim_status=if seen {self.territory_claim_status(self.local,id)}else{0};
            let adjacent=seen&&self.territory_adjacent_owned(self.local,id);
            let _=write!(s,"{}{{\"id\":{},\"name\":\"Territorium {}\",\"owner\":{},\"x\":{:.2},\"y\":{:.2},\"known\":{},\"canGive\":{},\"transferable\":{},\"reason\":\"{}\",\"buildings\":{},\"units\":{},\"garrison\":{},\"tributeBuildings\":{},\"tributeUnits\":{},\"tributeGarrison\":{},\"countsComplete\":{},\"lastTerritory\":{},\"claimStatus\":{},\"adjacent\":{},\"townCenters\":{},\"fortresses\":{},\"eligibleTownCenters\":{},\"eligibleFortresses\":{}{}{}}}",if id>0{","}else{""},id,id+1,owner,x,y,seen,transferable&&actual==self.local,transferable,reason,counts[0],counts[1],counts[2],tribute[0],tribute[1],tribute[2],complete,seen&&actual!=TERR_NONE&&self.players.get(actual as usize).map_or(false,|p|p.terr<=1),claim_status,adjacent,control[0],control[1],control[2],control[3],if capitol { ",\"capitolProtected\":true" } else { "" },if disputed { ",\"disputed\":true" } else { "" });
        }
        s.push(']');
    }
    pub(crate) fn append_territory_log(s: &mut String, e: &TerritoryEvent) {
        if e.kind == 7 {
            let _ = write!(
                s,
                "\"Spieler {}: Bündnis mit Spieler {} beendet, letztes Territorium verloren\"",
                e.from + 1,
                e.to + 1
            );
            return;
        }
        let verb = match e.kind {
            0 => "verschenkt ein Territorium",
            1 => "bietet einen Gebietstausch an",
            2 => "nimmt den Gebietstausch an",
            3 => "lehnt den Gebietstausch ab",
            4 => "zieht den Gebietstausch zurück",
            5 => "Gebietstausch abgelaufen",
            7 => "letztes Territorium verloren: bestehende Allianzen aufgehoben",
            _ => "Gebietstausch ungültig geworden",
        };
        let _ = write!(
            s,
            "\"Spieler {} → Spieler {}: {} (Territorium {}",
            e.from + 1,
            e.to + 1,
            verb,
            if e.give == NONE { 0 } else { e.give + 1 }
        );
        if e.want != NONE {
            let _ = write!(s, " gegen {}", e.want + 1);
        }
        s.push(')');
        let reason = match e.reason {
            1 => "; Krieg",
            2 => "; Eigentum verloren",
            3 => "; Spieler ausgeschieden",
            4 => "; Zeit abgelaufen",
            5 => "; KI verlangt mindestens gleichwertige Gebiete, ohne Allianz 10 Prozent mehr",
            _ => "",
        };
        s.push_str(reason);
        s.push('"');
    }
    pub fn write_territory(&self, b: &mut Vec<u8>) {
        b.extend_from_slice(&TERR_MAGIC.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        for &h in &self.territory.claims {
            b.extend_from_slice(&h.to_le_bytes());
        }
        for o in self.territory.offers {
            b.extend_from_slice(&[o.active as u8, o.give, o.want, 0]);
            b.extend_from_slice(&o.tick.to_le_bytes());
            b.extend_from_slice(&o.cooldown.to_le_bytes());
        }
        b.extend_from_slice(&(self.territory.log.len() as u32).to_le_bytes());
        for k in 0..12 {
            let e = self.territory.log.get(k).copied().unwrap_or_default();
            b.extend_from_slice(&e.tick.to_le_bytes());
            b.extend_from_slice(&[e.from, e.to, e.kind, e.reason]);
            b.extend_from_slice(&e.give.to_le_bytes());
            b.extend_from_slice(&e.want.to_le_bytes());
        }
    }
    pub fn read_territory(&mut self, b: &[u8]) -> Option<()> {
        if b.len() != TERR_BYTES {
            return None;
        }
        let u = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        if u(0) != TERR_MAGIC || u(4) != 1 {
            return None;
        }
        let mut pos = 8;
        for h in &mut self.territory.claims {
            *h = u(pos);
            pos += 4;
        }
        for o in &mut self.territory.offers {
            if b[pos] > 1 || b[pos + 3] != 0 {
                return None;
            }
            *o = TerritoryOffer {
                active: b[pos] == 1,
                give: b[pos + 1],
                want: b[pos + 2],
                tick: u(pos + 4),
                cooldown: u(pos + 8),
            };
            pos += 12;
            if (o.give != TERR_NONE && o.give as usize >= self.map.tseeds.len())
                || (o.want != TERR_NONE && o.want as usize >= self.map.tseeds.len())
                || o.tick > self.tick
                || o.cooldown > self.tick.saturating_add(100)
                || (o.active
                    && (o.give as usize >= self.map.tseeds.len()
                        || o.want as usize >= self.map.tseeds.len()
                        || o.give == o.want))
            {
                return None;
            }
        }
        let count = u(pos) as usize;
        pos += 4;
        if count > 12 {
            return None;
        }
        self.territory.log.clear();
        for k in 0..12 {
            let e = TerritoryEvent {
                tick: u(pos),
                from: b[pos + 4],
                to: b[pos + 5],
                kind: b[pos + 6],
                reason: b[pos + 7],
                give: u(pos + 8),
                want: u(pos + 12),
            };
            pos += 16;
            if k < count {
                if e.tick > self.tick
                    || e.from as usize >= self.nplayers
                    || e.to as usize >= self.nplayers
                    || e.from == e.to
                    || e.kind > 7
                    || e.reason > 5
                    || (e.give >= self.map.tseeds.len() as u32 && !(e.kind == 7 && e.give == NONE))
                    || (e.want != NONE && e.want >= self.map.tseeds.len() as u32)
                {
                    return None;
                }
                self.territory.log.push(e);
            }
        }
        Some(())
    }
    pub fn validate_territory_state(&self) -> bool {
        for (territory, &claim) in self.territory.claims.iter().enumerate() {
            if !self.valid_territory_claim(territory, claim) { return false; }
        }
        for a in 0..4 {
            for b in 0..4 {
                let o = self.territory.offers[a * 4 + b];
                if o.active
                    && (a == b
                        || a >= self.nplayers
                        || b >= self.nplayers
                        || !self.territory_offer_valid(a as u8, b as u8, o))
                {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_clock_territory_offer_expires_in_game_seconds_after_restore() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        for player in &mut game.players[..game.nplayers] { player.ai = false; }
        assert!(game.tick_with_source_delta(0.125));
        assert!(game.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(game.apply_diplomacy(1, 0, 3, 0, 0.0));
        let region = |seat| (0..32).find(|&id| game.territory_owned(seat, id)).unwrap();
        let (give, want) = (region(0), region(1));
        assert!(game.apply_territory(0, 1, 1, give, want));
        for _ in 0..30 { assert!(game.tick_with_source_delta(0.125)); }
        let mut ui = String::new();
        game.append_territory_player_ui(&mut ui, 0, 1);
        assert!(ui.contains("\"expires\":296.2"));
        assert!(ui.contains("\"territoryCooldown\":1.2"));
        game.save();
        assert_eq!(game.savebuf.get(16..20), Some(b"DCT1".as_slice()));
        let mut copy = Game::load(&game.savebuf).expect("source territory stamps load");
        copy.save();
        assert_eq!(game.savebuf, copy.savebuf);
        let mut bad = game.savebuf.clone();
        bad[24..28].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(Game::load(&bad).is_none());
        while game.sim_elapsed() < 300.125 {
            assert!(game.territory.offers[1].active);
            assert!(game.tick_with_source_delta(0.125));
            assert!(copy.tick_with_source_delta(0.125));
            assert_eq!(game.state_hash(), copy.state_hash());
        }
        assert_eq!(game.tick, 2401);
        assert!(!game.territory.offers[1].active);
        assert_eq!(game.territory.log.last().unwrap().reason, 4);
    }
    fn humans() -> Game {
        Game::new(7, 0, 0, 2, 0)
    }
    fn region(g: &Game, team: u8) -> u8 {
        g.ents[..g.high]
            .iter()
            .find(|e| e.alive && e.team == team && e.def == TC)
            .map(|e| g.map.terr[tile_of(e.x, e.y)])
            .unwrap()
    }
    fn keep_city(g: &mut Game, team: u8) -> usize {
        let (x, y) = g.map.starts[3];
        let i = g.spawn(TC, team, x as f32, y as f32);
        g.build_grid();
        g.update_fog();
        i
    }
    fn peace(g: &mut Game) {
        assert!(g.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
    }

    #[cfg(feature = "ee2-native13")]
    #[test]
    fn tribute_batch_preserves_input_order_and_standard_archive() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        let home = u32::from(region(&game, 0));
        let enemy = u32::from(region(&game, 1));
        let (id, &(x, y)) = game.map.tseeds.iter().enumerate()
            .find(|&(id, _)| game.towner[id] == TERR_NONE).unwrap();
        game.spawn(TC, 0, x, y);
        game.spawn(FORTRESS, 0, x + 2.0, y);
        game.build_grid();
        let expansion = id as u32;
        let before = game.state_hash();
        for invalid in [[home, home], [expansion, enemy], [expansion, u32::MAX]] {
            assert!(!game.apply_territory_gift_batch(0, 1, &invalid));
            assert_eq!(game.state_hash(), before);
        }
        game.save();
        let mut sequential = Game::load(&game.savebuf).expect("unchanged source save format");
        let order = [expansion, home];
        let action = (TERR_GIFT_BATCH_ACTION << 8 | 1) as i32;
        game.apply(0, crate::cmd::C_TERRITORY, 0.0, -1.0, action, &order);
        assert_eq!(game.towner[home as usize], 0);
        game.apply(0, crate::cmd::C_TERRITORY, -1.0, -1.0, action, &order);
        for id in order {
            assert!(sequential.apply_territory(0, 1, 0, id, NONE));
            assert_eq!(game.towner[id as usize], 1);
        }
        assert_eq!(game.territory.log.iter().map(|event| event.give).collect::<Vec<_>>(), order);
        sequential.civ_setup_open = game.civ_setup_open;
        game.save();
        sequential.save();
        assert_eq!(game.savebuf, sequential.savebuf);
    }

    #[cfg(feature = "ee2-native13")]
    #[test]
    fn registered_capitol_territory_is_excluded_from_both_tribute_legs() {
        for mode in [5, 6] {
            let mut game = Game::new(7, 1, 0, 1, 0);
            assert!(game.configure_match_capitols(0xee81, 500, 2, 0, 0, mode, 2, 0));
            game.players[1].ai = false;
            let home = [region(&game, 0), region(&game, 1)].map(u32::from);
            let mut expansion = [0; 2];
            for seat in 0..2 {
                let (id, &(x, y)) = game.map.tseeds.iter().enumerate()
                    .find(|&(id, _)| game.towner[id] == TERR_NONE).unwrap();
                game.spawn(TC, seat, x, y);
                game.spawn(FORTRESS, seat, x + 2.0, y);
                game.build_grid();
                assert!(game.territory_transferable(seat, id as u32));
                expansion[usize::from(seat)] = id as u32;
            }
            peace(&mut game);
            let mut ui = String::new();
            game.append_territory_ui(&mut ui);
            assert!(ui.contains("\"capitolProtected\":true"));
            let before = game.state_hash();
            assert!(!game.apply_territory(0, 1, 0, home[0], NONE));
            assert!(!game.apply_territory_gift_batch(0, 1, &[expansion[0], home[0]]));
            assert!(!game.apply_territory_gift_batch(0, 1, &[home[0], expansion[0]]));
            assert!(!game.apply_territory(0, 1, 1, home[0], expansion[1]));
            assert!(!game.apply_territory(0, 1, 1, expansion[0], home[1]));
            assert_eq!(game.state_hash(), before);

            // An offer produced by the older rules can still be restored byte for byte.
            game.territory.offers[1] = TerritoryOffer {
                active: true, give: expansion[0] as u8, want: home[1] as u8,
                tick: game.tick, cooldown: game.tick.saturating_add(100),
            };
            game.save();
            let mut restored = Game::load(&game.savebuf).expect("older active Capitol offer");
            restored.save();
            assert_eq!(game.savebuf, restored.savebuf);
            assert!(!restored.apply_territory(1, 0, 2, NONE, NONE));
            restored.prune_territory_offers();
            assert!(!restored.territory.offers[1].active);
            assert_eq!(restored.towner[home[1] as usize], 1);

            game.territory.offers[1] = TerritoryOffer::default();
            assert!(game.apply_territory(0, 1, 1, expansion[0], expansion[1]));
            assert!(game.apply_territory(1, 0, 2, NONE, NONE));
            assert_eq!(game.towner[expansion[0] as usize], 1);
            assert_eq!(game.towner[expansion[1] as usize], 0);
            assert_eq!(game.capitol_status(3, 0), 0);
            assert_eq!(game.capitol_status(3, 1), 1);
        }
    }

    #[test]
    fn gift_is_permanent_transfers_only_owner_objects_and_cancels_paid_queues() {
        let mut g = humans();
        let giver = region(&g, 0);
        let keep = keep_city(&mut g, 0);
        let city = g.get(g.territory.claims[giver as usize]).unwrap();
        let old_city = g.ents[city];
        let foreign = g.spawn(WORKER, 1, old_city.x + 6.0, old_city.y);
        let builder = g.spawn(WORKER, 0, g.ents[keep].x, g.ents[keep].y);
        g.ents[builder].order = O_BUILD;
        g.ents[builder].otgt = g.h(city);
        g.ents[city].ncargo = 2;
        g.ents[city].cargo[..2].fill(WORKER);
        g.ents[city].cargo_hp[0] = 50;
        g.ents[city].cargo_hp[1] = 95;
        let money = g.resource(0, 0);
        assert!(g.train(city, WORKER as QueueItem));
        let count = g.ents[..g.high].iter().filter(|e| e.alive).count();
        let handle = g.h(city);
        let recipient_money = g.resource(1, 0);
        assert!(g.apply_territory(0, 1, 0, giver as u32, NONE));
        assert_eq!(g.towner[giver as usize], 1);
        assert_eq!(g.h(city), handle);
        assert_eq!(g.ents[city].team, 1);
        assert_eq!(g.ents[city].qn, 0);
        assert_eq!(g.resource(0, 0), money);
        assert_eq!(g.resource(1, 0), recipient_money);
        assert_eq!(g.ents[city].ncargo, 2);
        assert_eq!(g.ents[city].cargo_hp[..2], [50, 95]);
        assert_eq!(g.ents[foreign].team, 1);
        assert_eq!(g.ents[builder].team, 0);
        assert_eq!(g.ents[builder].order, O_IDLE);
        assert_eq!(g.ents[keep].team, 0);
        assert_eq!(g.ents[..g.high].iter().filter(|e| e.alive).count(), count);
        for _ in 0..20 {
            g.build_grid();
        }
        assert_eq!(g.towner[giver as usize], 1);
        assert!(g.players[0].alive);
        g.save();
        let loaded = Game::load(&g.savebuf).unwrap();
        assert_eq!(g.state_hash(), loaded.state_hash());
        assert_eq!(loaded.towner[giver as usize], 1);
    }

    #[cfg(feature = "ee2-native13")]
    #[test]
    fn tribute_leaves_foundations_wonders_and_leaders_with_their_owner() {
        let mut game = Game::new(7, 1, 0, 1, 0);
        game.players[1].ai = false;
        assert!(game.configure_crowns(false));
        assert!(game.configure_priest_skirmish());
        assert!(game.configure_crown_leaders());
        let a = region(&game, 0);
        let b = region(&game, 1);
        let [mut excluded, mut houses] = [Vec::new(), Vec::new()];
        for (seat, region) in [(0, a), (1, b)] {
            let city = game.ents[game.get(game.territory.claims[usize::from(region)]).unwrap()];
            let site = game.spawn(HOUSE, seat, city.x + 4.0, city.y);
            game.ents[site].built = 0.4;
            let wonder = game.spawn_oriented(crate::wonders61::PARTHENON, seat, city.x - 6.0, city.y, 0.0);
            let leader = game.spawn(crate::leader58_defs::FIRST, seat, city.x, city.y + 4.0);
            let house = game.spawn(HOUSE, seat, city.x, city.y - 6.0);
            for index in [site, wonder, leader, house] {
                let e = game.ents[index];
                assert_eq!(game.map.terr[tile_of(e.x, e.y)], region);
            }
            excluded.extend([site, wonder, leader].map(|index| (game.h(index), seat)));
            houses.push((game.h(house), seat));
        }
        game.build_grid();
        let (counts, tribute) = game.territory_count_sets(usize::from(a), 0, false);
        assert_eq!(counts[0] - tribute[0], 2);
        assert_eq!(counts[1] - tribute[1], 1);
        peace(&mut game);
        assert!(game.apply_territory(0, 1, 1, u32::from(a), u32::from(b)));
        assert!(game.apply_territory(1, 0, 2, NONE, NONE));
        assert_eq!((game.towner[usize::from(a)], game.towner[usize::from(b)]), (1, 0));
        for (handle, old_owner) in excluded {
            assert_eq!(game.ents[game.get(handle).unwrap()].team, old_owner);
        }
        for (handle, old_owner) in houses {
            assert_eq!(game.ents[game.get(handle).unwrap()].team, 1 - old_owner);
        }
        game.save();
        let restored = Game::load(&game.savebuf).expect("mixed-owner territories remain valid");
        assert_eq!(restored.state_hash(), game.state_hash());
    }
    #[test]
    fn swap_requires_acceptance_is_atomic_and_keeps_alliance() {
        let mut g = humans();
        peace(&mut g);
        g.tick = 100;
        assert!(g.apply_diplomacy(0, 1, 1, 1, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
        let a = region(&g, 0);
        let b = region(&g, 1);
        let n = g.sim_units;
        assert!(g.apply_territory(0, 1, 1, a as u32, b as u32));
        assert_eq!(g.towner[a as usize], 0);
        assert_eq!(g.towner[b as usize], 1);
        assert!(g.territory.offers[1].active);
        assert!(g.apply_territory(1, 0, 2, NONE, NONE));
        assert_eq!(g.towner[a as usize], 1);
        assert_eq!(g.towner[b as usize], 0);
        assert_eq!(g.players[0].terr, 1);
        assert_eq!(g.players[1].terr, 1);
        assert!(g.players[0].alive && g.players[1].alive);
        assert!(g.allied(0, 1));
        assert!(g.shares_to(0, 1) && g.shares_to(1, 0));
        assert_eq!(g.sim_units, n);
        assert!(!g.territory.offers[1].active);
    }
    #[test]
    fn last_territory_gift_is_allowed_and_voids_existing_alliances() {
        let mut g = humans();
        peace(&mut g);
        g.tick = 100;
        assert!(g.apply_diplomacy(0, 1, 1, 1, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
        let a = region(&g, 0);
        assert!(g.apply_territory(0, 1, 0, a as u32, NONE));
        assert_eq!(g.players[0].terr, 0);
        assert!(!g.players[0].alive);
        assert!(!g.allied(0, 1));
        assert_eq!(g.relation(0, 1), crate::diplomacy::NEUTRAL);
        assert!(!g.shares_to(0, 1) && !g.shares_to(1, 0));
        g.save();
        assert!(Game::load(&g.savebuf).is_some());
    }
    #[test]
    fn stale_war_expired_rejected_and_withdrawn_offers_never_steal() {
        let mut g = humans();
        peace(&mut g);
        let a = region(&g, 0);
        let b = region(&g, 1);
        keep_city(&mut g, 0);
        assert!(g.apply_territory(0, 1, 1, a as u32, b as u32));
        assert!(g.apply_territory(0, 1, 0, a as u32, NONE));
        assert!(!g.territory.offers[1].active);
        assert!(!g.apply_territory(1, 0, 2, NONE, NONE));
        assert_eq!(g.towner[b as usize], 1);
        let c = region(&g, 0);
        g.tick = 100;
        assert!(g.apply_territory(0, 1, 1, c as u32, b as u32));
        assert!(g.apply_territory(1, 0, 3, NONE, NONE));
        assert_eq!(g.towner[c as usize], 0);
        g.tick = 200;
        assert!(g.apply_territory(0, 1, 1, c as u32, b as u32));
        assert!(g.apply_territory(0, 1, 4, NONE, NONE));
        g.tick = 300;
        assert!(g.apply_territory(0, 1, 1, c as u32, b as u32));
        assert!(g.apply_diplomacy(0, 1, 2, 0, 0.0));
        assert!(!g.territory.offers[1].active);
        assert!(!g.apply_territory(1, 0, 2, NONE, NONE));
        g.tick = 400;
        assert!(g.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
        assert!(g.apply_territory(0, 1, 1, c as u32, b as u32));
        g.tick = 1600;
        g.update_territory_diplomacy();
        assert!(!g.territory.offers[1].active);
        assert_eq!(g.towner[c as usize], 0);
    }
    #[test]
    fn invalid_arguments_and_forged_wire_have_no_effect() {
        let mut g = humans();
        let a = region(&g, 0);
        let b = region(&g, 1);
        let h = g.state_hash();
        for (to, action, give, want) in [
            (0, 0, a as u32, NONE),
            (256, 0, a as u32, NONE),
            (1, 5, a as u32, NONE),
            (1, 0, b as u32, NONE),
            (1, 0, a as u32, 0),
            (1, 1, a as u32, a as u32),
            (1, 1, a as u32, b as u32),
            (1, 0, 32, NONE),
        ] {
            assert!(!g.apply_territory(0, to, action, give, want));
            assert_eq!(g.state_hash(), h);
        }
        for v in [f32::NAN, f32::INFINITY, -2.0, 0.5, 32.0, 999999.0] {
            g.apply(0, crate::cmd::C_TERRITORY, v, -1.0, 1, &[]);
            assert_eq!(g.state_hash(), h);
        }
        g.apply(0, crate::cmd::C_TERRITORY, a as f32, -1.0, 1, &[g.h(0)]);
        assert_eq!(g.state_hash(), h);
    }
    #[test]
    fn saved_dip1_and_ter1_pending_offers_roundtrip() {
        let mut g = humans();
        peace(&mut g);
        let a = region(&g, 0);
        let b = region(&g, 1);
        assert!(g.apply_territory(0, 1, 1, a as u32, b as u32));
        g.save();
        let bytes = g.savebuf.clone();
        let loaded = Game::load(&bytes).unwrap();
        assert_eq!(loaded.state_hash(), g.state_hash());
        assert!(loaded.territory.offers[1].active);
        #[cfg(feature="ee2-up15")]
        let bytes=crate::native_attack_clock::split_save(&bytes).unwrap().0.to_vec();
        #[cfg(feature="ee2-reference")]
        let bytes=crate::city_ground::split_save(&bytes).unwrap().0.to_vec();
        let ter = bytes.len() - TERR_BYTES;
        let dip = ter - crate::diplomacy::DIP_BYTES;
        #[cfg(feature="ee2-up15")]
        {assert!(Game::load(&bytes[..ter]).is_none());return;}
        let legacy = Game::load(&bytes[..ter]).unwrap();
        assert_eq!(legacy.relation(0, 1), crate::diplomacy::NEUTRAL);
        assert!(!legacy.territory.offers[1].active);
        assert_eq!(legacy.towner, g.towner);
        assert_eq!(
            &bytes[dip..dip + 4],
            &crate::diplomacy::DIP_MAGIC.to_le_bytes()
        );
        assert_eq!(&bytes[ter..ter + 4], &TERR_MAGIC.to_le_bytes());
        let mut bad = bytes.clone();
        bad[ter + 136] = 2;
        assert!(Game::load(&bad).is_none());
        let mut bad = bytes;
        bad[ter + 8..ter + 12].copy_from_slice(&123456u32.to_le_bytes());
        assert!(Game::load(&bad).is_none());
    }
    #[test]
    fn city_loss_releases_anchor_and_invalidates_pending_offer() {
        let mut g = humans();
        peace(&mut g);
        let a = region(&g, 0);
        let b = region(&g, 1);
        keep_city(&mut g, 0);
        assert!(g.apply_territory(0, 1, 1, a as u32, b as u32));
        let i = g.get(g.territory.claims[a as usize]).unwrap();
        g.ents[i].hp = 0.0;
        g.tick();
        assert_eq!(g.towner[a as usize], TERR_NONE);
        assert!(!g.territory.offers[1].active);
        assert!(!g.apply_territory(1, 0, 2, NONE, NONE));
    }
    #[test]
    fn ai_swap_has_real_delay_and_requires_fair_value() {
        let mut g = Game::new(7, 1, 0, 1, 0);
        g.tick = 1200;
        assert!(g.apply_diplomacy(0, 1, 0, 0, 0.0));
        assert!(g.apply_diplomacy(1, 0, 3, 0, 0.0));
        let a = region(&g, 0);
        let b = region(&g, 1);
        assert!(g.apply_territory(0, 1, 1, a as u32, b as u32));
        g.tick = 1279;
        g.update_territory_diplomacy();
        assert!(g.territory.offers[1].active);
        g.tick = 1280;
        g.update_territory_diplomacy();
        assert!(!g.territory.offers[1].active);
        assert_eq!(g.territory.log.last().unwrap().reason, 5);
        assert_eq!(g.towner[a as usize], 0);
        let c = g.ents[g.get(g.territory.claims[a as usize]).unwrap()];
        for k in 0..10 {
            g.spawn(
                HOUSE,
                0,
                c.x + 5.0 + (k % 3) as f32 * 2.0,
                c.y + 5.0 + (k / 3) as f32 * 2.0,
            );
        }
        g.build_grid();
        g.tick = 1300;
        assert!(g.apply_territory(0, 1, 1, a as u32, b as u32));
        g.tick = 1380;
        g.update_territory_diplomacy();
        assert_eq!(g.towner[a as usize], 1);
        assert_eq!(g.towner[b as usize], 0);
    }
    #[test]
    fn territory_wire_is_deterministic_for_both_local_seats() {
        let mut a = Game::new(31, 0, 0, 2, 0);
        let mut b = Game::new(31, 0, 0, 2, 1);
        let r = region(&a, 0);
        a.submit(crate::cmd::C_TERRITORY, r as f32, -1.0, 1, &[]);
        let bytes = a.outbox.clone();
        a.inbox = bytes.clone();
        b.inbox = bytes;
        a.net_apply(0);
        b.net_apply(0);
        assert_eq!(a.state_hash(), b.state_hash());
        for _ in 0..80 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.state_hash(), b.state_hash());
    }
    #[test]
    fn unknown_region_json_does_not_leak_owner_last_territory_or_counts() {
        let mut g = humans();
        let id = region(&g, 1);
        let mut s = String::new();
        g.append_territory_ui(&mut s);
        let row = s
            .split(&format!("\"id\":{},", id))
            .nth(1)
            .unwrap()
            .split('}')
            .next()
            .unwrap();
        assert!(row.contains("\"owner\":255"));
        assert!(row.contains("\"known\":false"));
        assert!(row.contains("\"lastTerritory\":false"));
        assert!(row.contains("\"buildings\":0"));
        assert!(row.contains("\"units\":0"));
        assert!(row.contains("\"garrison\":0"));
        assert!(row.contains("\"countsComplete\":false"));
        g.diplomacy.visibility = 2;
        g.refresh_display_fog();
        let mut s = String::new();
        g.append_territory_ui(&mut s);
        let row = s
            .split(&format!("\"id\":{},", id))
            .nth(1)
            .unwrap()
            .split('}')
            .next()
            .unwrap();
        assert!(row.contains("\"owner\":1"));
        assert!(row.contains("\"countsComplete\":true"));
    }
    #[cfg(feature = "ee2-reference")]
    #[test]
    fn queued_research_refunds_old_owner_without_awarding_any_technology() {
        let mut g = humans();
        keep_city(&mut g, 0);
        let id = region(&g, 0);
        let base = g.ents[g.get(g.territory.claims[id as usize]).unwrap()];
        let (k, node) = crate::ee2_research::NODES
            .iter()
            .enumerate()
            .find(|(_, n)| n.epoch == 0)
            .unwrap();
        let b = g.spawn(node.host, 0, base.x + 8.0, base.y);
        g.research_points[0] = 1000.0;
        let before = g.research_points[0];
        assert!(g.train(b, ORIGINAL_RESEARCH0 + k as QueueItem));
        g.ents[b].qt = node.time * 0.8;
        assert!(g.apply_territory(0, 1, 0, id as u32, NONE));
        assert_eq!(g.research_points[0], before);
        assert_eq!(g.research_points[1], 0.0);
        assert!(!g.has_research(0, k) && !g.has_research(1, k));
        assert_eq!(g.ents[b].qn, 0);
    }
}
