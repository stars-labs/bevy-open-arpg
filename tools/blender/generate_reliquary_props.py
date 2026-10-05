"""Authored gothic interactables; Blender 5.1, deterministic geometry and stable paths.

blender --background --factory-startup --python tools/blender/generate_reliquary_props.py
Then run postprocess_glb_materials.py -- --only <filenames printed in props-manifest.json>.
Use -- --preview-only after postprocessing to render the final runtime GLBs.
"""
import argparse
import json
import math
from pathlib import Path
import random
import struct
import subprocess
import sys

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'assets/models'
PREVIEW = ROOT / 'target/validation/props'
TAU = math.tau


def material(name, color, metallic=0.0, roughness=0.8, glow=0.0):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1)
    mat.use_nodes = True
    p = mat.node_tree.nodes.get('Principled BSDF')
    p.inputs['Base Color'].default_value = (*color, 1)
    p.inputs['Metallic'].default_value = metallic
    p.inputs['Roughness'].default_value = roughness
    if glow:
        p.inputs['Emission Color'].default_value = (*color, 1)
        p.inputs['Emission Strength'].default_value = glow
    return mat


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    random.seed(71)
    global STONE, IRON, BRASS, WOOD, BONE, DARK, EMBER, SOUL, BLOOD, PARCHMENT
    STONE = material('reliquary stone', (0.19, 0.18, 0.17))
    IRON = material('bastion iron', (0.11, 0.13, 0.14), 0.72, 0.48)
    BRASS = material('dull relic gold', (0.40, 0.26, 0.105), 0.7, 0.43)
    WOOD = material('old leather', (0.115, 0.055, 0.025))
    BONE = material('aged bone', (0.56, 0.49, 0.34))
    DARK = material('hollow obsidian', (0.025, 0.029, 0.033))
    PARCHMENT = material('charcoal cloth', (0.48, 0.39, 0.24))
    EMBER = material('reliquary ember inset', (0.60, 0.13, 0.025), glow=0.8)
    SOUL = material('reliquary soul inset', (0.08, 0.31, 0.37), glow=0.6)
    BLOOD = material('reliquary blood inset', (0.36, 0.025, 0.035), glow=0.45)


def finish(obj, name, mat, bevel=0):
    obj.name = name
    obj.data.materials.append(mat)
    if bevel:
        mod = obj.modifiers.new('Worn edge chamfer', 'BEVEL')
        mod.width, mod.segments = bevel, 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def box(name, pos, size, mat, bevel=0.018):
    bpy.ops.mesh.primitive_cube_add(size=1, location=pos)
    obj = bpy.context.object
    obj.dimensions = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return finish(obj, name, mat, bevel)


def ball(name, pos, size, mat):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=10, location=pos)
    obj = bpy.context.object
    obj.scale = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    for face in obj.data.polygons:
        face.use_smooth = True
    return finish(obj, name, mat)


def ring(name, pos, radius, thickness, mat, vertical=False):
    bpy.ops.mesh.primitive_torus_add(major_segments=40, minor_segments=8,
        location=pos, major_radius=radius, minor_radius=thickness)
    obj = bpy.context.object
    if vertical:
        obj.rotation_euler.x = math.pi / 2
    for face in obj.data.polygons:
        face.use_smooth = True
    return finish(obj, name, mat)


def tube(name, points, radius, mat):
    curve = bpy.data.curves.new(name, 'CURVE')
    curve.dimensions = '3D'
    curve.bevel_depth = radius
    curve.bevel_resolution = 2
    spline = curve.splines.new('POLY')
    spline.points.add(len(points)-1)
    for p, xyz in zip(spline.points, points):
        p.co = (*xyz, 1)
    obj = bpy.data.objects.new(name, curve)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(mat)
    return obj


