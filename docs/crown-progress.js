const kinds = ['Militär', 'Wirtschaft', 'Imperium'];
const types = ['military', 'economic', 'imperial'];
const manifestPath = 'local-original/crowns/manifest.json';
const powers = [
  [
    [0,'airsuperiority','Luftüberlegenheit','Jagdflugzeuge erhalten 20 % mehr Lebenspunkte und Angriffskraft.'],
    [1,'amphibiouswarfare','Amphibische Kriegsführung','Transport- und Landungsschiffe erhalten 50 % mehr Lebenspunkte und Tempo.'],
    [2,'firesupport','Feuerunterstützung','Artillerie erhält 20 % mehr Lebenspunkte und Angriffskraft.'],
    [3,'infantry','Infanteriedoktrin','Leichte und schwere Infanterie erhält 20 % mehr Lebenspunkte und Angriffskraft.'],
    [4,'mobilewarfare','Mobile Kriegsführung','Leichte und schwere berittene und mechanisierte Einheiten erhalten 20 % mehr Lebenspunkte und Angriffskraft.'],
    [5,'sealanes','Seewege','Kriegsschiffe erhalten 20 % mehr Lebenspunkte und Angriffskraft.'],
    [6,'staticdefense','Statische Verteidigung','Abwehrstellungen erhalten 20 % mehr Lebenspunkte und Reichweite. Mauern erhalten doppelte Sichtweite.'],
    [7,'strategicbombing','Strategische Bombardierung','Bomber erhalten 20 % mehr Lebenspunkte und Angriffskraft.'],
    [8,'verticalenvelopment','Vertikale Umfassung','Jagd- und Kampfhubschrauberverbände werden verstärkt. Lufttransporter erhalten mehr Lebenspunkte und Platz.'],
  ],
  [
    [0,'aviationexpertise','Luftfahrtexpertise','Flugplätze produzieren Einheiten 20 % günstiger und in 20 % kürzerer Zeit.'],
    [1,'farming','Landwirtschaft','Bürger ernten Nahrung auf Feldern doppelt so schnell.'],
    [2,'foodprocessing','Nahrungsverarbeitung','Bürger sammeln Nahrung aus Wild und Futterpflanzen doppelt so schnell.'],
    [3,'immigration','Immigration','Während die Krone wirkt, treffen regelmäßig drei Bürger an einem Stadtzentrum ein.'],
    [4,'incometax','Einkommensteuer','Bürger fördern Gold doppelt so schnell.'],
    [5,'lumbering','Holzwirtschaft','Bürger schlagen Holz doppelt so schnell.'],
    [6,'marketstability','Marktstabilität','Käufe und Verkäufe verändern die Preise auf deinen Märkten langsamer.'],
    [7,'quarrying','Steinbrüche','Bürger bauen Stein doppelt so schnell ab.'],
    [8,'retooling','Umrüstung','Bürger fördern Zinn, Eisen und Salpeter doppelt so schnell. Ölförderung und Uranabbau liefern den doppelten Ertrag.'],
    [9,'rulethewaves','Herrschaft über die Meere','Häfen produzieren Einheiten 20 % günstiger und in 20 % kürzerer Zeit.'],
    [10,'wareconomy','Kriegswirtschaft','Kasernen, Mühlen und Ställe produzieren Einheiten 10 % günstiger und in 10 % kürzerer Zeit.'],
  ],
  [
    [0,'centralization','Zentralisierung','Eigene Gebäude im Stadtgebiet erleiden 30 % weniger Schaden und leisten 30 % länger Widerstand bei Eroberung.'],
    [1,'drills','Exerzieren','Landeinheiten der betroffenen Truppentypen bewegen sich 20 % schneller.'],
    [2,'expansionism','Expansion','Stadtzentren tragen 30 % mehr zum Bevölkerungslimit bei.'],
    [3,'exploration','Erkundung','Schiffe erhalten 25 % mehr Sichtweite und Tempo.'],
    [4,'fanaticism','Fanatismus','Eigene bekehrbare mobile Einheiten sind gegen Bekehrung geschützt.'],
    [5,'firingranges','Schießstände','Fernkämpfer zu Fuß, zu Pferd und in Fahrzeugen erhalten 20 % mehr Angriffsreichweite.'],
    [6,'missionaries','Missionare','Priester kosten 25 % weniger und erhalten 25 % mehr Tempo und Bekehrungsreichweite.'],
    [7,'piracy','Piraterie','Wenn eigene Schiffe gegnerische Einheiten oder Gebäude zerstören, erhältst du die Hälfte ihrer Rohstoffkosten.'],
    [8,'socializedmedicine','Öffentliches Gesundheitswesen','Heilung im Stadtgebiet und für stationierte Einheiten in Flugplätzen und Festungen steigt um 33 %.'],
    [9,'superspies','Superspione','25 % mehr Lebenspunkte und Tempo für Spione. Sabotage hält 25 % länger.'],
    [10,'thinktanks','Denkfabriken','Universitäten und Tempel liefern 10 % mehr Technologiepunkte.'],
  ],
];

export async function loadCrownArt(enabled) {
  if (!enabled) return {};
  try {
    const response = await fetch(manifestPath);
    if (!response.ok) return {};
    const manifest = await response.json();
    if (manifest.schema !== 1) return {};
    return manifest.profiles?.packed7?.images ?? {};
  } catch {
    return {};
  }
}

function background(art, name) {
  const row = art[name];
  if (!row || !/^local-original\/crowns\/packed7\/[a-z0-9_]+\.png$/.test(row.url)
      || !/^[a-f0-9]{64}$/.test(row.sha256)) return '';
  return `url("${row.url}?v=${row.sha256.slice(0,16)}")`;
}

