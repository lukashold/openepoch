export function mountSkirmishTeams({wasm, document, opponents, victory, soleSurvivor}) {
  if (typeof wasm.configure_skirmish_teams !== 'function') return {update() {}, apply: () => true};

  const section = document.createElement('fieldset');
  section.id = 'sk-teams';
  const legend = document.createElement('legend');
  legend.textContent = 'Teams';
  section.append(legend);

  function addRow(id, title, control) {
    const row = document.createElement('div');
    row.className = 'row';
    const label = document.createElement('label');
    label.htmlFor = control.id = id;
    label.textContent = title;
    row.append(label, control);
    section.append(row);
    return row;
  }

  const lock = document.createElement('input');
  lock.type = 'checkbox';
  addRow('sk-lock-teams', 'Feste Teams', lock);
  const seats = Array.from({length: 4}, (_, seat) => {
    const select = document.createElement('select');
    for (let team = 0; team <= 10; team++) {
      const option = document.createElement('option');
      option.value = String(team);
      option.textContent = team ? `Team ${team}` : 'Ohne Team';
      select.append(option);
    }
    return {select, row: addRow(`sk-team-${seat}`, seat ? `Spieler ${seat + 1}` : 'Dein Team', select)};
  });
  const note = document.createElement('p');
  note.textContent = 'Für Teams in Eroberung „Einziger Überlebender“ ausschalten.';
  section.append(note);
  opponents.parentElement.after(section);

  const enabled = () => !victory.disabled && victory.value === 'conquest' && !soleSurvivor.checked;
  const update = () => {
    section.hidden = victory.disabled || victory.value !== 'conquest';
    const playable = enabled();
    note.hidden = playable;
    lock.disabled = !playable;
    const players = Number(opponents.value) + 1;
    seats.forEach(({row, select}, seat) => {
      row.hidden = seat >= players;
      select.disabled = row.hidden || !playable;
    });
  };
  for (const element of [opponents, victory, soleSurvivor]) element.addEventListener('change', update);
  update();

  return {update, apply(players) {
    if (!enabled()) return true;
    const packed = seats.slice(0, players).reduce((value, {select}, seat) => value | (Number(select.value) << (4 * seat)), 0);
    if (!lock.checked && packed === 0) return true;
    return wasm.configure_skirmish_teams(packed, Number(lock.checked)) === 1;
  }};
}
