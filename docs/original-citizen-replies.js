// UIUnitSounds: 007C8160 admits an action, then 007C80A0 picks an index.
// The visible game registry identifies the lifetime of its local type cache.
export function createCitizenReplyPicker(random, seconds) {
  let session;
  const channels=new Map();
  return (plan, scope) => {
    const {world,owner}=scope;
    const {sourceIdentity,action,records}=plan;
    if(!Number.isInteger(world)||world<=0||!Number.isInteger(owner)||owner<0||owner>=4||
       ![0,1].includes(sourceIdentity)||!['select','order'].includes(action)||!records.length) return undefined;
    if(session!==world){session=world;channels.clear();}
    const key=`${owner}:${sourceIdentity}:${action}`;
    const previous=channels.get(key);
    const now=seconds();
    if(!(now>(previous?.time??0)+1)) return null;
    const bank=records.map(record=>record.file).join('\0');
    let index=random.word15()%records.length;
    if(bank===previous?.bank&&index===previous.index)index=(index+1)%records.length;
    channels.set(key,{time:Math.fround(now),bank,index});
    return records[index];
  };
}