def lathe(name, profile, mat, segments=32):
    verts = [(r*math.cos(i*TAU/segments), r*math.sin(i*TAU/segments), z)
             for r,z in profile for i in range(segments)]
    faces = []
    for level in range(len(profile)-1):
        for i in range(segments):
            j = (i+1)%segments
            faces.append((level*segments+i, level*segments+j,
                         (level+1)*segments+j, (level+1)*segments+i))
    faces += [tuple(reversed(range(segments))),
              tuple((len(profile)-1)*segments+i for i in range(segments))]
    mesh=bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    obj=bpy.data.objects.new(name,mesh)
    bpy.context.collection.objects.link(obj)
    for face in mesh.polygons:
        face.use_smooth = len(face.vertices)==4
    return finish(obj,name,mat)


def base(radius=0.6):
    lathe('octagonal weathered footing', [(radius,0.025),(radius,0.12),
        (radius*0.88,0.18),(radius*0.86,0.22)], STONE, 8)
    ring('inlaid brass perimeter',(0,0,0.17),radius*0.84,0.012,BRASS)


def skull(pos, scale=1):
    x,y,z=pos
    ball('skull cranium',(x,y,z),(0.095*scale,0.074*scale,0.10*scale),BONE)
    for sign in [-1,1]:
        ball('deep eye socket',(x+sign*0.038*scale,y-0.064*scale,z+0.006*scale),
             (0.027*scale,0.016*scale,0.027*scale),DARK)
        ball('cheek bone',(x+sign*0.07*scale,y-0.033*scale,z-0.034*scale),
             (0.024*scale,0.032*scale,0.034*scale),BONE)
    box('jaw',(x,y-0.012*scale,z-0.093*scale),(0.106*scale,0.078*scale,0.045*scale),BONE,0.008*scale)
    for i in [-1,0,1]:
        box('teeth separation',(x+i*0.027*scale,y-0.054*scale,z-0.083*scale),
            (0.006*scale,0.008*scale,0.032*scale),DARK,0)


def candle(pos, height=0.2):
    x,y,z=pos
    obj=lathe('wax taper',[(0.023,z),(0.021,z+height)],BONE,12)
    obj.location.x=x;obj.location.y=y
    ball('small candle flame',(x,y,z+height+0.025),(0.014,0.014,0.033),EMBER)


def pointed_arch(x,y,z,w,h,mat=None):
    mat = mat or IRON
    tube('pointed gothic arch',[(x-w,y,z),(x-w,y,z+h*.52),
        (x-w*.68,y,z+h*.78),(x,y,z+h),(x+w*.68,y,z+h*.78),
        (x+w,y,z+h*.52),(x+w,y,z)],0.037,mat)


def chest(vault=False, small=False):
    w,d,h=(1.12,.70,.58) if vault else ((.62,.46,.40) if small else (1.05,.64,.53))
    z=.32 if vault else .07
    if vault:base(.76)
    for i in range(7):
        box('aged oak side plank',((i-3)*w/7,0,z+h/2),(w/7-.01,d,h),WOOD,.012)
    # Curved stave lid, constructed as narrow planks around an arch.
    for i in range(10):
        a=-math.pi/2+(i+.5)*math.pi/10
        zz=z+h+math.cos(a)*d*.30
        yy=math.sin(a)*d*.49
        stave=box('arched oak lid',(0,yy,zz),(w,.11 if not small else .075,.055),WOOD,.012)
        stave.rotation_euler.x=-a
    for x in [-w*.36,w*.36]:
        box('iron lower strap',(x,-d*.51,z+h*.47),(.065,.025,h*.95),IRON,.008)
        tube('iron arched lid binding',[(x,math.sin(a)*d*.51,z+h+math.cos(a)*d*.34)
             for a in [(-math.pi/2+j*math.pi/12) for j in range(13)]],.027,IRON)
        for zz in [z+.08,z+h-.06]:
            ball('brass rivet',(x,-d*.535,zz),(.02,.012,.02),BRASS)
    box('lock escutcheon',(0,-d*.55,z+h*.75),(.16,.045,.20),BRASS,.022)
    box('key hole',(0,-d*.58,z+h*.76),(.035,.01,.072),DARK,.004)
    for side in [-1,1]:
        handle=ring('side carrying loop',(side*w*.51,0,z+h*.53),.095,.016,IRON,True)
        handle.rotation_euler.z=math.pi/2
    if vault:
        skull((0,-d*.58,z+h+.04),.78)
        for x in [-w*.43,w*.43]:
            candle((x,0,z+h+d*.32),.12)


