const QRCode = require('qrcode');
const fs = require('fs');
const [, , text, out] = process.argv;
if (!text || !out) {
  console.error('usage: qrgen.cjs <text> <png>');
  process.exit(2);
}
QRCode.toFile(out, text, {
  errorCorrectionLevel: 'M',
  width: 512,
  margin: 2,
  color: { dark: '#000000', light: '#FFFFFF' },
}, (err) => { if (err) { console.error(err.message); process.exit(1); } });
