"""Author the Reliquary Knight body and its sole runtime-animated limb set.

blender --background --factory-startup --python tools/blender/generate_hero.py
Postprocess only hero.glb, hero_arm.glb, hero_leg.glb, hero_mantle.glb afterward.
The body deliberately contains no arms, legs or weapon. Limbs are centered,
pose-sized parts; player_limb_pose owns their placement and animation.
"""
import math
from pathlib import Path
import bpy
from mathutils import Vector, Quaternion

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'assets/models'
PREVIEW = ROOT / 'target/validation/hero'


def reset():
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)


def material(name, color, metal=0., roughness=.5, emission=0.):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    bs = m.node_tree.nodes.get('Principled BSDF')
    bs.inputs['Base Color'].default_value = (*color, 1)
    bs.inputs['Metallic'].default_value = metal
    bs.inputs['Roughness'].default_value = roughness
    if emission:
        bs.inputs['Emission Color'].default_value = (*color, 1)
        bs.inputs['Emission Strength'].default_value = emission
    return m


STEEL = material('cold worn steel', (.24, .29, .33), .8, .31)
EDGE = material('matron silver', (.56, .59, .59), .82, .28)
GOLD = material('dull relic gold', (.52, .31, .10), .72, .37)
DARK = material('charcoal cloth', (.035, .041, .046), 0., .9)
CLOTH = material('choir cloth', (.17, .023, .027), 0., .94)
LEATHER = material('old leather', (.085, .045, .025), .05, .82)
EMBER = material('ember glow', (.95, .22, .045), .0, .4, 1.5)


def finish(obj, name, mat, bevel=0.):
    obj.name = name
    obj.data.materials.append(mat)
    if bevel:
        mod = obj.modifiers.new('hand-forged edges', 'BEVEL')
        mod.width = bevel
        mod.segments = 3
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    for face in obj.data.polygons:
        face.use_smooth = True
    if not obj.data.uv_layers:
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.select_all(action='DESELECT')
        obj.select_set(True)
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.mesh.select_all(action='SELECT')
        bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=.03)
        bpy.ops.object.mode_set(mode='OBJECT')
    return obj


def ellipsoid(name, loc, size, mat):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=16, location=loc)
    obj = bpy.context.object
    obj.scale = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return finish(obj, name, mat)


def loft(name, rings, mat, sides=20, bevel=.012):
    """Closed elliptical section armor, rings = (height, width, depth, front offset)."""
    vertices = [(rx*math.cos(2*math.pi*i/sides), cy+ry*math.sin(2*math.pi*i/sides), z)
                for z, rx, ry, cy in rings for i in range(sides)]
    faces = [tuple(reversed(range(sides)))]
    for r in range(len(rings)-1):
        for i in range(sides):
            a=r*sides+i; b=r*sides+(i+1)%sides
            faces.append((a,b,b+sides,a+sides))
    faces.append(tuple((len(rings)-1)*sides+i for i in range(sides)))
    mesh=bpy.data.meshes.new(name); mesh.from_pydata(vertices, [], faces); mesh.update()
    obj=bpy.data.objects.new(name,mesh); bpy.context.collection.objects.link(obj)
    return finish(obj,name,mat,bevel)


def rod(name, a, b, radius, mat, vertices=12):
    delta=Vector(b)-Vector(a)
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=delta.length,
                                      location=(Vector(a)+Vector(b))*.5)
    obj=bpy.context.object
    obj.rotation_euler=delta.to_track_quat('Z','Y').to_euler()
    return finish(obj,name,mat,.005)


def plate(name, points, mat, thick=.022):
    n=len(points)
    verts=points+[(x,y+thick,z) for x,y,z in points]
    faces=[tuple(reversed(range(n))),tuple(range(n,2*n))]
    faces += [(i,(i+1)%n,(i+1)%n+n,i+n) for i in range(n)]
    mesh=bpy.data.meshes.new(name);mesh.from_pydata(verts,[],faces);mesh.update()
    obj=bpy.data.objects.new(name,mesh);bpy.context.collection.objects.link(obj)
    return finish(obj,name,mat,.008)


def trim_ring(name,z,rx,ry,cy=0.,mat=GOLD,radius=.012):
    for i in range(32):
        a=2*math.pi*i/32;b=2*math.pi*(i+1)/32
        rod(name,(rx*math.cos(a),cy+ry*math.sin(a),z),
            (rx*math.cos(b),cy+ry*math.sin(b),z),radius,mat,8)