def altar():
    base(.75)
    for x in [-.30,.30]:
        for y in [-.22,.22]:
            box('altar carved foot',(x,y,.46),(.20,.20,.48),STONE,.035)
    box('sacrificial slab',(0,0,.76),(1.04,.75,.17),STONE,.045)
    box('brass slab rim',(0,0,.855),(.92,.64,.025),BRASS,.008)
    box('ritual black surface',(0,0,.875),(.84,.56,.025),DARK,.008)
    ring('blood channel',(0,0,.898),.19,.013,BLOOD)
    skull((0,-.38,.64),1.1)
    for x,y in [(-.38,-.20),(.38,-.20),(-.38,.20),(.38,.20)]:candle((x,y,.90),.16)


def shrine(kind):
    base(.58)
    if kind=='fortune':
        box('reliquary pedestal',(0,0,.40),(.55,.44,.36),STONE,.03)
        for y in [-.17,.17]:pointed_arch(0,y,.49,.27,.94,BRASS)
        for x in [-.24,.24]:
            tube('chapel column',[(x,-.17,.44),(x,-.17,1.00)],.045,IRON)
        ring('suspended ancient seal',(0,-.02,.91),.14,.034,BRASS,True)
        skull((0,-.048,.9),.95)
        for x in [-.33,.33]:candle((x,-.06,.23),.3)
    elif kind=='cursed':
        box('tombstone base',(0,0,.67),(.45,.22,.92),DARK,.04)
        for y in [-.13,.13]:pointed_arch(0,y,.27,.24,1.07,IRON)
        box('blood sigil vertical',(0,-.135,.81),(.025,.018,.32),BLOOD,.004)
        box('blood sigil arms',(0,-.135,.88),(.20,.018,.025),BLOOD,.004)
        skull((0,-.16,.45),1.1)
        for x in [-.30,.30]:
            tube('hanging shackle',[(x,0,1.06),(x,-.08,.72),(x,-.12,.34)],.017,IRON)
            ring('bound spirit shackle',(x,-.12,.32),.072,.017,IRON,True)
    else:
        glow=SOUL if kind=='storm' else EMBER
        lathe('carved pedestal',[(.32,.21),(.27,.34),(.23,.48),(.24,.58)],STONE,12)
        for i in range(4):
            a=i*TAU/4
            def v(r,z):return (math.cos(a)*r,math.sin(a)*r,z)
            tube('iron reliquary cage',[v(.30,.38),v(.31,.90),v(.24,1.20),v(.08,1.40)],.04,IRON)
        ring('cage lower collar',(0,0,.57),.28,.027,BRASS)
        ring('cage upper collar',(0,0,1.09),.27,.027,BRASS)
        crystal=lathe('restrained glowing relic',[(0,.58),(.13,.76),(.09,1.05),(0,1.19)],glow,6)
        for i in range(4):
            a=i*TAU/4
            skull((math.cos(a)*.24,math.sin(a)*.24,.47),.58)


def well(soul=False):
    base(.76 if soul else .65)
    lathe('sculpted basin',[(.40,.19),(.35,.30),(.42,.48),(.59,.67),(.61,.76),
        (.52,.76),(.49,.65),(.34,.42)],STONE)
    ring('rim binding',(0,0,.74),.57,.028,BRASS)
    lathe('still luminous water',[(.46,.64),(.46,.65)],SOUL)
    for i in range(6):
        a=i*TAU/6
        skull((math.cos(a)*.49,math.sin(a)*.49,.55),.58)
    if soul:
        for i in range(4):
            a=i*TAU/4
            tube('font arched brace',[(math.cos(a)*.48,math.sin(a)*.48,.36),
                (math.cos(a)*.60,math.sin(a)*.60,.93),
                (math.cos(a)*.44,math.sin(a)*.44,1.19)],.035,IRON)
        ball('bound soul',(0,0,.87),(.06,.06,.095),SOUL)


