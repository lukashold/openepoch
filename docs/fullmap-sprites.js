export function createFullmapSpriteCache(teams) {
  const images = new Map(), tinted = new Map();

  function image(sprite) {
    if (!sprite) return null;
    if (!images.has(sprite.url)) {
      const img = new Image();
      img.decoding = 'async'; img.src = sprite.url;
      images.set(sprite.url,img);
    }
    const img = images.get(sprite.url);
    return img.complete && img.naturalWidth ? img : null;
  }

  function colorize(sprite,img,owner) {
    const key = `${sprite.url}:${owner}`;
    if (!tinted.has(key)) {
      const canvas = document.createElement('canvas');
      canvas.width = img.naturalWidth;
      canvas.height = img.naturalHeight;
      const ctx = canvas.getContext('2d');
      ctx.drawImage(img,0,0);
      const pixels = ctx.getImageData(0,0,canvas.width,canvas.height);
      const color = teams[owner].map(value=>Math.round(value*255)/255);
      for (let offset=0; offset<pixels.data.length; offset+=4) {
        for (let channel=0; channel<3; channel++) pixels.data[offset+channel] *= color[channel];
      }
      ctx.putImageData(pixels,0,0);
      tinted.set(key,canvas);
    }
    return tinted.get(key);
  }

  return {image,colorize};
}