def hit_bones():
    arm=bpy.data.armatures.new('hit_bones');obj=bpy.data.objects.new('hit_bones',arm)
    bpy.context.collection.objects.link(obj);bpy.context.view_layer.objects.active=obj
    bpy.ops.object.select_all(action='DESELECT');obj.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT')
    coords={'hit_head':((0,0,1.64),(0,0,1.91)),
            'hit_chest':((0,0,.8),(0,0,1.36)),
            'hit_weapon':((.58,-.14,.42),(.9,-.22,1.32)),
            'hit_left':((-.42,0,.72),(-.58,0,1.32)),
            'hit_right':((.42,0,.72),(.58,0,1.32))}
    for name,(a,b) in coords.items():
        bone=arm.edit_bones.new(name);bone.head=a;bone.tail=b
    bpy.ops.object.mode_set(mode='OBJECT')
    # Metadata-only contacts must never become extra visible spheres or discs.
    for name,(_,pos) in coords.items():
        for suffix in ['_marker','_contact_zone']:
            empty=bpy.data.objects.new(name+suffix,None)
            bpy.context.collection.objects.link(empty);empty.location=pos


def export(name):
    # Merge only rendered meshes by material, leaving the hit rig and contacts
    # untouched. Detailed rivets must not become hundreds of draw calls.
    for mat in list(bpy.data.materials):
        group=[o for o in bpy.context.scene.objects if o.type=='MESH' and
               o.data.materials and o.data.materials[0]==mat]
        if not group:
            continue
        bpy.ops.object.select_all(action='DESELECT')
        for obj in group: obj.select_set(True)
        bpy.context.view_layer.objects.active=group[0]
        bpy.ops.object.join()
        group[0].name=name+' '+mat.name
        group[0]['hero_part']=name
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.export_scene.gltf(filepath=str(OUT/(name+'.glb')),export_format='GLB',
        use_selection=True,export_apply=True,export_yup=True,export_extras=True)
    print('EXPORTED',name)


def body():
    reset()
    loft('quilted arming doublet',[(.75,.25,.17,0),(1.05,.29,.19,0),(1.39,.34,.22,0),(1.51,.23,.17,0)],DARK)
    loft('fluted breastplate',[(.93,.24,.185,-.005),(1.10,.31,.215,-.018),
         (1.30,.355,.23,-.012),(1.43,.31,.205,0),(1.47,.22,.15,0)],STEEL)
    for side in [-1,1]:
        plate('gilded breastplate chevron',[(side*.035,-.259,1.18),(side*.28,-.191,1.35),
              (side*.31,-.173,1.37),(side*.035,-.271,1.22)],GOLD)
        for i in range(3):
            rod('chest raised fluting',(side*(.05+i*.065),-.236,1.07),
                (side*(.08+i*.085),-.225+i*.013,1.32),.009,EDGE)
    for i in range(3):
        z=.87+i*.075
        loft('overlapping fauld plate '+str(i),[(z,.285,.205,0),(z+.06,.275,.195,0)],STEEL)
        trim_ring('fauld brass edging',z,.285,.205)
    loft('leather sword belt',[(.91,.289,.211,0),(.969,.289,.211,0)],LEATHER)
    plate('reliquary belt buckle',[(-.075,-.239,.91),(.075,-.239,.91),(.066,-.239,.99),(-.066,-.239,.99)],GOLD)
    ellipsoid('buckle carnelian',(0,-.258,.953),(.03,.012,.025),EMBER)
    loft('raised gorget',[(1.44,.225,.16,0),(1.57,.19,.15,0)],STEEL)
    trim_ring('gorget gold lip',1.56,.193,.153)
    ellipsoid('helmet skull',(0,.005,1.755),(.25,.223,.284),STEEL)
    # Angular enclosed visor and nose ridge, not a featureless sphere.
    for side in [-1,1]:
        plate('enclosed angular visor',[(0,-.249,1.72),(side*.213,-.14,1.75),
             (side*.184,-.14,1.60),(side*.073,-.23,1.53),(0,-.267,1.60)],EDGE)
        rod('brow gilding',(side*.018,-.241,1.815),(side*.205,-.142,1.795),.018,GOLD)
        rod('shadowed eye slit',(side*.035,-.254,1.776),(side*.181,-.184,1.778),.021,DARK)
        rod('ember eye glint',(side*.06,-.269,1.78),(side*.143,-.213,1.78),.005,EMBER)
        for j in range(3):
            x=side*(.055+.035*j)
            rod('visor breathing slot',(x,-.255+abs(x)*.35,1.615),(x,-.25+abs(x)*.35,1.66),.009,DARK)
    rod('helmet center ridge',(0,-.223,1.61),(0,-.246,1.9),.018,GOLD)
    plate('helmet crest',[(0,-.10,1.96),(0,.08,2.065),(0,.23,1.86)],GOLD,.036)
    # Two short split cloth tassets leave both moving greaves fully visible.
    for side in [-1,1]:
        plate('split crimson tasset',[(side*.04,-.195,.87),(side*.23,-.16,.86),
             (side*.26,-.17,.56),(side*.16,-.215,.50),(side*.07,-.23,.57)],CLOTH)
        rod('tasset gold stitching',(side*.07,-.222,.81),(side*.10,-.248,.59),.007,GOLD)
    # Closed, ribbed flowing cape with a scalloped hem, attached behind torso.
    cols=12; rows=7; verts=[]
    for r in range(rows):
        t=r/(rows-1)
        for c in range(cols+1):
            u=c/cols*2-1
            verts.append((u*(.26+.15*t), .27+.14*t+.028*math.cos(c*math.pi/2),
                          1.49-1.02*t + (.045*math.cos(c*math.pi/2) if r==rows-1 else 0)))
    faces=[]
    for r in range(rows-1):
        for c in range(cols):
            a=r*(cols+1)+c;faces.append((a,a+cols+1,a+cols+2,a+1))
    mesh=bpy.data.meshes.new('scalloped cloak');mesh.from_pydata(verts,[],faces);mesh.update()
    cloak=bpy.data.objects.new('crimson pilgrim cloak',mesh);bpy.context.collection.objects.link(cloak)
    finish(cloak,'crimson pilgrim cloak',CLOTH)
    solid=cloak.modifiers.new('cloth thickness','SOLIDIFY');solid.thickness=.018
    bpy.context.view_layer.objects.active=cloak;bpy.ops.object.modifier_apply(modifier=solid.name)
    hit_bones();export('hero')


