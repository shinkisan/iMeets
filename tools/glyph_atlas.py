#!/usr/bin/env python3
"""Render captured glyph JSON to a labelled PNG for manual verification."""
import argparse,json,struct,zlib
from pathlib import Path

DIGITS=['111101101101111','010110010010111','111001111100111','111001111001111','101101111001001','111100111001111','111100111101111','111001001001001','111101111101111','111101111001111','010101111101101','110101110101110','011100100100011','110101101101110','111100110100111','111100110100100']

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('input',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
 entries=json.loads(a.input.read_text());w=600;h=((len(entries)+7)//8)*90;pixels=bytearray([255])*(w*h*3)
 def put(x,y,c):
  if 0<=x<w and 0<=y<h:pixels[(y*w+x)*3:(y*w+x)*3+3]=bytes(c)
 for i,item in enumerate(entries):
  ox=i%8*75;oy=i//8*90
  for j,ch in enumerate(item['code'][-2:]):
   pattern=DIGITS[int(ch,16)]
   for yy in range(5):
    for xx in range(3):
     if pattern[yy*3+xx]=='1':
      for sy in range(2):
       for sx in range(2):put(ox+20+j*9+xx*2+sx,oy+yy*2+sy,(0,0,0))
  for yy,row in enumerate(item['rows']):
   for xx,ch in enumerate(row):
    if ch=='#':
     for sy in range(4):
      for sx in range(4):put(ox+10+xx*4+sx,oy+20+yy*4+sy,(0,0,80))
 def chunk(k,b):return struct.pack('>I',len(b))+k+b+struct.pack('>I',zlib.crc32(k+b)&0xffffffff)
 raw=b''.join(b'\0'+pixels[y*w*3:(y+1)*w*3] for y in range(h))
 a.output.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b''))

if __name__=='__main__':main()
