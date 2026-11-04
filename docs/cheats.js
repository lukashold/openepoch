// EE2 command spelling; effects are implemented in the Rust single-player simulation.
const codes = new Map([['icheat',0],['idontcheat',1],['loot',2],['sea monkeys',3],['give tech',4]]);
export function parseCheat(input) {
  return codes.get(input.trim().toLowerCase().replace(/\s+/g,' ')) ?? null;
}
