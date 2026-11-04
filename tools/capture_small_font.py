#!/usr/bin/env python3
"""Capture Sweets' 8px status-name font from a snapshot paused mid-render.

The snapshot must be stopped after the first name character was drawn and
before the second is read. Only a temporary snapshot is modified. Verify the
first character remains か and independently verify the resulting table.
"""
import argparse,json,struct,subprocess,tempfile,zlib
from pathlib import Path
from capture_menu_font import codes, png_rgb


def locate(path):
    snapshot = path.read_bytes()
    if snapshot[:8] != b'T18SNAP1': raise ValueError('bad snapshot magic')
    data = bytearray(zlib.decompress(snapshot[8:])); i=0
    while i < len(data):
        n=data[i];i+=1;name=bytes(data[i:i+n]).decode();i+=n
        size=struct.unpack_from('<I',data,i)[0];i+=4
        if name=='sram':
            pos=i+0x17874
            if data[pos:pos+6] != bytes.fromhex('040604190000'):
                raise ValueError('snapshot does not contain the expected name at the expected address')
            return snapshot[:8],data,pos
        i+=size
    raise ValueError('no SRAM chunk')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('rom','snapshot','emulator','output'):parser.add_argument('--'+name,required=True,type=Path)
    parser.add_argument('--only',type=lambda x:int(x,0),action='append')
    parser.add_argument('--skip-empty',action='store_true')
    args=parser.parse_args();magic,data,pos=locate(args.snapshot);captured=[]
    with tempfile.TemporaryDirectory(prefix='small-font-') as tmp:
        snap=Path(tmp)/'input.t18s';image=Path(tmp)/'screen.png'
        for code,ch in codes():
            if args.only and code not in args.only:continue
            data[pos:pos+6]=bytes.fromhex('0406')+code.to_bytes(2,'big')+bytes(2)
            snap.write_bytes(magic+zlib.compress(data,1))
            p=subprocess.run([str(args.emulator),str(args.rom),'--snapshot-in',str(snap),'--seconds','0.5','--screenshot',str(image),'--quiet'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            if p.returncode or 'result: Limit' not in p.stdout:raise RuntimeError(p.stderr+'\n'+p.stdout)
            w,h,rows=png_rgb(image)
            if (w,h)!=(384,384):raise ValueError('unexpected screen size')
            glyph=[]
            for y in range(78,87):
                line=''
                for x in range(20,30):
                    r,g,b=rows[y*3][x*9:x*9+3]
                    line += '#' if r<100 and g<100 and b>60 else '.'
                glyph.append(line)
            if not any('#' in line for line in glyph):
                if args.skip_empty:
                    print(f'{code:04x} {ch} blank',flush=True)
                    continue
                raise RuntimeError(f'empty glyph {code:04x} {ch}')
            captured.append({'code':f'{code:04x}','char':ch,'rows':glyph})
            print(f'{code:04x} {ch}',flush=True)
    args.output.write_text(json.dumps(captured,ensure_ascii=False,indent=2)+'\n')

if __name__=='__main__':main()