def obelisk():
    base(.48)
    shaft = lathe('weathered obelisk',[(.29,.20),(.23,.34),(.20,1.40),(0,1.78)],DARK,4)
    shaft.rotation_euler.z = math.pi / 4
    for i in range(5):
        z = .44 + i * .20
        face = -.166 + i * .004
        tube('incised rune chevron',[(-.04,face,z),(.025,face,z+.045),(-.035,face,z+.085)],.010,BLOOD)
    for x in [-.28,.28]:skull((x,-.13,.27),.65)


def urn():
    lathe('funerary amphora',[(.19,.03),(.20,.10),(.27,.21),(.31,.39),(.28,.58),
        (.16,.69),(.14,.76),(.19,.80),(.19,.84),(.13,.84),(.115,.76)],WOOD)
    for z,r in [(.13,.22),(.62,.245),(.79,.18)]:ring('antique bronze collar',(0,0,z),r,.018,BRASS)
    for sign in [-1,1]:
        tube('amphora handle',[(sign*.18,0,.71),(sign*.34,0,.73),(sign*.38,0,.57),(sign*.27,0,.44)],.031,BRASS)
    skull((0,-.28,.40),.7)


def book():
    cover=box('leather bound chronicle',(0,0,.08),(.57,.38,.07),WOOD,.02)
    for side in [-1,1]:
        pages=box('open parchment folio',(side*.13,0,.125),(.25,.33,.032),PARCHMENT,.006)
        pages.rotation_euler.y=side*-.1
        for i in range(5):
            box('ink manuscript line',(side*.13,-.11+i*.047,.149),(.18,.007,.003),DARK,0)
    box('crimson bookmark',(.08,-.20,.10),(.034,.18,.012),BLOOD,.002)
    for x in [-.25,.25]:
        for y in [-.15,.15]:box('brass cover corner',(x,y,.12),(.055,.055,.012),BRASS,.006)


def sword():
    # A forged blade and crossguard, preserving the shrine's upright orientation.
    verts=[(-.09,0,.20),(.09,0,.20),(-.065,0,1.1),(.065,0,1.1),(0,0,1.36),
           (0,-.034,.24),(0,-.025,1.09),(0,.034,.24),(0,.025,1.09)]
    faces=[(0,5,6,2),(1,3,6,5),(2,6,4),(6,3,4),(0,2,8,7),(1,7,8,3),(2,4,8),(3,8,4),(0,7,1,5)]
    mesh=bpy.data.meshes.new('forged diamond section blade');mesh.from_pydata(verts,[],faces)
    obj=bpy.data.objects.new('forged diamond section blade',mesh);bpy.context.collection.objects.link(obj);finish(obj,obj.name,IRON)
    tube('swept crossguard',[(-.28,0,.20),(-.17,0,.14),(0,0,.18),(.17,0,.14),(.28,0,.20)],.028,BRASS)
    lathe('leather hilt',[(.037,-.24),(.048,-.20),(.045,.14)],WOOD,16)
    for z in [-.18,-.13,-.08,-.03,.02,.07]:ring('hilt binding',(0,0,z),.046,.007,BRASS)
    ball('pommel', (0,0,-.25),(.064,.05,.068),BRASS)


def rift():
    # A fractured standing arch, not a flat luminous traffic sign.
    base(.62)
    for sign in [-1,1]:
        box('broken portal jamb',(sign*.51,0,.47),(.20,.24,.52),DARK,.028)
        for i in range(6):
            a=math.pi*.10+i*math.pi*.072
            x=sign*math.cos(a)*.51;z=.60+math.sin(a)*.65
            block=box('rift arch voussoir',(x,0,z),(.19,.22,.22),DARK,.025)
            block.rotation_euler.y=sign*a
        tube('ember seam',[(sign*.41,-.12,.48),(sign*.40,-.12,.84),
            (sign*.21,-.12,1.12),(0,-.12,1.23)],.013,EMBER)
    for x in [-.35,.35]:skull((x,-.15,.28),.70)


