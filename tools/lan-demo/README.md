# LAN QR demo

Runs the two-process QR pairing demo and writes a contact sheet with the QR,
the host screens and the join screens.

```sh
bash tools/lan-demo/run.sh
# report: tools/lan-demo/out/report.png
```

Defaults:

- host save: `/tmp/male-adult`
- join save: `/tmp/female-adult`
- ROM: `/mnt/c/Users/gurri/Downloads/fw_tg18_jp_v062_2019-09-12_sweets.bin`
- `POCKET_MEETS_ROM` overrides the ROM
- the first two arguments override the saves

The QR helper under `tools/qr` needs `npm install` once.
