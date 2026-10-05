"""Authored wilderness silhouettes replacing runtime primitive stand-ins.
Run with Blender --background --python this_file. Deterministic seed 41.
"""
import math
import random
from pathlib import Path
import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'assets/models'
PREVIEW = ROOT / 'target/validation/asset-replacement'
PREVIEW.mkdir(parents=True, exist_ok=True)
rng = random.Random(41)


def material(name, color, metallic=0.0, emission=0.0):
    m = bpy.data.materials.new(name)
    m.diffuse_color = (*color, 1)
    m.use_nodes = True
    b = m.node_tree.nodes.get('Principled BSDF')
    b.inputs['Base Color'].default_value = (*color, 1)
    b.inputs['Metallic'].default_value = metallic
    b.inputs['Roughness'].default_value = .78
    b.inputs['Emission Color'].default_value = (*color, 1)
    b.inputs['Emission Strength'].default_value = emission
    return m

STONE = material('reliquary stone', (.22, .24, .25))
DARK = material('hollow obsidian', (.085, .10, .12))
BARK = material('old leather', (.16, .10, .065))
IRON = material('cold worn steel', (.15, .18, .20), .65)
GOLD = material('dull relic gold', (.39, .27, .12), .55)
GLOW = material('bound ember', (.75, .17, .035), .05, .65)
CRYSTAL = material('hollow crystal', (.08, .27, .32), .2, .25)


def finish(obj, name, mat):
    obj.name = name
    obj.data.materials.append(mat)
    return obj


def block(name, location, size, mat, bevel=.035):
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.object
    obj.dimensions = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    finish(obj, name, mat)
    if bevel:
        mod = obj.modifiers.new('chipped masonry edges', 'BEVEL')
        mod.width = bevel
        mod.segments = 2
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def branch(name, a, b, r1, r2, mat=BARK):
    a, b = Vector(a), Vector(b)
    bpy.ops.mesh.primitive_cone_add(vertices=7, radius1=r1, radius2=r2, depth=(b-a).length, location=(a+b)*.5)
    obj = bpy.context.object
    obj.rotation_euler = (b-a).to_track_quat('Z', 'Y').to_euler()
    return finish(obj, name, mat)


def rubble(name, loc, scale, mat=STONE):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=1, location=loc)
    obj = bpy.context.object
    for v in obj.data.vertices:
        v.co *= rng.uniform(.82, 1.14)
        v.co.z = max(v.co.z, -.55)
    obj.scale = scale
    obj.rotation_euler.z = rng.random()*math.tau
    return finish(obj, name, mat)


def rock():
    rubble('split ridge stone', (0, 0, .64), (.98, .80, .98))
    rubble('weathered ledge', (.58, .08, .35), (.66, .53, .54), DARK)
    for i in range(6):
        a = i*math.tau/6
        rubble('talus fragment', (.87*math.cos(a), .68*math.sin(a), .08), (.15, .19, .15))


def tree():
    trunk = [(0, 0, 0), (.12, .06, .8), (-.12, .08, 1.65), (.08, .05, 2.5), (.30, .13, 3.15)]
    for i in range(4):
        branch('twisted dead trunk', trunk[i], trunk[i+1], .24-i*.047, .19-i*.044)
    for i in range(7):
        a = i*2.4
        z = .7+i*.23
        start = (.04, .06, z)
        elbow = (.65*math.cos(a), .65*math.sin(a), z+.24)
        tip = (1.08*math.cos(a+.15), 1.08*math.sin(a+.15), z+.66)
        branch('crooked bough', start, elbow, .105, .065)
        branch('dead finger branch', elbow, tip, .065, .018)
        fork = (tip[0]+.18, tip[1]-.15, tip[2]+.32)
        branch('forked twig', tip, fork, .023, .004)
    for i in range(6):
        a=i*math.tau/6
        branch('exposed root', (.08, .02, .28), (.67*math.cos(a), .67*math.sin(a), .03), .14, .025)


def crystals():
    for i in range(7):
        a=i*2.4
        radius=.34 if i else 0
        x,y=math.cos(a)*radius, math.sin(a)*radius
        h=1.8 if i==0 else rng.uniform(.65,1.3)
        # Hexagonal shaft and asymmetric pointed crown, rather than a traffic cone.
        branch('relic crystal shaft', (x,y,.08), (x+.09,y,h*.75), .13, .15, CRYSTAL)
        branch('crystal termination', (x+.09,y,h*.75), (x+.14,y,h), .15, 0, CRYSTAL)
    for i in range(5):
        a=i*1.4
        rubble('crystal bedrock', (.43*math.cos(a),.38*math.sin(a),.06),(.25,.22,.18),DARK)