function item(tag, name, content) {
  const element = document.createElement(tag);
  element.className = name;
  if (content) element.textContent = content;
  return element;
}

function formatTime(seconds) {
  const remaining = Math.ceil(seconds);
  return `${Math.floor(remaining / 60)}:${String(remaining % 60).padStart(2, '0')}`;
}

export function mountCrownProgress({button, panel, ledger, timers, chooser, wasm, art}) {
  const crownButton = background(art, 'spr_crownsbut');
  if (crownButton) button.style.setProperty('--crown-button', crownButton);
  const choiceList = chooser.querySelector('.crown-choices');
  chooser.style.setProperty('--crown-dialog-top', background(art, 'spr_game_dialog_bkg_wide_top'));
  chooser.style.setProperty('--crown-dialog-bottom', background(art, 'spr_game_dialog_bkg_wide_bot'));
  chooser.style.setProperty('--crown-close-art', background(art, 'spr_game_metabut_med'));
  const available = kind => powers[kind].filter(([index]) => wasm.crown_can_choose_power(kind, index) === 1);
  let openKind = -1;
  function open(kind) {
    if (chooser.open) return;
    const options = available(kind);
    if (!options.length) return;
    openKind = kind;
    const choices = options.map(([index,key,label,description]) => {
      const option = item('button', 'crown-power-choice');
      option.type = 'button';
      option.dataset.crownPower = key === 'superspies' ? 'super-spies' : key;
      option.style.setProperty('--crown-choice-icon', background(art, `icon_crown_${key}`));
      option.append(item('strong', '', label), item('span', '', description));
      option.addEventListener('click', () => {
        if (wasm.crown_choose_power(kind, index) === 1) chooser.close();
        update(own);
      });
      return option;
    });
    choiceList.replaceChildren(...choices);
    chooser.showModal();
  }
  const columns = kinds.map((name, kind) => {
    const timer = item('button', 'crown-timer');
    timer.type = 'button';
    timer.disabled = true;
    timer.hidden = true;
    timer.dataset.kind = kind;
    timer.style.backgroundImage = background(art, 'spr_crowns_timer_bkg');
    const icon = item('i', 'crown-timer-icon');
    icon.style.backgroundImage = background(art, `icon_crown_${types[kind]}`);
    const time = item('span', 'crown-timer-time');
    const ring = item('i', 'crown-timer-ring');
    ring.style.backgroundImage = background(art, 'spr_crowns_timer_circle');
    timer.append(ring, icon, time);
    timer.addEventListener('click', () => { if (!timer.disabled) open(kind); });
    timers.append(timer);
    return {name, timer, time};
  });
  const rows = Array.from({length: 4}, (_, owner) => {
    const row = item('div', 'crown-ledger-row');
    row.hidden = true;
    const player = item('b', 'crown-ledger-player', `Spieler ${owner + 1}`);
    row.append(player);
    const categories = kinds.map((name, kind) => {
      const cell = item('div', 'crown-ledger-cell');
      const icon = item('i', 'crown-ledger-icon');
      icon.style.backgroundImage = background(art, `icon_crown_${types[kind]}`);
      const score = item('span', 'crown-ledger-score');
      const techs = Array.from({length: 4}, () => item('i', 'crown-tech'));
      const meter = item('span', 'crown-techs');
      meter.style.setProperty('--crown-meter', background(art, 'spr_crownmeter_short'));
      meter.append(...techs);
      cell.append(icon, item('span', 'crown-ledger-kind', name), score, meter);
      row.append(cell);
      return {score, techs};
    });
    ledger.append(row);
    return {row, player, categories};
  });
  let own = 0;
  function close() {
    panel.hidden = true;
    button.setAttribute('aria-expanded', 'false');
    if (chooser.open) chooser.close();
  }
  function toggle() {
    if (button.hidden) return false;
    panel.hidden = !panel.hidden;
    button.setAttribute('aria-expanded', String(!panel.hidden));
    if (!panel.hidden) update(own);
    return true;
  }
  function update(local) {
    own = local;
    const present = Number.isFinite(wasm.crown_rating(local, 0, 0));
    button.hidden = !present;
    if (!present) close();
    for (const [kind, {name, timer, time}] of columns.entries()) {
      const seconds = wasm.crown_holder(kind, 1);
      timer.hidden = !present || wasm.crown_holder(kind, 0) !== local + 1;
      timer.disabled = timer.hidden || available(kind).length === 0;
      if (openKind === kind && timer.disabled && chooser.open) chooser.close();
      if (timer.hidden) continue;
      timer.style.setProperty('--crown-angle', `${Math.min(1, seconds / 300) * 360}deg`);
      const formatted = formatTime(seconds);
      time.textContent = formatted;
      timer.title = `${name} · ${formatted}${timer.disabled ? '' : ' · Kronenkraft auswählen'}`;
    }
    if (panel.hidden) return;
    rows.forEach(({row, player, categories}, owner) => {
      row.hidden = !Number.isFinite(wasm.crown_rating(owner, 0, 0));
      if (row.hidden) return;
      player.textContent = owner === local ? 'Du' : `Spieler ${owner + 1}`;
      categories.forEach(({score, techs}, kind) => {
        const current = Math.trunc(wasm.crown_rating(owner, kind, 0));
        const target = wasm.crown_rating(owner, kind, 1);
        score.textContent = `${current} / ${target}`;
        techs.forEach((tech, slot) => tech.classList.toggle('done', wasm.crown_technology(owner, kind, slot) === 1));
      });
    });
  }
  button.addEventListener('click', toggle);
  return {toggle, update, close};
}
