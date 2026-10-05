"""Author the complete Reliquary enemy cast with articulated, textured geometry.

blender --background --factory-startup --python tools/blender/generate_reliquary_enemies.py
Optional -- --preview renders an orthographic contact sheet to target/validation.
Character forward is Blender -Y; all feet meet Z=0. Four named pivot meshes
are the actual arms/legs animated by enemy.rs, never duplicate effect meshes.
"""
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).parent))
import generate_enemy_roster as old

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/models"
PREVIEW = ROOT / "target/validation/reliquary-enemies"
IRON, STEEL, BONE, CLOTH, GOLD = old.IRON, old.STEEL, old.BONE, old.CLOTH, old.GOLD
DARK, LEATHER, EMBER, SOUL, VIOLET = old.OBSIDIAN, old.LEATHER, old.EMBER, old.SOUL, old.VIOLET
# Keep emission accents small. Most of each silhouette is textured opaque PBR.
for m in (EMBER, SOUL, VIOLET, old.BLOOD):
    m.node_tree.nodes['Principled BSDF'].inputs['Emission Strength'].default_value = 0.65

# Existing stable hit-zone heights/scales. Gameplay tuning does not change.
CAST = [
    ('skeleton', 'skeleton', .86, 1.92*.86),
    ('ashbone_guard', 'guard', 1.02, 2.06),
    ('cultist', 'caster', .92, 1.92*.92),
    ('butcher', 'heavy', 1.18, 1.92*1.18),
    ('bone_stalker', 'stalker', .86, 1.58),
    ('ashen_marksman', 'marksman', .94, 1.94),
    ('treasure_imp', 'imp', .64, 1.28),
    ('seal_warden', 'warden', 1.05, 2.24),
    ('keeper', 'boss', 1.30, 2.78),
    ('nemesis', 'duelist', 1.02, 2.08),
    ('hollow_thrall', 'skeleton', .92, 1.92*.92),
    ('hollow_stalker', 'stalker', .88, 1.58),
    ('choir_acolyte', 'caster', .94, 1.92*.94),
    ('choir_marksman', 'marksman', .96, 1.99),
    ('hollow_reaver', 'heavy', 1.18, 1.92*1.18),
    ('bell_warden', 'bell', 1.08, 2.34),
    ('hollow_matron', 'matron', 1.34, 1.92*1.34),
    ('quartermaster', 'merchant', 1.0, 1.92),
]


def mesh(name, verts, faces, mat, smooth=True):
    data = bpy.data.meshes.new(name)
    data.from_pydata(verts, [], faces)
    data.update()
    ob = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(ob)
    old.finish_mesh(ob, mat, smooth)
    return ob


def rings(name, profile, mat, center=(0, 0, 0), pleats=0, ragged=0):
    """Closed sculpted contour, with elliptical rings and optional fabric folds."""
    verts, faces, n = [], [], 32
    for j, (z, rx, ry) in enumerate(profile):
        for i in range(n):
            a = math.tau * i / n
            fold = 1 + pleats * math.cos(a * 8)
            dz = ragged * (.5 + .5*math.cos(a*7)) if j == 0 else 0
            verts.append((center[0]+rx*math.cos(a)*fold,
                          center[1]+ry*math.sin(a)*fold, center[2]+z+dz))
    for j in range(len(profile)-1):
        for i in range(n):
            a, b = j*n+i, j*n+(i+1)%n
            faces.append((a, b, b+n, a+n))
    faces += [tuple(reversed(range(n))), tuple((len(profile)-1)*n+i for i in range(n))]
    return mesh(name, verts, faces, mat)


def tube(name, points, radius, mat):
    curve = bpy.data.curves.new(name, 'CURVE')
    curve.dimensions = '3D'
    curve.resolution_u = 2
    curve.bevel_depth, curve.bevel_resolution = radius, 1
    spline = curve.splines.new('POLY')
    spline.points.add(len(points)-1)
    for v,p in zip(spline.points, points):
        v.co = (*p,1)
    ob = bpy.data.objects.new(name,curve)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    bpy.context.view_layer.objects.active=ob
    ob.select_set(True)
    bpy.ops.object.convert(target='MESH')
    ob.select_set(False)
    return ob


def link(name, start, end, r1, r2, mat):
    a,b=Vector(start),Vector(end)
    ob=old.cone(name,(a+b)/2,r1,r2,(b-a).length,mat,16)
    ob.rotation_euler=(b-a).to_track_quat('Z','Y').to_euler()
    for poly in ob.data.polygons: poly.use_smooth=True
    return ob


