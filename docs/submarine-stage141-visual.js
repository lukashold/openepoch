const submarineDefinitions=new Set([34,36,45]);
const phases=new Map([
  [10,Object.freeze({definition:34,epoch:10,voice:'Submarine_11'})],
  [12,Object.freeze({definition:36,epoch:12,voice:'Submarine_13'})],
  [14,Object.freeze({definition:45,epoch:14,voice:'Submarine_15'})],
]);

export function submarineStageVisual(definition,stage){
  return submarineDefinitions.has(definition)?phases.get(stage):undefined;
}
