#!/usr/bin/env python3
"""Reproducible extraction of original Soulbinder portrait and seven skill cells.
Source generated 2026-10-05 with image_gen; funerary silver/jade palette.
The raw sheet intentionally remains tracked to permit deterministic rebuilding.
"""
from pathlib import Path
import subprocess
ROOT=Path(__file__).resolve().parents[2]
SOURCE=Path(__file__).with_name('soulbinder-sources')/'soulbinder-sheet.png'
OUT=ROOT/'assets/images/generated/ui'
def magick(*args): subprocess.run(['magick',*map(str,args)],check=True)
magick(SOURCE,'-crop','1100x1024+0+0','+repage','-resize','768x1024^','-gravity','center','-extent','768x1024','-strip',f"PNG32:{OUT/'class-portrait-soulbinder.png'}")
args=[]
for i in range(7):
    args += ['(',str(SOURCE),'-gravity','NorthWest','-crop',f'161x137+1141+{7+i*142}','+repage','-resize','206x206!','-background','black','-gravity','center','-extent','256x256',')']
magick(*args,'+append','-strip',f"PNG32:{OUT/'soulbinder-skill-icons.png'}")

# Reject an empty or nearly black result before shipping UI artwork.
mean=float(subprocess.check_output(['magick',str(OUT/'soulbinder-skill-icons.png'),'-format','%[fx:mean]','info:'],text=True))
if mean < .04:
    raise RuntimeError(f'Skill atlas lost visible RGB content: mean={mean}')