def join_since(name, before, pivot):
    objects=[o for o in bpy.context.scene.objects if o.type=='MESH' and o not in before]
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects: o.select_set(True)
    bpy.context.view_layer.objects.active=objects[0]
    bpy.ops.object.join()
    ob=objects[0]
    ob.name=name
    bpy.context.scene.cursor.location=pivot
    bpy.ops.object.origin_set(type='ORIGIN_CURSOR')
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    return ob


def skull(z, accent, hood=False):
    old.sphere('cranium',(0,0,z),(.195,.167,.205),BONE)
    old.sphere('zygomatic left',(-.142,-.108,z-.056),(.064,.065,.069),BONE,16)
    old.sphere('zygomatic right',(.142,-.108,z-.056),(.064,.065,.069),BONE,16)
    for side in (-1,1):
        old.sphere('deep orbital socket',(side*.079,-.153,z+.008),(.066,.026,.064),DARK,16)
        old.sphere('buried eye',(side*.079,-.176,z+.009),(.018,.008,.018),accent,12)
        tube('heavy brow',[(side*.025,-.17,z+.063),(side*.082,-.171,z+.08),(side*.147,-.132,z+.071)],.025,BONE)
    mesh('nasal cavity',[(-.025,-.167,z-.034),(.025,-.167,z-.034),(0,-.182,z-.086)],[(0,1,2)],DARK,False)
    old.sphere('mandible',(0,-.07,z-.15),(.12,.105,.046),BONE,16)
    for i in range(7):
        old.sphere('ivory tooth',((i-3)*.029,-.17,z-.106),(.012,.018,.025),BONE,12)
    if hood:
        # Thick hood wall wraps the skull but leaves a real face opening.
        for row in range(3):
            points=[]
            for i in range(29):
                a=math.pi*.1 + math.pi*1.8*i/28
                points.append((.24*math.cos(a),.018+row*.043,z+.02+.28*math.sin(a)))
            tube('layered cowl hem',points,.052,CLOTH)
        rings('hood rear',[(z-.17,.19,.10),(z+.08,.24,.12),(z+.25,.15,.08),(z+.28,.035,.03)],CLOTH,(0,.10,0))


def blade(x, z, heavy=False):
    link('bound grip',(x,-.12,z-.25),(x,-.12,z+.03),.042,.037,LEATHER)
    for i in range(5): old.torus('grip binding',(x,-.12,z-.23+i*.045),.04,.007,GOLD,major_segments=12,minor_segments=4)
    tube('swept crossguard',[(x-.22,-.12,z-.015),(x-.13,-.15,z+.055),(x,-.15,z+.035),(x+.13,-.15,z+.055),(x+.22,-.12,z-.015)],.025,GOLD)
    width=.21 if heavy else .067
    verts=[(x-width,-.12,z+.05),(x+width,-.12,z+.05),(x+width*.8,-.12,z+.70),(x+.05,-.12,z+.89),(x-width,-.12,z+.77),(x,-.165,z+.40),(x,-.09,z+.40)]
    faces=[]
    for i in range(5): faces += [(i,(i+1)%5,5),((i+1)%5,i,6)]
    mesh('forged blade with central ridge',verts,faces,STEEL,False)


def shield(x):
    contour=[(-.26,.40),(.0,.48),(.26,.40),(.28,.0),(.16,-.28),(0,-.48),(-.16,-.28),(-.28,0)]
    verts=[(x+a,-.26,1.08+b) for a,b in contour]+[(x,-.35,1.08)]+[(x+a,-.18,1.08+b) for a,b in contour]
    faces=[(i,(i+1)%8,8) for i in range(8)]+[tuple(range(9,17))]
    faces += [(i,9+i,9+(i+1)%8,(i+1)%8) for i in range(8)]
    mesh('curved kite shield',verts,faces,IRON,False)
    tube('shield brass rim',[(x+a,-.27,1.08+b) for a,b in contour+[contour[0]]],.021,GOLD)
    tube('shield raised spine',[(x,-.355,.67),(x,-.365,1.08),(x,-.32,1.49)],.018,GOLD)
    old.sphere('shield boss',(x,-.36,1.1),(.075,.04,.075),STEEL)


