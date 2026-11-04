// Enter only from a click/key gesture. Browser/Electron permissions remain in
// charge; an unavailable request is reported instead of faking fullscreen.
export function createFullscreenController({document:doc=globalThis.document,buttons=[],notify=()=>{},onChange=()=>{}}={}){
  let pending=false;
  const active=()=>!!doc.fullscreenElement;
  function sync(){
    const full=active();
    for(const button of buttons){
      button.textContent=full?'Vollbild beenden':'Vollbild';
      button.setAttribute('aria-pressed',String(full));button.disabled=pending;
      button.title=full?'Vollbild verlassen · F11 oder Esc':'Spiel im Vollbild anzeigen · F11';
    }
    onChange(full);
  }
  async function toggle(){
    if(pending)return false;
    if(!active()&&typeof doc.documentElement?.requestFullscreen!=='function'){
      notify('Dieser Browser unterstützt kein Spielvollbild. Im normalen Browser kannst du F11 verwenden.');return false;
    }
    pending=true;sync();
    try{
      if(active())await doc.exitFullscreen();else await doc.documentElement.requestFullscreen();
      return active();
    }catch{
      notify('Vollbild wurde vom Browser blockiert. Öffne das Spiel gegebenenfalls im normalen Browser und nutze F11.');return false;
    }finally{pending=false;sync();}
  }
  for(const button of buttons)button.addEventListener('click',toggle);
  doc.addEventListener('fullscreenchange',sync);sync();
  return {toggle,get active(){return active();},dispose(){doc.removeEventListener('fullscreenchange',sync);for(const button of buttons)button.removeEventListener('click',toggle);}};
}