def arm():
    reset()
    # Source Z maps to game Y. Normalize around center for runtime pose scaling.
    loft('arm mail sleeve',[(-.30,.09,.09,0),(.27,.105,.1,0),(.36,.075,.075,0)],DARK)
    loft('upper arm articulated plate',[(.025,.10,.095,0),(.27,.12,.10,0),(.34,.09,.08,0)],STEEL)
    ellipsoid('elbow couter',(0,-.035,-.005),(.117,.11,.10),EDGE)
    loft('vambrace', [(-.26,.075,.07,0),(-.20,.095,.085,0),(-.065,.115,.095,0)],STEEL)
    trim_ring('vambrace cuff',-.25,.083,.077,mat=GOLD,radius=.012)
    ellipsoid('closed gauntlet',(0,-.008,-.325),(.092,.082,.095),STEEL)
    for i in range(4):
        rod('gauntlet knuckle',( -.060+i*.04,-.077,-.30),(-.060+i*.04,-.076,-.36),.012,EDGE)
    normalize((.24,.22,.665), z_center=.0075);export('hero_arm')


def leg():
    reset()
    loft('leg arming hose',[(-.30,.073,.085,0),(.31,.095,.09,0),(.39,.10,.085,0)],DARK)
    loft('cuissart thigh plate',[(.12,.092,.085,0),(.36,.11,.10,0),(.42,.085,.08,0)],STEEL)
    ellipsoid('knee poleyn',(0,-.045,.075),(.12,.105,.095),EDGE)
    loft('ridged steel greave',[(-.28,.075,.075,0),(-.16,.095,.075,0),(.01,.108,.09,0)],STEEL)
    rod('greave center ridge',(0,-.083,-.26),(0,-.11,.02),.013,GOLD)
    ellipsoid('armored sabaton',(0,-.065,-.345),(.108,.165,.065),STEEL)
    for i in range(4):
        y=-.015-i*.049
        rod('sabaton overlapping bands',(-.087,y,-.308),(.087,y,-.308),.009,EDGE)
    trim_ring('greave ankle rim',-.27,.08,.08,radius=.01)
    normalize((.25,.36,.66));export('hero_leg')


def mantle():
    reset()
    for side in [-1,1]:
        for layer in range(3):
            ellipsoid('layered pauldron', (side*(.39+.025*layer),.025,.045-layer*.04),
                      (.21-layer*.022,.245-layer*.012,.135-layer*.013),STEEL)
        for i in range(3):
            ellipsoid('pauldron brass rivet',(side*(.35+.05*i),-.198,-.013),(.018,.012,.018),GOLD)
        rod('pauldron bright ridge',(side*.28,-.12,.14),(side*.49,.02,.15),.016,GOLD)
    normalize((1.2,.53,.29));export('hero_mantle')


