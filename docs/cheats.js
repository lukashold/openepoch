// EE2 command spelling; effects are implemented in the Rust single-player simulation.
const codes = new Map([['icheat',0],['idontcheat',1],['loot',2],['sea monkeys',3],['give tech',4]]);
const upCodes = new Map([['taxes',5],['punish',6],['play god',7],['toggle fog',8]]);
export function parseCheat(input, up = false) {
  const command = input.trim().toLowerCase().replace(/\s+/g,' ');
  return codes.get(command) ?? (up ? upCodes.get(command) : null) ?? null;
}
