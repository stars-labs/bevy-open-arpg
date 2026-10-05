"""Restrained ground sigils and tapered dash ribbons; Blender 5.1.

Run with -- --preview-only after scoped material postprocessing for contact sheet.
"""
import argparse
import json
import math
from pathlib import Path
import struct
import subprocess
import sys

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
import generate_reliquary_props as art

art.PREVIEW = art.ROOT / 'target/validation/reliquary-effects'
COLORS = {
    'ember': (.58, .14, .035), 'arcane': (.27, .20, .55),
    'frost': (.21, .45, .55), 'blood': (.42, .035, .055),
    'ward': (.50, .38, .12),
}


def polar(radius, angle, height=.038):
    return (radius * math.cos(angle), radius * math.sin(angle), height)


def aura(kind):
    mat = art.material(f'{kind} inscribed aura', COLORS[kind], glow=.55)
    art.ring('fine outer boundary', (0, 0, .035), .71, .010, mat)
    if kind == 'arcane':
        for phase in [0, math.pi]:
            art.tube('triangular binding', [polar(.55, phase + i*math.tau/3)
                     for i in range(4)], .010, mat)
    elif kind == 'frost':
        for i in range(6):
            a = i * math.tau / 6
            art.tube('frost ray', [polar(.38,a), polar(.61,a)], .010, mat)
            art.tube('frost fork', [polar(.48,a-.12), polar(.55,a),
                     polar(.48,a+.12)], .008, mat)
    elif kind == 'blood':
        for i in range(3):
            a=i*math.tau/3
            art.tube('siphon crescent', [polar(.48 + .035*math.sin(j*math.pi/14),
                     a + j*.085) for j in range(15)], .012, mat)
    elif kind == 'ward':
        for i in range(4):
            a=i*math.pi/2
            art.tube('ward shield glyph', [polar(.53,a-.18),polar(.62,a-.18),
                     polar(.62,a+.18),polar(.53,a+.18),polar(.45,a),polar(.53,a-.18)], .009, mat)
    else:
        for i in range(6):
            a=i*math.tau/6
            art.tube('ember curl', [polar(.44,a-.13),polar(.55,a-.11),
                     polar(.61,a),polar(.55,a+.09)], .010, mat)
    # Tiny radial etched ticks provide a common visual language, never pillars.
    for i in range(12):
        a=i*math.tau/12
        art.tube('boundary etching', [polar(.68,a),polar(.72,a)], .007, mat)


def shadow_trail():
    mat = art.material('silver blue shadow ribbon', (.15,.31,.42), glow=.35)
    for streak in range(3):
        verts=[]
        count=20
        for i in range(count+1):
            t=i/count
            x=-.78 + t*1.32
            y=(streak-1)*.13 + math.sin(t*math.pi)*.09
            z=.22 + streak*.065 + math.sin(t*math.pi)*.045
            width=math.sin(t*math.pi)**.8 * (.038-streak*.006)
            verts.extend([(x,y-width,z),(x,y+width,z)])
        faces=[(i*2,i*2+1,i*2+3,i*2+2) for i in range(count)]
        mesh=bpy.data.meshes.new('tapered motion ribbon')
        mesh.from_pydata(verts,[],faces)
        obj=bpy.data.objects.new('tapered motion ribbon',mesh)
        bpy.context.collection.objects.link(obj)
        art.finish(obj,obj.name,mat)


def crescent(name, radius, start, sweep, width, height, mat, ellipse=1.0):
    vertices=[]
    count=28
    for i in range(count+1):
        t=i/count
        angle=start+sweep*t
        thickness=width*math.sin(math.pi*t)**.65
        for r in [radius-thickness*.5,radius+thickness*.5]:
            vertices.append((math.cos(angle)*r,math.sin(angle)*r*ellipse,height))
    mesh=bpy.data.meshes.new(name)
    mesh.from_pydata(vertices,[],[(i*2,i*2+1,i*2+3,i*2+2) for i in range(count)])
    obj=bpy.data.objects.new(name,mesh)
    bpy.context.collection.objects.link(obj)
    art.finish(obj,name,mat)


def combat_effect(name):
    color=(.32,.42,.49) if name=='slash_arc' else ((.15,.32,.44) if name=='arcane_impact' else (.52,.42,.27))
    mat=art.material(name+' etched energy',color,glow=.40)
    if name=='slash_arc':
        crescent('sweeping forged edge',.82,0,math.pi,.07,.055,mat,.37)
        crescent('fading inner edge',.73,.14,math.pi*.83,.025,.062,mat,.37)
    elif name=='bone_fracture_echo':
        for i in range(3):
            crescent('broken echo arc',.39,i*math.tau/3,math.pi*.49,.036,.075+i*.009,mat,.78)
            a=i*math.tau/3
            art.tube('fracture zigzag',[polar(.16,a,.08),polar(.24,a+.09,.08),polar(.32,a-.04,.08)],.008,mat)
    elif name=='arcane_impact':
        art.ring('etched arcane boundary',(0,0,.065),.41,.009,mat)
        for i in range(3):
            a=i*math.tau/3
            crescent('arcane spiral trace',.28,a,math.pi*.48,.028,.085,mat)
    elif name=='hit_bone_lock':
        for i in range(4):
            a=i*math.pi/2
            art.tube('contact lock corner',[polar(.21,a-.22,.06),polar(.26,a-.22,.06),
                polar(.26,a+.22,.06),polar(.21,a+.22,.06)],.008,mat)
    else:
        for i in range(4):
            a=i*math.pi/2
            crescent('runic contact arc',.31,a,math.pi*.32,.018,.06,mat,.74)
            art.tube('angular hit rune',[polar(.20,a,.06),polar(.24,a+.10,.06),
                polar(.28,a,.06)],.007,mat)