def normalize(size, z_center=0.):
    # Runtime scales are (width,height,depth); Blender coordinates (width,depth,height).
    for obj in bpy.context.scene.objects:
        if obj.type=='MESH':
            bpy.ops.object.select_all(action='DESELECT')
            obj.select_set(True);bpy.context.view_layer.objects.active=obj
            bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
            for vertex in obj.data.vertices:
                vertex.co.x/=size[0];vertex.co.y/=size[1];vertex.co.z/=size[2]
            obj.location.x/=size[0];obj.location.y/=size[1];obj.location.z=(obj.location.z-z_center)/size[2]


def preview():
    reset(); PREVIEW.mkdir(parents=True,exist_ok=True)
    # Assemble the exact exported pieces, including postprocessing if rerun.
    def part(name,location,scale,rotation=(0,0,0)):
        bpy.ops.object.select_all(action='DESELECT')
        bpy.ops.import_scene.gltf(filepath=str(OUT/(name+'.glb')))
        objects=list(bpy.context.selected_objects)
        pivot=bpy.data.objects.new('preview '+name,None);bpy.context.collection.objects.link(pivot)
        for obj in objects:
            if obj.parent is None:obj.parent=pivot
        pivot.location=location;pivot.scale=scale;pivot.rotation_euler=rotation
    part('hero',(0,0,0),(1,1,1))
    part('hero_mantle',(0,-.03,1.43),(1.02,.34,.15))
    for shoulder,hand in [((-.42,-.03,1.43),(-.49,-.08,.89)),((.42,-.03,1.43),(.50,-.23,.94))]:
        a=Vector(shoulder);b=Vector(hand);delta=a-b
        part('hero_arm',(a+b)*.5,(.20,.18,delta.length),delta.to_track_quat('Z','Y').to_euler())
    part('hero_leg',(-.30,.05,.42),(.20,.22,.66),(0,-.08,0))
    part('hero_leg',(.30,.05,.42),(.20,.22,.66),(0,.08,0))
    sword_rotation = (Quaternion((1,0,0),-.58) * Quaternion((0,0,1),.22)
                      * Quaternion((0,1,0),.95)).to_euler()
    part('sword',(.50,-.23,.94),(.92,.92,.92),sword_rotation)
    bpy.ops.mesh.primitive_plane_add(size=200)
    ground=bpy.context.object;ground.name='preview ground';ground.data.materials.append(DARK)
    scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=24
    scene.cycles.use_denoising=True
    scene.world.color=(.20,.20,.20)
    for loc,power,size in [((3,-4,6),650,4),((-3,-1,4),450,3),((1,3,4),850,3)]:
        bpy.ops.object.light_add(type='AREA',location=loc);lamp=bpy.context.object
        lamp.data.energy=power;lamp.data.shape='DISK';lamp.data.size=size
        lamp.rotation_euler=(Vector((0,0,1))-lamp.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add();camera=bpy.context.object;scene.camera=camera
    camera.data.type='ORTHO';camera.data.ortho_scale=2.75
    scene.render.resolution_x=650;scene.render.resolution_y=800;scene.render.resolution_percentage=100
    for name,loc in [('front',(3,-6,3)),('rear',(-3,6,3)),('profile',(6,-1,2.8))]:
        camera.location=loc;camera.rotation_euler=(Vector((0,0,1))-camera.location).to_track_quat('-Z','Y').to_euler()
        scene.render.filepath=str(PREVIEW/(name+'.png'));bpy.ops.render.render(write_still=True)
    import numpy as np
    tiles=[]
    for name in ['front','rear','profile']:
        tile=bpy.data.images.load(str(PREVIEW/(name+'.png')),check_existing=False)
        pixels=np.empty(650*800*4,dtype=np.float32);tile.pixels.foreach_get(pixels)
        tiles.append(pixels.reshape(800,650,4))
    sheet=bpy.data.images.new('hero contact sheet',width=1950,height=800)
    sheet.pixels.foreach_set(np.concatenate(tiles,axis=1).ravel())
    sheet.filepath_raw=str(PREVIEW/'contact-sheet.png');sheet.file_format='PNG';sheet.save()
    print('PREVIEW',PREVIEW)


if __name__=='__main__':
    import sys
    if '--preview-only' not in sys.argv:
        body();arm();leg();mantle()
    if '--preview' in sys.argv or '--preview-only' in sys.argv:
        preview()