def ruin():
    # Broken archivolt and coursed masonry; unit-scale base for streamed ruins.
    for side in [-1,1]:
        for row in range(4):
            block('worn pier course', (side*.38,0,.1+row*.19),(.23,.38,.18),STONE,.018)
        block('pier plinth',(side*.38,0,.055),(.36,.49,.11),DARK,.025)
    for i in range(9):
        a=math.pi*i/8
        if i==2: continue
        stone=block('broken arch voussoir',(.38*math.cos(a),0,.70+.38*math.sin(a)),(.17,.34,.18),STONE,.014)
        stone.rotation_euler.y=a-math.pi/2
    for i in range(4):
        rubble('fallen arch fragments',(rng.uniform(-.7,.7),rng.uniform(-.38,.3),.035),(.14,.13,.08),DARK)


def gate():
    for z,w,d,h in [(.16,1.2,.9,.32),(.40,.94,.70,.18),(2.13,.65,.52,3.3),(3.87,.86,.70,.18)]:
        block('sanctum carved masonry',(0,0,z),(w,d,h),DARK,.055)
    for side in [-1,1]:
        block('iron buttress',(side*.34,0,2.1),(.11,.58,3.15),IRON,.025)
        for z in [.75,1.5,2.25,3.0]:
            block('brass binding',(side*.34,-.32,z),(.16,.07,.16),GOLD,.018)
    block('recessed seal tablet',(0,-.272,2.1),(.43,.028,2.75),STONE,.012)
    for i in range(7):
        z=1.1+i*.30
        rune=block('ember incised rune',(0,-.292,z),(.22,.022,.025),GLOW,.006)
        rune.rotation_euler.y=(-1 if i%2 else 1)*.5
    # Pointed double finial around a bound cinder, not a glowing cone.
    for side in [-1,1]:
        branch('gothic finial',(side*.27,0,3.93),(side*.08,0,4.6),.14,.025,IRON)
    rubble('bound cinder',(0,0,4.17),(.11,.12,.20),GLOW)


BUILDERS={'scenery_rock':rock,'scenery_dead_tree':tree,'scenery_crystal':crystals,'scenery_ruin':ruin,'sanctum_gate':gate}
PREVIEWS=[]
for name,builder in BUILDERS.items():
    bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
    builder()
    # One mesh primitive per material; baked geometry keeps streamed scenes inexpensive.
    for mat in list(bpy.data.materials):
        objs=[o for o in bpy.context.scene.objects if o.type=='MESH' and o.active_material==mat]
        if not objs: continue
        bpy.ops.object.select_all(action='DESELECT')
        for o in objs:o.select_set(True)
        bpy.context.view_layer.objects.active=objs[0]
        bpy.ops.object.join()
        o=bpy.context.object
        bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
        bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_all(action='SELECT')
        bpy.ops.uv.smart_project(island_margin=.02)
        bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.export_scene.gltf(filepath=str(OUT/f'{name}.glb'),export_format='GLB',export_yup=True)
    for o in list(bpy.context.scene.objects):
        clone=o.copy(); clone.data=o.data.copy(); PREVIEWS.append((name,clone))

bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
for name,obj in PREVIEWS:
    bpy.context.collection.objects.link(obj)
    obj.location.x += list(BUILDERS).index(name)*3.7-7.4
bpy.ops.mesh.primitive_plane_add(size=50)
bpy.context.object.data.materials.append(DARK)
bpy.ops.object.camera_add(location=(10,-19,12))
cam=bpy.context.object; cam.rotation_euler=(Vector((0,0,1.3))-cam.location).to_track_quat('-Z','Y').to_euler(); cam.data.type='ORTHO'; cam.data.ortho_scale=22
bpy.context.scene.camera=cam
for loc,power,size in [((0,-6,10),2100,12),((5,5,8),1700,10)]:
    bpy.ops.object.light_add(type='AREA',location=loc); o=bpy.context.object; o.data.energy=power; o.data.shape='DISK';o.data.size=size;o.rotation_euler=(-o.location).to_track_quat('-Z','Y').to_euler()
s=bpy.context.scene;s.render.engine='CYCLES';s.cycles.samples=24;s.render.resolution_x=1600;s.render.resolution_y=800;s.render.resolution_percentage=100;s.world.color=(.15,.15,.15);s.render.filepath=str(PREVIEW/'environment.png');bpy.ops.render.render(write_still=True)
