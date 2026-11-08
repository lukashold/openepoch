export function checkStartingTeams(value, players) {
  if (value === undefined) return undefined;
  if (!value || typeof value.locked !== 'boolean' || !Array.isArray(value.groups) ||
      value.groups.length !== players || players < 2 || players > 4 ||
      Array.from(value.groups).some(group => !Number.isInteger(group) || group < 0 || group > 10) ||
      (!value.locked && value.groups.every(group => group === 0))) {
    throw Error('Ungültige Teamzuordnung für diesen Raum.');
  }
  return {groups: [...value.groups], locked: value.locked};
}

export function packStartingTeams({groups}) {
  return groups.reduce((packed, group, seat) => packed | (group << (4 * seat)), 0);
}

export function checkpointStartingTeams(teams) {
  return teams?.locked ? (1 << 16) | packStartingTeams(teams) : 0;
}

export function mountNetworkTeamPicker({document, anchor, victory, available}) {
  if (!available) return {configure() {}, read: () => undefined};

  const fieldset = document.createElement('fieldset');
  fieldset.id = 'mp-roster-starting-teams';
  const legend = document.createElement('legend');
  legend.textContent = 'Startteams';
  fieldset.append(legend);
  const addRow = (title, control) => {
    const label = document.createElement('label');
    label.append(title);
    const row = document.createElement('div');
    row.append(label, control);
    row.className = 'row';
    label.htmlFor = control.id;
    fieldset.append(row);
    return {label, row};
  };

  const lock = document.createElement('input');
  lock.type = 'checkbox';
  lock.id = 'mp-roster-lock-teams';
  addRow('Feste Teams', lock);
  const seats = Array.from({length: 4}, (_, seat) => {
    const select = document.createElement('select');
    select.id = `mp-roster-team-${seat}`;
    for (let group = 0; group <= 10; group++) {
      const option = document.createElement('option');
      option.value = String(group);
      option.textContent = group === 0 ? 'Ohne Team' : `Team ${group}`;
      select.append(option);
    }
    return {select, ...addRow(`Spieler ${seat + 1}`, select)};
  });
  anchor.before(fieldset);

  let players = 0;
  const enabled = () => players > 0 && victory.value === 'conquest';
  const update = () => { fieldset.hidden = !enabled(); };
  victory.addEventListener('change', update);
  update();
  return {
    configure(humans, ai, editable) {
      players = editable ? humans + ai : 0;
      lock.checked = false;
      seats.forEach(({select, row, label}, seat) => {
        select.value = '0';
        row.hidden = seat >= players;
        label.textContent = seat >= humans ? `KI ${seat - humans + 1}` : `Spieler ${seat + 1}`;
      });
      update();
    },
    read() {
      if (!enabled()) return undefined;
      const groups = seats.slice(0, players).map(({select}) => Number(select.value));
      return lock.checked || groups.some(group => group !== 0) ? checkStartingTeams({groups, locked: lock.checked}, players) : undefined;
    },
  };
}