def build(name, role, scale, height):
    old.reset()
    soul=name.startswith(('hollow','choir','bell'))
    accent=SOUL if soul else EMBER
    if role in ('warden','matron'): accent=VIOLET
    if role=='duelist': accent=old.BLOOD
    robe=role in ('caster','warden','bell','matron','merchant')
    skeletal=role in ('skeleton','stalker')
    bulky=role in ('heavy','boss','bell')
    width=1.2 if bulky else .9 if role in ('stalker','imp') else 1.0
    # All base artwork is authored at 1.9m, scaled once with its hip pivots.
    if skeletal:
        link('vertebral column',(0,.07,.82),(0,.07,1.48),.058,.062,BONE)
        for i in range(6):
            z=1.04+i*.066
            for side in (-1,1):
                tube('curved separate ribs',[(0,.075,z+.028),(side*.17,.02,z+.055),(side*.23,-.075,z+.018),(side*.17,-.157,z-.018),(side*.032,-.157,z-.026)],.024,BONE)
        rings('pelvic girdle',[(.77,.15,.11),(.81,.23,.135),(.9,.19,.12)],BONE)
    elif robe:
        rings('weighted pleated robe',[(.11,.39,.27),(.34,.34,.23),(.80,.24,.17),(1.0,.21,.15),(1.34,.31,.18),(1.43,.25,.14)],CLOTH,pleats=.07,ragged=.07)
        for s in (-1,1):
            tube('robe brocade seam',[(s*.30,-.22,.18),(s*.25,-.21,.5),(s*.17,-.17,1.02),(s*.19,-.19,1.32)],.009,GOLD)
        rings('braided waist sash',[(.90,.24,.17),(.96,.23,.16),(1.0,.235,.165)],LEATHER)
        old.sphere('reliquary clasp',(0,-.18,.96),(.056,.022,.071),GOLD)
    else:
        rings('tailored cuirass',[(.83,.23*width,.15),(.98,.25*width,.17),(1.18,.32*width,.21),(1.35,.34*width,.195),(1.44,.245*width,.135)],IRON)
        for i in range(4):
            rings('overlapping waist lames',[(.71+i*.054,.27*width,.18),(.765+i*.054,.245*width,.167)],STEEL if i%2==0 else IRON)
        tube('breastplate ridge',[(0,-.171,.98),(0,-.218,1.2),(0,-.19,1.41)],.014,GOLD)
        for side in (-1,1):
            tube('breastplate engraving',[(side*.22,-.17,1.05),(side*.25,-.205,1.22),(side*.15,-.198,1.35)],.01,GOLD)
        # Sculpted hanging tabard, deliberately split from articulated legs.
        mesh('torn front tabard',[(-.13,-.19,.93),(.13,-.19,.93),(.16,-.20,.46),(.05,-.225,.50),(-.04,-.21,.43),(-.16,-.2,.48)],[(0,1,2,3,4,5)],CLOTH,False)
        ob=bpy.context.scene.objects['torn front tabard']
        mod=ob.modifiers.new('cloth thickness','SOLIDIFY'); mod.thickness=.012
    skull(1.63,accent,hood=robe or role=='marksman')
    if not skeletal and not robe and role!='imp':
        rings('forged bascinet',[(1.59,.205,.177),(1.75,.206,.169),(1.84,.125,.11),(1.89,.025,.025)],IRON)
        for side in (-1,1):
            tube('temple brass edging',[(side*.19,-.085,1.52),(side*.2,-.11,1.7),(side*.14,-.11,1.82)],.013,GOLD)
    if role in ('boss','matron','warden','duelist'):
        old.torus('crown circlet',(0,0,1.79),.205,.025,GOLD)
        for i in range(7):
            a=i*math.tau/7
            link('swept crown thorn',(.20*math.cos(a),.17*math.sin(a),1.78),(.25*math.cos(a),.23*math.sin(a),2.0+(i%2)*.12),.037,.002,GOLD)
    for side in (-1,1):
        before=set(bpy.context.scene.objects)
        x=side*.15*width
        hip=(x,0,.83); knee=(x,-.025,.46); ankle=(x,.015,.12)
        if skeletal:
            link('femur',hip,knee,.055,.047,BONE)
            old.sphere('knee joint',knee,(.062,.055,.061),BONE,16)
            for dx in (-.023,.023): link('tibia fibula',(x+dx,-.025,.44),(x+dx,.015,.12),.022,.017,BONE)
            old.sphere('skeletal instep',(x,-.035,.068),(.065,.135,.055),BONE,16)
            for t in range(3): link('toe',(x+(t-1)*.043,-.09,.045),(x+(t-1)*.043,-.19,.037),.018,.012,BONE)
        else:
            link('fitted upper leg',hip,knee,.10,.077,LEATHER)
            old.sphere('knee cop',(x,-.079,.46),(.10,.06,.11),STEEL)
            rings('tapered greave',[(.12,.075,.075),(.29,.098,.085),(.43,.086,.074)],IRON,(x,0,0))
            old.sphere('pointed leather boot',(x,-.052,.076),(.088,.16,.075),LEATHER)
            tube('greave central ridge',[(x,-.078,.15),(x,-.10,.28),(x,-.08,.40)],.012,GOLD)
        join_since('enemy_left_leg' if side<0 else 'enemy_right_leg',before,hip)
    for side in (-1,1):
        before=set(bpy.context.scene.objects)
        x=side*.32*width
        shoulder=(x,0,1.34); elbow=(side*.405*width,-.008,1.04); hand=(side*.43*width,-.12,.81)
        link('upper arm',shoulder,elbow,.07 if skeletal else .104,.055 if skeletal else .079,BONE if skeletal else CLOTH)
        old.sphere('elbow',elbow,(.065,.071,.07),BONE if skeletal else STEEL,16)
        link('forearm',elbow,hand,.065 if skeletal else .082,.042 if skeletal else .055,BONE if skeletal else IRON)
        old.sphere('closed hand',hand,(.061,.065,.085),BONE if skeletal else LEATHER,16)
        if not skeletal:
            for layer in range(3):
                old.sphere('overlapping pauldron',(x+side*layer*.038,0,1.36-layer*.032),(.145-layer*.016,.195-layer*.023,.075),STEEL if layer==0 else IRON)
            if bulky:
                for i in range(3): link('pauldron thorn',(x+side*.10,-.09+i*.09,1.41),(x+side*.24,-.13+i*.12,1.57),.036,.001,GOLD)
        if side>0:
            if role in ('caster','warden','matron','merchant','bell'):
                link('crooked staff',(x+.14,-.12,.06),(x+.14,-.12,1.83),.032,.021,LEATHER)
                old.torus('relic staff cage',(x+.14,-.12,1.79),.127,.025,GOLD,(math.pi/2,0,0))
                old.sphere('small captive soul',(x+.14,-.12,1.79),(.055,.055,.072),accent)
            elif role=='marksman':
                link('crossbow stock',(x,-.03,.81),(x,-.68,.86),.048,.028,LEATHER)
                tube('recurve crossbow limbs',[(x-.34,-.50,.90),(x-.22,-.61,.88),(x,-.53,.86),(x+.22,-.61,.88),(x+.34,-.50,.90)],.027,STEEL)
                tube('bowstring',[(x-.34,-.50,.90),(x,-.17,.86),(x+.34,-.50,.90)],.006,LEATHER)
                link('loaded quarrel',(x,-.16,.89),(x,-.79,.89),.008,.005,STEEL)
            else: blade(x+.06,.88,heavy=bulky)
        elif role in ('guard','warden'): shield(x-.12)
        elif role in ('stalker','duelist'): blade(x-.03,.70)
        elif role=='bell':
            rings('bronze tolling bell',[(.66,.23,.23),(.72,.235,.235),(.77,.17,.17),(1.0,.11,.11),(1.09,.025,.025)],GOLD,(x-.14,-.09,0))
            old.sphere('bell clapper',(x-.14,-.09,.62),(.045,.045,.064),IRON)
        join_since('enemy_left_arm' if side<0 else 'enemy_right_arm',before,shoulder)
    if role in ('marksman','imp','merchant'):
        old.sphere('travel satchel',(-.15,.205,.97),(.22,.12,.31),LEATHER)
        for i in range(5):
            if role=='marksman': link('quiver arrow',(-.23+i*.035,.22,1.08),(-.20+i*.035,.25,1.59),.01,.007,STEEL)
            else: old.torus('satchel fastening',(-.30+i*.067,.31,1.07),.021,.008,GOLD,(math.pi/2,0,0),12,4)
    if role in ('stalker','imp'):
        for side in (-1,1):
            tube('curved horn',[(side*.14,.01,1.76),(side*.25,.035,1.87),(side*.28,.07,2.0),(side*.21,.10,2.07)],.033,BONE)
    # Merge remaining torso geometry: only five animated/body mesh nodes per enemy.
    static=[o for o in bpy.context.scene.objects if o.type=='MESH' and not o.name.startswith('enemy_')]
    join_since('enemy_body',set(bpy.context.scene.objects)-set(static),(0,0,0))
    factor=height/(2.15 if role in ('boss','matron','warden','duelist','stalker','imp') else 1.9)
    for ob in list(bpy.context.scene.objects):
        if ob.type=='MESH':
            ob.location*=factor
            ob.scale*=factor
            bpy.context.view_layer.objects.active=ob
            ob.select_set(True)
            bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
            ob.select_set(False)
    for ob in list(bpy.context.scene.objects):
        if ob.type != 'MESH': continue
        bpy.ops.object.select_all(action='DESELECT')
        ob.select_set(True)
        bpy.context.view_layer.objects.active=ob
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.mesh.select_all(action='SELECT')
        bpy.ops.uv.smart_project(island_margin=.015)
        bpy.ops.object.mode_set(mode='OBJECT')
    old.add_hit_rig(scale=scale,height=height,weapon_side=-1 if role=='bell' else 1)
    # Keep legacy authored hit-bone endpoints for the original chapter models.
    if name in ('skeleton', 'cultist', 'butcher', 'hollow_thrall',
                'choir_acolyte', 'hollow_reaver', 'hollow_matron'):
        hollow = name.startswith(('hollow', 'choir'))
        points = {
            'hit_head': ((0,0,1.55 if hollow else 1.65), (0,0,1.92)),
            'hit_chest': ((0,0,.72 if hollow else .78), (0,0,1.38 if hollow else 1.35)),
            'hit_weapon': ((.58,-.10 if hollow else -.14,.50 if hollow else .42),
                           (.98 if hollow else .9,-.16 if hollow else -.22,1.52 if hollow else 1.32)),
            'hit_left': ((-.46 if hollow else -.42,0,.68 if hollow else .72),
                         (-.62 if hollow else -.58,0,1.30 if hollow else 1.32)),
            'hit_right': ((.46 if hollow else .42,0,.68 if hollow else .72),
                          (.62 if hollow else .58,0,1.30 if hollow else 1.32)),
        }
        rig = bpy.data.objects['hit_bones']
        bpy.context.view_layer.objects.active=rig
        bpy.ops.object.mode_set(mode='EDIT')
        for key,(head,tail) in points.items():
            rig.data.edit_bones[key].head=Vector(head)*scale
            rig.data.edit_bones[key].tail=Vector(tail)*scale
        bpy.ops.object.mode_set(mode='OBJECT')
        for key,(_,tail) in points.items():
            for suffix in ('_marker','_contact_zone'):
                bpy.data.objects[key+suffix].location=Vector(tail)*scale
    old.export(name)