# Contact effects are open, tapered geometry: never opaque cores or radial rods.
CONTACT_EFFECTS = {
    'hit_spark': 'steel', 'bone_shatter': 'bone', 'bone_impact': 'bone',
    'blood_spray': 'blood', 'execution_burst': 'blood', 'holy_impact': 'gold',
    'ember_impact': 'ember', 'frost_impact': 'frost', 'void_impact': 'void',
    'frenzy_impact': 'blood', 'vampiric_siphon': 'blood',
    'desecrator_burst': 'void', 'guard_clash': 'steel', 'armor_break': 'steel',
    'soul_ward_hit': 'gold', 'marrow_flash': 'bone', 'elite_affix_break': 'void',
    'shadow_burst': 'void', 'headshot_burst': 'gold', 'crit_bone_crown': 'bone',
    'crit_burst': 'ember', 'stagger_burst': 'gold',
}


def contact_effect(name):
    kind = CONTACT_EFFECTS[name]
    colors = {'steel': (.48,.40,.25), 'bone': (.46,.40,.29),
              'blood': (.23,.016,.012), 'gold': (.48,.32,.10),
              'ember': (.55,.12,.024), 'frost': (.18,.40,.49),
              'void': (.21,.12,.31)}
    mat = art.material(name+' contact fragments', colors[kind],
                       glow=.0 if kind in ('blood','bone') else .3)
    # Asymmetric, pointed flakes taper to a vertex in each direction. Their
    # sparse, low footprint keeps the target visible through a successful hit.
    count = 5 if kind in ('bone','steel') else 4
    verts, faces = [], []
    for i in range(count):
        a = i*2.399 + .31
        radius = .09 + .022*(i%3)
        length = .06 + .018*(i%3)
        width = .013 if kind == 'blood' else .009
        x,y = math.cos(a)*radius, math.sin(a)*radius
        z = .07 + .035*(i%3)
        tangent=(-math.sin(a),math.cos(a))
        start=len(verts)
        verts.extend([(x-math.cos(a)*length,y-math.sin(a)*length,z-.015),
                      (x+tangent[0]*width,y+tangent[1]*width,z+.006),
                      (x+math.cos(a)*length,y+math.sin(a)*length,z+.035),
                      (x-tangent[0]*width,y-tangent[1]*width,z-.006)])
        faces.extend([(start,start+1,start+2),(start,start+2,start+3)])
    mesh=bpy.data.meshes.new(name+' tapered flakes')
    mesh.from_pydata(verts,[],faces)
    obj=bpy.data.objects.new(name+' tapered flakes',mesh)
    bpy.context.collection.objects.link(obj)
    art.finish(obj,obj.name,mat)
    if kind in ('frost','void','gold'):
        for i in range(2):
            crescent('broken contact arc',.19+i*.04,i*math.pi+.2,
                     math.pi*.48,.012,.08+i*.016,mat,.68)


NAMES=[f'affix_{kind}_aura' for kind in COLORS]+[
    'shadow_trail','slash_arc','bone_fracture_echo','arcane_impact','hit_bone_lock','hit_bone_rune']
NAMES += list(CONTACT_EFFECTS)
MARKERS={f'affix_{kind}_aura':kind+' ground inscriptions' for kind in COLORS}
MARKERS.update({'shadow_trail':'tapered shadow ribbons','slash_arc':'forged slash crescent',
    'bone_fracture_echo':'fracture echo crescents','arcane_impact':'arcane impact sigil',
    'hit_bone_lock':'bone lock brackets','hit_bone_rune':'bone hit runic arcs'})

MARKERS.update({name: name+' tapered contact fragments' for name in CONTACT_EFFECTS})

def verify():
    report=[]
    for name in NAMES:
        raw=(art.OUT/f'{name}.glb').read_bytes()
        length=struct.unpack_from('<I',raw,12)[0]
        doc=json.loads(raw[20:20+length])
        primitives=[p for mesh in doc['meshes'] for p in mesh['primitives']]
        assert len(doc['scenes']) == 1 and len(primitives) == 1, name
        report.append({'asset':name,'bytes':len(raw),'material_primitives':len(primitives),
            'triangles':sum(doc['accessors'][p['indices']]['count']//3 for p in primitives)})
    (art.PREVIEW/'verified-effects.json').write_text(json.dumps(report,indent=2)+'\n')
    print(f'Verified {len(NAMES)} single-scene, single-material effects.')


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--preview-only',action='store_true')
    parser.add_argument('--only', nargs='+')
    args=parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
    art.PREVIEW.mkdir(parents=True,exist_ok=True)
    names=args.only or NAMES
    for name in names:
        if args.preview_only:
            art.preview(name)
        else:
            art.reset()
            if name in CONTACT_EFFECTS:contact_effect(name)
            elif name=='shadow_trail':shadow_trail()
            elif name.startswith('affix_'):aura(name.removeprefix('affix_').removesuffix('_aura'))
            else:combat_effect(name)
            art.export(name,semantic_name=MARKERS[name])
    verify()
    if args.preview_only:
        subprocess.run(['magick','montage',*[str(art.PREVIEW/f'{n}.png') for n in names],
            '-set','label','%t','-font',str(art.ROOT/'assets/fonts/CrimsonPro.ttf'),
            '-pointsize','20','-fill','#dac79e','-background','#222329',
            '-geometry','320x320+7+20','-tile','4x',str(art.PREVIEW/'contact-sheet.png')],check=True)


if __name__=='__main__':main()