def ground_sigil(radius=.62, glow=None):
    glow=glow or SOUL
    for r in [radius,radius*.84]:ring('engraved circular seal',(0,0,.025),r,.012,glow)
    for i in range(12):
        a=i*TAU/12
        x,y=math.cos(a)*radius*.91,math.sin(a)*radius*.91
        obj=box('etched rune',(x,y,.028),(.045,.017,.009),glow,.002);obj.rotation_euler.z=a


def fissure(ember=False):
    glow=EMBER if ember else SOUL
    for i in range(11):
        a=i*TAU/11
        r=.40+random.random()*.14
        stone=box('fractured floor lip',(math.cos(a)*r,math.sin(a)*r,.10),
                  (.25,.18,.16),DARK,.025)
        stone.rotation_euler=(.1*(i%3),.13*(i%2),a)
        tube('luminous crack',[(0,0,.035),(math.cos(a)*r*.48,math.sin(a)*r*.48,.04),
             (math.cos(a+.2)*r*.89,math.sin(a+.2)*r*.89,.055)],.011,glow)
    for i in range(5):
        a=i*TAU/5
        ball('rising ember mote',(math.cos(a)*.19,math.sin(a)*.19,.18+i*.04),(.022,.022,.035),glow)


def loot_prism():
    # A tied leather treasure pouch and scattered coins rather than a floating
    # geometric prism / vertical tower. Rarity is handled by the runtime ring.
    lathe('worn leather treasure pouch',[(.11,.015),(.18,.08),(.19,.18),
        (.15,.28),(.07,.33),(.085,.37)],WOOD,24)
    ring('tied drawstring',(0,0,.325),.077,.012,BRASS)
    tube('drawstring knot',[(-.05,-.07,.33),(-.11,-.12,.31),
        (-.13,-.10,.25),(-.06,-.08,.325),(.04,-.10,.29)],.011,BRASS)
    for i in range(8):
        a=i*2.4;r=.16+i*.014
        coin=lathe('scattered antique coin',[(.047,0),(.047,.012)],BRASS,16)
        coin.location=(math.cos(a)*r,math.sin(a)*r,.014+(i%2)*.012)
        coin.rotation_euler=(.03*(i%3),.04,a)
        ring('coin minted edge',Vector(coin.location)+Vector((0,0,.013)),.035,.003,BRASS)


def restorative_pickup(fury=False):
    # Ground-centred, hand-sized reliquary phial: coloured enamel, iron cage,
    # engraved brass neck and cork. Silhouette stays readable without a glow ball.
    enamel = SOUL if fury else BLOOD
    lathe('enamel medicine vessel',[(.075,.02),(.13,.08),(.135,.20),
        (.10,.28),(.048,.32),(.048,.39)],enamel,24)
    lathe('iron foot',[(.078,.01),(.10,.035),(.095,.065)],IRON,24)
    for z,r in [(.08,.13),(.245,.117),(.33,.054),(.39,.06)]:
        ring('engraved brass collar',(0,0,z),r,.012,BRASS)
    for i in range(4):
        a=TAU*i/4
        tube('forged phial brace',[(math.cos(a)*r,math.sin(a)*r,z)
             for r,z in [(.095,.04),(.139,.13),(.13,.22),(.10,.28)]],.012,IRON)
    lathe('sealed cork',[(.043,.385),(.045,.43),(.039,.448)],WOOD,16)
    ring('carrying loop',(0,0,.475),.038,.009,BRASS,True)
    if fury:
        tube('lightning enamel crest',[(-.025,-.139,.23),(.017,-.143,.19),
             (-.014,-.143,.16),(.025,-.139,.115)],.009,BRASS)
    else:
        box('healer seal upright',(0,-.143,.18),(.018,.013,.095),BRASS,.003)
        box('healer seal crossbar',(0,-.147,.185),(.065,.013,.018),BRASS,.003)


