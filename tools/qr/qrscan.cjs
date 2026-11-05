const fs = require('fs');
const jsQR = require('jsqr');
const { PNG } = require('pngjs');
const file = process.argv[2];
if (!file) {
  console.error('usage: qrscan.cjs <png>');
  process.exit(2);
}
const png = PNG.sync.read(fs.readFileSync(file));
const code = jsQR(new Uint8ClampedArray(png.data), png.width, png.height);
if (!code) {
  console.error('no qr code');
  process.exit(1);
}
console.log(code.data);
