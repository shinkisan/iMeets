const QRCode = require('qrcode');
const fs = require('fs');
const [, , text, out] = process.argv;
if (!text || !out) {
  console.error('usage: qrraw.cjs <text> <rgb>');
  process.exit(2);
}
const qr = QRCode.create(text, { errorCorrectionLevel: 'M' });
const size = qr.modules.size;
const data = qr.modules.data;
const scale = 4;
const border = 4 * scale;
const dim = size * scale + border * 2;
const rgb = Buffer.alloc(dim * dim * 3, 255);
for (let y = 0; y < size; y++) {
  for (let x = 0; x < size; x++) {
    if (!data[y * size + x]) continue;
    for (let dy = 0; dy < scale; dy++) {
      for (let dx = 0; dx < scale; dx++) {
        const px = x * scale + dx + border;
        const py = y * scale + dy + border;
        const i = (py * dim + px) * 3;
        rgb[i] = 0; rgb[i + 1] = 0; rgb[i + 2] = 0;
      }
    }
  }
}
fs.writeFileSync(out, rgb);
console.log(JSON.stringify({ dim }));