def death_remains():
    # An irregular small scatter of discarded armour and bone, with empty gaps.
    # No solid backing disc, emission, or giant geometric marker.
    skull((-.12,-.07,.10),.76)
    for i in range(4):
        a=i*2.15+.35
        x,y=math.cos(a)*(.18+i*.026),math.sin(a)*(.15+i*.022)
        tube('broken bone',[(x-.06,y-.025,.027),(x,y,.035),
             (x+.065,y+.02,.023)],.018,BONE)
        ball('bone joint',(x-.06,y-.025,.027),(.026,.024,.022),BONE)
    plate=box('fallen armour plate',(.12,.06,.055),(.23,.18,.05),IRON,.017)
    plate.rotation_euler=(.18,.12,.63)
    tube('torn plate rim',[(.035,.01,.067),(.15,-.015,.066),
         (.225,.06,.057)],.012,BRASS)


GENERATORS={
    'health_pickup':restorative_pickup,'fury_pickup':lambda:restorative_pickup(True),
    'death_remains':death_remains,'sword':sword,'chest':chest,'altar':altar,'soul_font':lambda:well(True),
    'fortune_shrine':lambda:shrine('fortune'),'storm_shrine':lambda:shrine('storm'),
    'healing_well':well,'cursed_shrine':lambda:shrine('cursed'),'blood_obelisk':obelisk,
    'reliquary_vault':lambda:chest(vault=True),'ember_rift_prop':rift,
    'ashen_pylon':lambda:shrine('ember'),'lore_page':book,'breakable_urn':urn,
    'breakable_coffer':lambda:chest(small=True),'loot_prism':loot_prism,
    'objective_sigil':ground_sigil,'ember_vent':lambda:fissure(True),
    'soul_fissure':fissure,'boss_summon_portal':lambda:ground_sigil(1.25,BLOOD),
}


def export(name, semantic_name=None):
    bpy.ops.object.select_all(action='SELECT')
    # One object with a small stable material set; no hundreds of scene entities.
    for obj in list(bpy.context.selected_objects):
        if obj.type=='CURVE':
            bpy.context.view_layer.objects.active=obj
            bpy.ops.object.convert(target='MESH')
    bpy.ops.object.select_all(action='SELECT')
    bpy.context.view_layer.objects.active=bpy.context.selected_objects[0]
    bpy.ops.object.join()
    obj=bpy.context.object;obj.name=semantic_name or name
    obj.data.name=semantic_name or name
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=1.15192,island_margin=.02)
    bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.export_scene.gltf(filepath=str(OUT/f'{name}.glb'),export_format='GLB',
        use_selection=True,export_apply=True,export_yup=True,export_cameras=False,
        export_lights=False,export_animations=False)
    corners=[obj.matrix_world @ Vector(corner) for corner in obj.bound_box]
    return {'name':name,'vertices':len(obj.data.vertices),'faces':len(obj.data.polygons),
        'bounds_min':[min(c[i] for c in corners) for i in range(3)],
        'bounds_max':[max(c[i] for c in corners) for i in range(3)]}