def preview(name):
    PREVIEW.mkdir(parents=True,exist_ok=True)
    scene=bpy.context.scene
    scene.render.engine='CYCLES'
    scene.cycles.samples=12
    scene.render.resolution_x=480
    scene.render.resolution_y=520
    scene.render.resolution_percentage=100
    scene.world.color=(.17,.17,.17)
    bpy.ops.mesh.primitive_plane_add(size=200,location=(0,0,-.025))
    floor=old.material('preview floor',(.045,.055,.065),0,.9)
    bpy.context.object.data.materials.append(floor)
    for loc,energy,size in [((3,-4,6),650,5),((-3,-1,3),350,4),((1,3,5),800,3)]:
        bpy.ops.object.light_add(type='AREA',location=loc)
        ob=bpy.context.object; ob.data.energy=energy; ob.data.shape='DISK'; ob.data.size=size
        ob.rotation_euler=(Vector((0,0,1))-ob.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=(3,-6,3.4))
    cam=bpy.context.object
    cam.rotation_euler=(Vector((0,0,1.25))-cam.location).to_track_quat('-Z','Y').to_euler()
    cam.data.type='ORTHO'; cam.data.ortho_scale=3.6
    scene.camera=cam
    scene.render.filepath=str(PREVIEW/f'{name}.png')
    bpy.ops.render.render(write_still=True)


if __name__=='__main__':
    for name,role,scale,height in CAST:
        if '--preview-only' in sys.argv:
            old.reset()
            bpy.ops.import_scene.gltf(filepath=str(OUT/f'{name}.glb'))
        else:
            build(name,role,scale,height)
        if '--preview' in sys.argv or '--preview-only' in sys.argv: preview(name)