def preview(name):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(OUT/f'{name}.glb'))
    raw = (OUT/f'{name}.glb').read_bytes()
    length = struct.unpack_from('<I', raw, 12)[0]
    document = json.loads(raw[20:20+length])
    for spec in document.get('materials', []):
        mat = bpy.data.materials.get(spec['name'])
        if mat is not None:
            mat.diffuse_color = spec.get('pbrMetallicRoughness', {}).get('baseColorFactor', [1,1,1,1])
    objects=[o for o in bpy.context.scene.objects if o.type=='MESH']
    corners=[o.matrix_world @ Vector(c) for o in objects for c in o.bound_box]
    lo=Vector([min(c[i] for c in corners) for i in range(3)])
    hi=Vector([max(c[i] for c in corners) for i in range(3)])
    center=(hi+lo)/2
    extent=max(hi-lo)
    bpy.ops.object.camera_add(location=center+Vector((2,-3,2.25))*extent)
    camera=bpy.context.object
    camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.type='ORTHO';camera.data.ortho_scale=extent*1.55
    scene=bpy.context.scene;scene.camera=camera
    scene.render.engine='BLENDER_WORKBENCH'
    scene.display.shading.light='STUDIO'
    scene.display.shading.studio_light='paint.sl'
    scene.display.shading.color_type='MATERIAL'
    scene.display.shading.show_shadows=True
    scene.display.shading.show_cavity=True
    scene.display.shading.cavity_type='BOTH'
    scene.display.shading.curvature_ridge_factor=1.5
    scene.display.shading.curvature_valley_factor=1.2
    scene.display.shading.background_type='WORLD'
    scene.world = bpy.data.worlds.new("Preview World")
    scene.world.color=(.045,.048,.055)
    scene.render.resolution_x=384;scene.render.resolution_y=384
    scene.render.resolution_percentage=100
    scene.render.image_settings.file_format='PNG'
    scene.render.filepath=str(PREVIEW/f'{name}.png')
    bpy.ops.render.render(write_still=True)


def verify_assets():
    report = []
    for name in GENERATORS:
        path = OUT/f'{name}.glb'
        data = path.read_bytes()
        assert data[:4] == b'glTF' and len(data) > 1000, name
        length = struct.unpack_from('<I', data, 12)[0]
        doc = json.loads(data[20:20+length])
        assert len(doc.get('scenes', [])) == 1, name
        primitives = [p for mesh in doc['meshes'] for p in mesh['primitives']]
        assert primitives and len(primitives) <= 8, (name, len(primitives))
        for material in doc.get('materials', []):
            assert material.get('alphaMode', 'OPAQUE') == 'OPAQUE', name
        report.append({'asset': str(path.relative_to(ROOT)), 'bytes': len(data),
            'meshes': len(doc['meshes']), 'material_primitives': len(primitives),
            'triangles': sum(doc['accessors'][p['indices']]['count']//3 for p in primitives),
            'materials': [m['name'] for m in doc['materials']]})
    (PREVIEW/'verified-assets.json').write_text(json.dumps(report, indent=2)+'\n')
    print(f'Verified {len(report)} GLBs: single scenes, <=8 material primitives, opaque materials.')


def main():
    args=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
    parser=argparse.ArgumentParser()
    parser.add_argument('--preview-only',action='store_true')
    parser.add_argument('--verify-only',action='store_true')
    parser.add_argument('--only',nargs='+')
    opts=parser.parse_args(args)
    PREVIEW.mkdir(parents=True,exist_ok=True)
    names=opts.only or list(GENERATORS)
    manifest=[]
    for name in names:
        if opts.verify_only:
            continue
        if opts.preview_only:preview(name)
        else:
            reset();GENERATORS[name]();manifest.append(export(name))
    if not opts.preview_only and not opts.verify_only:
        manifest_path = PREVIEW/'props-manifest.json'
        previous = json.loads(manifest_path.read_text()) if manifest_path.exists() else []
        by_name = {entry['name']: entry for entry in previous}
        by_name.update({entry['name']: entry for entry in manifest})
        manifest_path.write_text(json.dumps(list(by_name.values()),indent=2)+'\n')
    verify_assets()
    if opts.preview_only:
        subprocess.run([
            'magick', 'montage', *[str(PREVIEW/f'{name}.png') for name in names],
            '-set', 'label', '%t', '-font', str(ROOT/'assets/fonts/CrimsonPro.ttf'),
            '-pointsize', '20', '-fill', '#dac79e', '-background', '#222329',
            '-geometry', '320x320+7+20', '-tile', '5x', str(PREVIEW/'contact-sheet.png')
        ], check=True)


if __name__=='__main__':main()
