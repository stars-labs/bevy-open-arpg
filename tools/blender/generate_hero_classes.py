"""Author the Stalker and Embercaller hero classes, their weapons and projectiles.

blender --background --factory-startup --python tools/blender/generate_hero_classes.py
blender --background --factory-startup --python tools/blender/postprocess_glb_materials.py \
    -- --only <the filenames in OUTPUTS>
Pass `-- --preview-only` after postprocessing to render the comparison sheets,
or `-- --only NAME [...]` to rebuild a subset.

The part contract is the knight's (generate_hero.py): a body owns torso, head
and cloak only, while arm, leg and mantle are unit-sized parts that
player_limb_pose scales and places. The divisors below are those runtime
scales, so authored limb dimensions are world dimensions at idle.
Weapons keep sword.glb's convention: grip at the origin, long axis on Blender
+Z (glTF +Y). Projectiles are centred and fly along Blender -Y (glTF +Z), the
same forward as characters.
"""
import importlib.util
import math
from pathlib import Path
import sys

import bpy
from mathutils import Quaternion, Vector

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
OUT = ROOT / 'assets/models'
PREVIEW = ROOT / 'target/validation/class-system/models'
TAU = math.tau

# Loaded as a plain module so its helpers and palette exist in this Blender
# session without running the knight export guarded by __main__.
_spec = importlib.util.spec_from_file_location('generate_hero_helpers', HERE / 'generate_hero.py')
hero = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(hero)

ARM_SIZE, ARM_CENTER = (.20, .18, .665), .0075
LEG_SIZE = (.20, .24, .68)
MANTLE_SIZE = (1.02, .34, .15)

GOLD, EMBER, HIDE = hero.GOLD, hero.EMBER, hero.LEATHER
SHADOW = hero.DARK
BONE = hero.material('aged bone', (.56, .49, .34), 0., .62)
ASH = hero.material('ash grey leather', (.20, .19, .175), .04, .8)
TEAL = hero.material('hunter teal cloth', (.03, .15, .125), 0., .92)
SINEW = hero.material('bow sinew', (.43, .34, .21), 0., .7)
SIGHT = hero.material('hunter sight glow', (.30, .85, .65), 0., .4, .9)
VIOLET = hero.material('ashpriest violet cloth', (.062, .044, .22), 0., .9)
COAT = hero.material('ash charcoal cloth', (.066, .064, .074), 0., .92)
WOOD = hero.material('charred staff wood', (.06, .046, .038), 0., .88)
CRYSTAL = hero.material('ember crystal glow', (.98, .34, .06), 0., .3, 2.2)


# ---------------------------------------------------------------- helpers

def reset():
    hero.reset()
    for store in (bpy.data.meshes, bpy.data.armatures):
        for block in list(store):
            if block.users == 0:
                store.remove(block)


def outward(obj):
    bpy.ops.object.select_all(action='DESELECT')
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode='EDIT')
    bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode='OBJECT')
    return obj


def mesh_object(name, verts, faces, mat, closed=True, crease=48.):
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([tuple(v) for v in verts], [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    hero.finish(obj, name, mat)
    if closed:
        outward(obj)
    if crease:
        mesh.set_sharp_from_angle(angle=math.radians(crease))
    return obj


def skin(name, rings, mat, closed=False, crease=48.):
    """Quad skin over equal-length point rings; capped unless the loop closes."""
    n, count = len(rings[0]), len(rings)
    faces = []
    for r in range(count if closed else count - 1):
        s = (r + 1) % count
        for i in range(n):
            j = (i + 1) % n
            faces.append((r*n+i, r*n+j, s*n+j, s*n+i))
    if not closed:
        faces.append(tuple(reversed(range(n))))
        faces.append(tuple((count-1)*n+i for i in range(n)))
    return mesh_object(name, [p for ring in rings for p in ring], faces, mat, crease=crease)


def tube(name, points, radii, mat, sides=8, side=(1, 0, 0), closed=False, crease=48.):
    """Tapering limb along a path. Radii are (along `side`, across) pairs."""
    pts = [Vector(p) for p in points]
    ref = Vector(side).normalized()
    rings = []
    for i, p in enumerate(pts):
        if closed:
            tangent = pts[(i+1) % len(pts)] - pts[i-1]
        else:
            tangent = pts[min(i+1, len(pts)-1)] - pts[max(i-1, 0)]
        tangent.normalize()
        u = ref - tangent*ref.dot(tangent)
        u = tangent.orthogonal() if u.length < 1e-5 else u
        u.normalize()
        v = tangent.cross(u)
        ra, rb = radii[i] if isinstance(radii[i], tuple) else (radii[i], radii[i])
        rings.append([p + u*(ra*math.cos(TAU*k/sides)) + v*(rb*math.sin(TAU*k/sides))
                      for k in range(sides)])
    return skin(name, rings, mat, closed, crease)


def smooth(points, steps=4):
    """Catmull-Rom resample so limbs and branches bend instead of kinking."""
    pts = [Vector(p) for p in points]
    ext = [pts[0]*2-pts[1]] + pts + [pts[-1]*2-pts[-2]]
    out = []
    for i in range(1, len(ext)-2):
        a, b, c, d = ext[i-1:i+3]
        for s in range(steps):
            t = s/steps
            out.append(.5*((2*b) + (c-a)*t + (2*a-5*b+4*c-d)*t*t + (3*b-a-3*c+d)*t**3))
    return out + [pts[-1]]


def lerp_list(values, count):
    out = []
    for i in range(count):
        f = i*(len(values)-1)/(count-1)
        k = min(int(f), len(values)-2)
        a, b = values[k], values[k+1]
        t = f-k
        if isinstance(a, tuple):
            out.append(tuple(x+(y-x)*t for x, y in zip(a, b)))
        else:
            out.append(a+(b-a)*t)
    return out


def limb(name, points, radii, mat, sides=8, side=(1, 0, 0), steps=4):
    path = smooth(points, steps)
    return tube(name, path, lerp_list(radii, len(path)), mat, sides, side)


def zring(z, rx, ry, cy=0., n=36, lobes=0, amp=0., wave=0., cx=0.):
    """Horizontal ellipse; lobes pleat the radius and scallop the height."""
    out = []
    for i in range(n):
        t = TAU*i/n
        fold = math.cos(lobes*(t+math.pi/2)) if lobes else 0.
        m = 1+amp*fold
        out.append((cx+rx*m*math.cos(t), cy+ry*m*math.sin(t), z+wave*fold))
    return out


def yring(y, zc, rx, rz, n=28, brow=0.):
    """Vertical ellipse facing Blender -Y; brow pulls the top edge forward."""
    return [(rx*math.cos(TAU*i/n), y-brow*max(0., math.sin(TAU*i/n))**2,
             zc+rz*math.sin(TAU*i/n)) for i in range(n)]


def hoop(name, z, rx, ry, radius, mat, cy=0., lobes=0, amp=0., wave=0., cx=0.):
    return tube(name, zring(z, rx, ry, cy, 40, lobes, amp, wave, cx), [radius]*40, mat,
                6, (0, 0, 1), closed=True)


def sheet(name, grid, mat, thick=.018):
    rows, cols = len(grid), len(grid[0])
    faces = [(r*cols+c, (r+1)*cols+c, (r+1)*cols+c+1, r*cols+c+1)
             for r in range(rows-1) for c in range(cols-1)]
    obj = mesh_object(name, [p for row in grid for p in row], faces, mat, closed=False, crease=0)
    solid = obj.modifiers.new('cloth thickness', 'SOLIDIFY')
    solid.thickness = thick
    solid.offset = 0
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=solid.name)
    outward(obj)
    obj.data.set_sharp_from_angle(angle=math.radians(60))
    return obj


def ellipsoid(name, loc, size, mat, segments=24, rings=16):
    """Lat-long ellipsoid from explicit data; Blender's uv-sphere primitive does
    not export byte-identically across runs, so the knight helper is not used."""
    cx, cy, cz = loc
    sx, sy, sz = size
    verts = [(cx, cy, cz+sz)]
    for r in range(1, rings):
        phi = math.pi*r/rings
        verts += [(cx+sx*math.sin(phi)*math.cos(TAU*i/segments), cy+sy*math.sin(phi)*math.sin(TAU*i/segments),
                   cz+sz*math.cos(phi)) for i in range(segments)]
    verts.append((cx, cy, cz-sz))
    last = len(verts)-1
    faces = [(0, 1+(i+1) % segments, 1+i) for i in range(segments)]
    for r in range(rings-2):
        a, b = 1+r*segments, 1+(r+1)*segments
        faces += [(a+i, a+(i+1) % segments, b+(i+1) % segments, b+i) for i in range(segments)]
    base = 1+(rings-2)*segments
    faces += [(last, base+i, base+(i+1) % segments) for i in range(segments)]
    return mesh_object(name, verts, faces, mat, crease=0)


def plate(name, points, mat, thick=.022):
    return outward(hero.plate(name, points, mat, thick))


def shard(name, tip, tail, radius, mat, sides=5, belly=.35, squash=1., side=None):
    """Faceted bipyramid: a splinter, crystal or blade, never a ball."""
    a, b = Vector(tip), Vector(tail)
    axis = (b-a).normalized()
    u = Vector(side) if side else axis.orthogonal()
    u = (u - axis*u.dot(axis)).normalized()
    v = axis.cross(u)
    mid = a + (b-a)*belly
    verts = [a] + [mid + u*(radius*math.cos(TAU*i/sides)) + v*(radius*squash*math.sin(TAU*i/sides))
                   for i in range(sides)] + [b]
    faces = [(0, 1+i, 1+(i+1) % sides) for i in range(sides)]
    faces += [(sides+1, 1+(i+1) % sides, 1+i) for i in range(sides)]
    obj = mesh_object(name, verts, faces, mat, crease=0)
    for face in obj.data.polygons:
        face.use_smooth = False
    return obj


def flat(name, points, mat):
    """Single upward-facing etched polygon for ground telegraphs."""
    return mesh_object(name, points, [tuple(range(len(points)))], mat, closed=False, crease=0)


def apply_meshes():
    meshes = [o for o in bpy.context.scene.objects if o.type == 'MESH']
    for obj in meshes:
        bpy.ops.object.select_all(action='DESELECT')
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return meshes


def write(name):
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.export_scene.gltf(filepath=str(OUT/(name+'.glb')), export_format='GLB',
        use_selection=True, export_apply=True, export_yup=True, export_extras=True,
        export_cameras=False, export_lights=False, export_animations=False)
    print('EXPORTED', name)


def export_part(name):
    """Knight part layout: one mesh per material, hit rig left untouched."""
    apply_meshes()
    for mat in list(bpy.data.materials):
        group = [o for o in bpy.context.scene.objects if o.type == 'MESH'
                 and o.data.materials[0] == mat]
        if not group:
            continue
        bpy.ops.object.select_all(action='DESELECT')
        for obj in group:
            obj.select_set(True)
        bpy.context.view_layer.objects.active = group[0]
        if len(group) > 1:
            bpy.ops.object.join()
        group[0].name = group[0].data.name = name+' '+mat.name
        group[0]['hero_part'] = name
    write(name)


def export_single(name):
    """sword.glb layout: one mesh node carrying every material."""
    meshes = apply_meshes()
    bpy.ops.object.select_all(action='DESELECT')
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    meshes[0].name = meshes[0].data.name = name
    write(name)


# ---------------------------------------------------------------- stalker

def stalker_body():
    reset()
    hero.loft('hunter jerkin', [(.72, .235, .16, 0), (.93, .205, .14, 0), (1.16, .27, .17, 0),
              (1.38, .335, .19, 0), (1.50, .20, .15, 0)], ASH)
    # Bone harness: a keeled sternum plate with three tusk ribs sweeping back.
    plate('bone sternum plate', [(0, -.218, 1.01), (.078, -.212, 1.10), (.066, -.208, 1.30),
          (0, -.214, 1.35), (-.066, -.208, 1.30), (-.078, -.212, 1.10)], BONE, .034)
    limb('sternum keel', [(0, -.232, 1.04), (0, -.24, 1.18), (0, -.228, 1.32)],
         [(.012, .010), (.02, .016), (.008, .008)], BONE, 6)
    for s in [-1, 1]:
        for i in range(3):
            z = 1.09+i*.078
            limb('harness tusk rib', [(s*.05, -.214, z), (s*(.16+.012*i), -.176, z+.035),
                 (s*(.255+.015*i), -.094, z+.05)], [.021, .017, .005], BONE, 6, (0, 0, 1))
    hero.loft('hunter belt', [(.895, .246, .176, 0), (.955, .246, .176, 0)], HIDE)
    plate('bone belt toggle', [(-.05, -.212, .90), (.05, -.212, .90), (.062, -.212, .925),
          (.05, -.212, .95), (-.05, -.212, .95), (-.062, -.212, .925)], BONE, .028)
    plate('belt pouch', [(.115, -.212, .80), (.215, -.185, .80), (.225, -.185, .905),
          (.105, -.212, .905)], HIDE, .07)
    limb('pouch bone peg', [(.145, -.224, .87), (.19, -.212, .862)], [.011, .006], BONE, 6, (0, 0, 1), 1)
    # Short split hide tassets and one sash tail; both moving legs stay visible.
    for s in [-1, 1]:
        plate('hide hip tasset', [(s*.10, -.186, .90), (s*.245, -.115, .89), (s*.262, -.125, .66),
              (s*.19, -.165, .60), (s*.115, -.20, .68)], ASH, .026)
    plate('teal sash tail', [(-.10, -.214, .90), (-.01, -.224, .90), (.0, -.236, .64),
          (-.045, -.238, .55), (-.085, -.232, .66)], TEAL, .018)
    plate('rear hide tasset', [(-.20, .16, .90), (.20, .16, .90), (.17, .19, .66),
          (0, .2, .60), (-.17, .19, .66)], ASH, .026)
    hero.loft('hunter neck wrap', [(1.42, .215, .17, 0), (1.50, .235, .19, -.01),
              (1.58, .20, .165, 0)], TEAL)
    # Deep hood: recessed face opening, forward brow, peak trailing backwards.
    skin('deep hunter hood', [yring(-.11, 1.745, .14, .17), yring(-.232, 1.745, .163, .196, brow=.05),
         yring(-.25, 1.75, .19, .228, brow=.075), yring(-.17, 1.752, .238, .264),
         yring(-.04, 1.76, .264, .284), yring(.10, 1.775, .246, .262),
         yring(.22, 1.79, .186, .20), yring(.33, 1.792, .116, .125),
         yring(.42, 1.775, .056, .06), yring(.49, 1.745, .012, .012)], TEAL)
    ellipsoid('hood shadow', (0, -.118, 1.745), (.132, .022, .162), SHADOW)
    plate('bone muzzle mask', [(-.088, -.168, 1.725), (.088, -.168, 1.725), (.072, -.168, 1.64),
          (.026, -.168, 1.592), (-.026, -.168, 1.592), (-.072, -.168, 1.64)], BONE, .03)
    for s in [-1, 1]:
        plate('mask breathing slit', [(s*.022, -.172, 1.615), (s*.034, -.172, 1.615),
              (s*.05, -.172, 1.69), (s*.038, -.172, 1.69)], SHADOW, .012)
        shard('hunter eye glint', (s*.03, -.148, 1.762), (s*.098, -.148, 1.79), .012, SIGHT, 4)
    for i in range(5):
        y = -.15+i*.11
        limb('hood ridge stitch', [(-.02, y, 2.035-.012*i-.008*i*i), (.02, y, 2.035-.012*i-.008*i*i)],
             [.007, .007], BONE, 5, (0, 1, 0), 1)
    # Ragged half-cloak, cut short and hung off-centre toward the quiver side.
    rag = [.04, .13, .02, .16, .07, .0, .12, .03, .15, .05, .10]
    cols, rows = len(rag)-1, 6
    grid = []
    for r in range(rows):
        t = r/(rows-1)
        row = []
        for c in range(cols+1):
            u = c/cols*2-1
            row.append((-.05+u*(.27+.10*t), .225+.13*t+.024*math.cos(c*math.pi*.66),
                        1.47-t*(.50+rag[c])))
        grid.append(row)
    sheet('ragged teal half cloak', grid, TEAL)
    # Quiver slung across the back; shafts rise over the draw-hand shoulder.
    foot, lip = Vector((.14, .40, .93)), Vector((-.21, .37, 1.57))
    axis = (lip-foot).normalized()
    tube('hide quiver', [foot-axis*.01, foot, foot.lerp(lip, .5), lip],
         [.02, .05, .06, .068], HIDE, 10)
    tube('quiver bone lip', [lip-axis*.03, lip+axis*.012], [.076, .08], BONE, 10)
    tube('quiver bone heel', [foot-axis*.022, foot+axis*.04], [.036, .058], BONE, 10)
    tube('quiver sling', [lip+Vector((.02, -.07, -.04)), Vector((-.16, .22, 1.47)),
         Vector((-.2, .0, 1.5))], [(.022, .008)]*3, HIDE, 6, (0, 0, 1))
    across = axis.cross(Vector((0, 1, 0))).normalized()
    for i, (dx, dy, reach) in enumerate([(-.03, -.02, .17), (.028, -.012, .2), (0, .03, .15), (-.012, .008, .22)]):
        base = lip + across*dx + Vector((0, dy, 0))
        tube('quivered bone shaft', [base-axis*.05, base+axis*reach], [.009, .009], BONE, 6)
        for turn in [0, 1]:
            vane = across if turn == 0 else Vector((0, 1, 0))
            tube('quivered fletching', [base+axis*(reach-.10), base+axis*(reach-.05), base+axis*(reach-.006)],
                 [(.008, .003), (.03, .003), (.02, .003)], SIGHT if i == 3 and turn == 0 else TEAL, 4, vane)
    hero.hit_bones()
    export_part('hero_stalker')


def stalker_arm():
    reset()
    hero.loft('hunter sleeve', [(-.29, .052, .05, 0), (-.03, .066, .062, 0), (.22, .078, .072, 0),
              (.36, .058, .056, 0)], ASH)
    hero.loft('teal sleeve binding', [(.09, .083, .077, 0), (.17, .085, .079, 0)], TEAL)
    ellipsoid('hide elbow guard', (0, .026, -.02), (.071, .066, .074), HIDE)
    hero.loft('bone bracer', [(-.275, .064, .06, -.003), (-.20, .08, .073, -.006),
              (-.07, .084, .076, -.006)], BONE)
    for z, rx, ry in [(-.235, .078, .072), (-.11, .088, .08)]:
        hoop('bracer hide strap', z, rx, ry, .009, HIDE, -.006)
    limb('bracer tusk ridge', [(0, -.083, -.25), (0, -.096, -.16), (0, -.088, -.06)],
         [.011, .016, .005], BONE, 6)
    ellipsoid('archer glove', (0, -.006, -.325), (.066, .06, .078), HIDE)
    for i in range(3):
        x = -.036+i*.036
        limb('glove finger tab', [(x, -.056, -.30), (x, -.062, -.335), (x, -.05, -.375)],
             [.014, .015, .009], ASH, 6)
    hero.normalize(ARM_SIZE, z_center=ARM_CENTER)
    export_part('hero_stalker_arm')


def stalker_leg():
    reset()
    hero.loft('hunter trouser', [(-.30, .06, .066, 0), (.02, .074, .078, 0), (.30, .09, .09, 0),
              (.40, .094, .086, 0)], ASH)
    hero.loft('teal thigh wrap', [(.17, .089, .091, 0), (.235, .092, .092, 0)], TEAL)
    hero.loft('tall soft hide boot', [(-.385, .066, .07, 0), (-.25, .073, .079, 0),
              (-.07, .085, .089, 0), (-.03, .093, .096, 0)], HIDE)
    for z, rx, ry in [(-.27, .075, .081), (-.15, .082, .087)]:
        hoop('boot binding strap', z, rx, ry, .009, ASH)
    plate('bone knee cap', [(-.058, -.108, -.03), (.058, -.108, -.03), (.07, -.112, .06),
          (0, -.118, .135), (-.07, -.112, .06)], BONE, .04)
    limb('bone shin splint', [(0, -.092, -.07), (0, -.092, -.2), (0, -.082, -.33)],
         [(.024, .012), (.018, .011), (.008, .006)], BONE, 6)
    ellipsoid('soft boot foot', (0, -.065, -.38), (.082, .152, .048), HIDE)
    limb('boot toe seam', [(-.05, -.16, -.356), (0, -.2, -.36), (.05, -.16, -.356)],
         [.008, .009, .008], ASH, 5, (0, 0, 1))
    hero.normalize(LEG_SIZE)
    export_part('hero_stalker_leg')


def stalker_mantle():
    reset()
    # A soft cowl collar draped over the shoulders: cloth, no forged pauldrons.
    skin('teal shoulder cowl', [zring(-.118, .472, .238, .012, lobes=9, wave=.022),
         zring(-.03, .44, .232, .01), zring(.05, .315, .203, .005), zring(.115, .23, .178),
         zring(.15, .205, .165)], TEAL)
    skin('hide collar yoke', [zring(.03, .345, .214, .005), zring(.085, .268, .19),
         zring(.128, .226, .176)], ASH)
    # One strapped scapula scale guards the bow arm; the draw arm stays bare.
    for i in range(2):
        ellipsoid('bone shoulder scale', (.375+.05*i, .0, .03-.045*i),
                       (.118-.012*i, .158-.014*i, .05), BONE)
    for y in [-.09, .09]:
        limb('shoulder tusk', [(.40, y, .05), (.50, y*1.25, .10), (.545, y*1.4, .18)],
             [.02, .015, .004], BONE, 6, (0, 1, 0))
    limb('scale lashing', [(.33, -.165, -.04), (.42, -.172, -.0), (.5, -.12, -.03)],
         [(.02, .007)]*3, HIDE, 6, (0, 0, 1))
    limb('bone cowl toggle', [(-.07, -.236, -.0), (0, -.25, .006), (.07, -.236, -.0)],
         [.007, .019, .007], BONE, 6, (0, 0, 1))
    for s in [-1, 1]:
        limb('toggle cord', [(s*.03, -.238, .0), (s*.07, -.226, .06), (s*.10, -.196, .10)],
             [.007, .007, .006], HIDE, 5, (0, 0, 1))
    hero.normalize(MANTLE_SIZE)
    export_part('hero_stalker_mantle')


# ------------------------------------------------------------ embercaller

ROBE = [(.27, .375, .345, .07, .026), (.50, .33, .295, .055, 0), (.78, .29, .232, .03, 0),
        (.98, .238, .172, 0, 0), (1.20, .285, .19, 0, 0), (1.38, .335, .195, 0, 0),
        (1.50, .20, .15, 0, 0)]


def robe_at(z):
    """(rx, ry) of the inner robe at height z, pleat crests included."""
    for (z0, x0, y0, a0, _), (z1, x1, y1, a1, _) in zip(ROBE, ROBE[1:]):
        if z <= z1:
            t = max(0., (z-z0)/(z1-z0))
            amp = 1+a0+(a1-a0)*t
            return (x0+(x1-x0)*t)*amp, (y0+(y1-y0)*t)*amp
    return ROBE[-1][1], ROBE[-1][2]


def charm(name, top, drop, kind):
    top = Vector(top)
    end = top - Vector((0, 0, drop))
    tube(name+' cord', [top, end], [.005, .005], HIDE, 5)
    if kind == 'vial':
        tube(name+' bone vial', [end+Vector((0, 0, .012)), end, end-Vector((0, 0, .05)),
             end-Vector((0, 0, .078))], [.01, .022, .026, .008], BONE, 8)
        tube(name+' gold stopper', [end+Vector((0, 0, .004)), end+Vector((0, 0, .024))],
             [.016, .012], GOLD, 8)
    elif kind == 'ember':
        shard(name+' ember bead', end-Vector((0, 0, .075)), end, .021, EMBER, 5, .5)
        hoop(name+' ember cage', end.z-.038, .026, .026, .005, GOLD, end.y, cx=end.x)
    else:
        shard(name+' gold reliquary tag', end-Vector((0, 0, .09)), end+Vector((0, 0, .01)),
              .034, GOLD, 4, .62, .4, (1, 0, 0))


def embercaller_body():
    reset()
    skin('layered violet robe', [zring(z, rx, ry, 0, 48, 12, amp, wave)
         for z, rx, ry, amp, wave in ROBE], VIOLET)
    hoop('robe hem gilding', .30, .373, .343, .011, GOLD, lobes=12, amp=.066, wave=.026)
    # Open-fronted ash coat: the violet robe shows down the centre and below.
    gap, cols, rows = math.radians(78), 30, 9
    grid = []
    for r in range(rows):
        f = r/(rows-1)
        row = []
        for c in range(cols+1):
            t = c/cols
            angle = -math.pi/2 + gap/2 + t*(TAU-gap)
            hem = .56 - .17*math.sin(math.pi*t) - (.035 if c % 2 else 0.)
            z = 1.46 + (hem-1.46)*f
            rx, ry = robe_at(z)
            swell = .03 + .02*f
            row.append(((rx+swell)*math.cos(angle), (ry+swell)*math.sin(angle), z))
        grid.append(row)
    sheet('open ash coat', grid, COAT, .02)
    for c in [0, cols]:
        limb('coat front gilding', [grid[r][c] for r in range(rows)], [.011]*rows, GOLD, 6, (0, 1, 0), 2)
    # Narrow stole carrying the ember sigil.
    grid = []
    for r in range(8):
        z = 1.40 - r*.115
        _, ry = robe_at(z)
        half = .07 + .003*r
        grid.append([(u*half, -ry-.014-.004*abs(u), z - (.05 if r == 7 and u == 0 else 0))
                     for u in [-1, 0, 1]])
    sheet('ash stole', grid, COAT, .016)
    for u in [0, 2]:
        limb('stole gilding', [grid[r][u] for r in range(8)], [.008]*8, GOLD, 5, (0, 1, 0), 2)
    shard('stole gold tassel', (0, grid[7][1][1]-.004, .49), (0, grid[7][1][1]-.004, .61), .024, GOLD, 4, .7)
    sy = -robe_at(1.22)[1]-.03
    tube('ember sigil ring', [(.056*math.cos(TAU*i/20), sy, 1.22+.056*math.sin(TAU*i/20)) for i in range(20)],
         [.0075]*20, EMBER, 5, (0, 1, 0), closed=True)
    shard('ember sigil flame', (0, sy, 1.262), (0, sy, 1.186), .015, EMBER, 4, .6, .5, (1, 0, 0))
    for s in [-1, 1]:
        shard('ember sigil wing', (s*.036, sy, 1.236), (s*.014, sy, 1.19), .008, EMBER, 4, .5, .6, (0, 1, 0))
        shard('ember sigil ray', (s*.064, sy, 1.22), (s*.1, sy+.004, 1.22), .008, EMBER, 4, .3, .6, (0, 1, 0))
    shard('ember sigil ray', (0, sy, 1.284), (0, sy+.002, 1.325), .008, EMBER, 4, .3, .6, (0, 1, 0))
    # Cord girdle with hanging reliquary charms.
    bx, by = robe_at(.98)
    hero.loft('hide girdle', [(.95, bx+.052, by+.052, 0), (1.01, bx+.052, by+.052, 0)], HIDE)
    hoop('girdle gilding', 1.012, bx+.054, by+.054, .009, GOLD)
    for angle, drop, kind in [(-2.38, .17, 'vial'), (-2.02, .25, 'ember'), (-.80, .14, 'tag'),
                              (-1.13, .22, 'vial'), (-.42, .19, 'ember')]:
        charm('reliquary', ((bx+.062)*math.cos(angle), (by+.062)*math.sin(angle), .955), drop, kind)
    # Tall cowl: violet peak, shadowed face, thin gold circlet of thorns.
    hero.loft('tall ash cowl', [(1.48, .20, .17, 0), (1.60, .255, .245, 0), (1.77, .265, .262, .005),
              (1.92, .215, .22, .03), (2.04, .13, .14, .066), (2.13, .055, .06, .10),
              (2.175, .012, .012, .116)], VIOLET)
    ellipsoid('cowl shadow', (0, -.205, 1.735), (.14, .07, .168), SHADOW)
    tube('cowl rolled hem', [(.158*math.cos(TAU*i/28), -.268+.062*math.cos(TAU*i/28)**2,
         1.735+.186*math.sin(TAU*i/28)) for i in range(28)], [.021]*28, VIOLET, 6, (0, 1, 0), closed=True)
    for s in [-1, 1]:
        shard('ember eye glint', (s*.03, -.274, 1.752), (s*.082, -.268, 1.772), .011, EMBER, 4)
    hoop('thin gold circlet', 1.915, .226, .23, .011, GOLD, .029)
    for i in range(9):
        a = -math.pi/2 + (i-4)*.56
        rise = .15 - .022*abs(i-4)
        base = Vector((.226*math.cos(a), .029+.23*math.sin(a), 1.915))
        lean = Vector((math.cos(a), math.sin(a), 0))*.028
        shard('circlet thorn', base+lean+Vector((0, 0, rise)), base-Vector((0, 0, .008)), .014, GOLD, 4, .86)
    shard('circlet ember stone', (0, -.212, 1.955), (0, -.208, 1.885), .02, EMBER, 4, .5, .6, (1, 0, 0))
    # Liturgical back panel instead of a cape, so the robe hem stays the silhouette.
    grid = []
    for r in range(8):
        z = 1.46 - r*.125
        _, ry = robe_at(z)
        half = .115 + .008*r
        grid.append([(u*half, ry+.062+.012*math.cos(u*3.), z - (.07 if r == 7 and u == 0 else 0))
                     for u in [-1, -.5, 0, .5, 1]])
    sheet('violet back panel', grid, VIOLET, .016)
    limb('back panel gilding', [(grid[7][c][0], grid[7][c][1]+.004, grid[7][c][2]) for c in range(5)],
         [.009]*5, GOLD, 5, (0, 1, 0), 1)
    hero.hit_bones()
    export_part('hero_embercaller')


def embercaller_arm():
    reset()
    skin('violet bell sleeve', [zring(-.262, .13, .114, n=24, lobes=8, amp=.035, wave=.008),
         zring(-.17, .119, .106, n=24, lobes=8, amp=.02), zring(.02, .094, .088, n=24),
         zring(.25, .088, .084, n=24), zring(.36, .066, .066, n=24)], VIOLET)
    hoop('sleeve cuff gilding', -.245, .129, .113, .008, GOLD, lobes=8, amp=.03, wave=.008)
    hero.loft('ash sleeve cap', [(.12, .099, .093, 0), (.25, .096, .09, 0), (.36, .08, .078, 0)], COAT)
    hoop('sleeve cap gilding', .128, .1, .094, .008, GOLD)
    hero.loft('dark under sleeve', [(-.305, .05, .048, 0), (-.25, .062, .058, 0)], SHADOW)
    ellipsoid('ash wrapped hand', (0, -.004, -.33), (.054, .05, .07), BONE)
    for z in [-.318, -.345]:
        hoop('hand ash binding', z, .053, .05, .006, COAT, -.004)
    hero.normalize(ARM_SIZE, z_center=ARM_CENTER)
    export_part('hero_embercaller_arm')


def embercaller_leg():
    reset()
    hero.loft('ashpriest hose', [(-.30, .058, .064, 0), (.02, .072, .076, 0), (.31, .086, .086, 0),
              (.40, .09, .084, 0)], SHADOW)
    skin('violet under robe wrap', [zring(-.235, .083, .088, n=20, lobes=5, wave=.012),
         zring(-.05, .084, .087, n=20), zring(.21, .092, .092, n=20)], VIOLET)
    hero.loft('soft ash boot', [(-.39, .064, .068, 0), (-.30, .071, .077, 0), (-.20, .079, .084, 0)], HIDE)
    hoop('boot gold anklet', -.27, .075, .081, .008, GOLD)
    ellipsoid('pointed slipper', (0, -.07, -.386), (.078, .156, .044), HIDE)
    shard('gilded slipper toe', (0, -.262, -.368), (0, -.15, -.39), .04, GOLD, 5, .7, .62, (1, 0, 0))
    hero.normalize(LEG_SIZE)
    export_part('hero_embercaller_leg')


def embercaller_mantle():
    reset()
    skin('ash shoulder shawl', [zring(-.15, .52, .262, .012, lobes=8, wave=.03),
         zring(-.05, .47, .246, .01), zring(.04, .32, .206, .005), zring(.10, .232, .178)], COAT)
    hoop('shawl hem gilding', -.142, .521, .263, .0095, GOLD, .012, lobes=8, wave=.03)
    skin('violet upper capelet', [zring(-.035, .39, .232, .008, lobes=8, wave=-.014),
         zring(.03, .312, .206, .004), zring(.09, .242, .182), zring(.125, .222, .172)], VIOLET)
    hoop('capelet hem gilding', -.03, .391, .233, .007, GOLD, .008, lobes=8, wave=-.014)
    limb('gold clasp chain', [(-.13, -.236, -.0), (0, -.262, -.035), (.13, -.236, -.0)],
         [.008, .008, .008], GOLD, 5, (0, 0, 1))
    shard('ember clasp stone', (0, -.274, -.085), (0, -.268, .0), .026, EMBER, 4, .55, .5, (1, 0, 0))
    for s in [-1, 1]:
        shard('shawl gold drop', (s*.515, .012, -.25), (s*.515, .012, -.14), .02, GOLD, 4, .7)
    hero.normalize(MANTLE_SIZE)
    export_part('hero_embercaller_mantle')


# ---------------------------------------------------------------- weapons

def bone_bow():
    reset()
    # Stave on +Z, back toward -Y (forward); the string sits at +Y by the archer.
    upper = [(-.012, .12), (.03, .36), (.13, .62), (.218, .82), (.24, .93), (.205, 1.0), (.14, 1.05)]
    spine = [(0, y, -z) for y, z in reversed(upper)] + [(0, -.012, 0)] + [(0, y, z) for y, z in upper]
    half = [(.036, .034), (.05, .024), (.046, .019), (.038, .016), (.03, .014), (.022, .012), (.012, .008)]
    limb('bone recurve stave', spine, list(reversed(half)) + [(.034, .036)] + half, BONE, 8)
    for s in [-1, 1]:
        for z, r in [(.36, .058), (.62, .054), (.82, .045)]:
            y = dict((round(zz, 2), yy) for yy, zz in upper)[z]
            ellipsoid('stave vertebra knuckle', (0, y, s*z), (r, .03, .034), BONE)
        limb('riser tusk', [(0, -.01, s*.14), (0, -.085, s*.20), (0, -.165, s*.30)],
             [.024, .017, .004], BONE, 6)
        for z in [.135, .185]:
            hoop('sinew riser lashing', s*z, .043, .04, .008, SINEW, -.006)
        hoop('sinew nock lashing', s*.985, .03, .018, .007, SINEW, .215)
        hoop('teal grip ribbon', s*.105, .041, .043, .009, TEAL, -.012)
        tube('sinew bowstring loop', [(0, .252, s*.93), (0, .23, s*.995), (0, .156, s*1.046)],
             [.007, .007, .007], SINEW, 5)
    tube('hide bow grip', [(0, -.012, -.1), (0, -.014, 0), (0, -.012, .1)], [.039, .041, .039], HIDE, 10)
    tube('taut sinew bowstring', [(0, .252, -.93), (0, .252, .93)], [.007, .007], SINEW, 6)
    tube('string serving', [(0, .252, -.07), (0, .252, .07)], [.011, .011], TEAL, 6)
    export_single('bone_bow')


def ember_staff():
    reset()
    # 2.52 long: foot at -1.008, grip at the origin (40 % up), cage tips at 1.512.
    wobble = [(0, 0, -.9), (.012, -.008, -.6), (-.01, .006, -.3), (0, 0, -.1), (0, 0, .16),
              (.011, .006, .42), (-.013, .009, .7), (.01, -.006, .95), (0, 0, 1.13)]
    limb('charred staff shaft', wobble, [.027, .03, .031, .034, .034, .031, .03, .031, .036], WOOD, 9)
    for x, y, z, r in [(.03, -.01, -.52, .03), (-.03, .012, .58, .028), (.028, .014, .86, .026)]:
        limb('shaft burl', [(x*.3, y*.3, z-.05), (x, y, z), (x*.4, y*.4, z+.06)],
             [.012, r, .01], WOOD, 6)
    tube('gilded foot spike', [(0, 0, -1.008), (0, 0, -.955), (0, 0, -.9), (0, 0, -.87)],
         [.004, .024, .032, .03], GOLD, 8)
    tube('violet grip wrap', [(0, 0, -.1), (0, 0, .02), (0, 0, .15)], [.039, .041, .039], VIOLET, 10)
    for z in [-.105, .155]:
        hoop('grip gilding', z, .041, .041, .009, GOLD)
    tube('gilded head collar', [(0, 0, 1.09), (0, 0, 1.12), (0, 0, 1.17), (0, 0, 1.185)],
         [.036, .046, .05, .04], GOLD, 10)
    for i in range(4):
        a = math.pi/4 + i*math.pi/2
        d = Vector((math.cos(a), math.sin(a), 0))
        path = [d*r + Vector((0, 0, z)) for r, z in
                [(.036, 1.16), (.1, 1.215), (.138, 1.31), (.122, 1.405), (.07, 1.47), (.04, 1.512)]]
        limb('charred cage claw', path, [.019, .019, .017, .014, .01, .003], WOOD, 6, d.cross(Vector((0, 0, 1))))
        hoop('claw gold band', 1.31, .013, .013, .006, GOLD, .138*d.y, cx=.138*d.x)
    shard('caged ember crystal', (0, 0, 1.455), (0, 0, 1.205), .07, CRYSTAL, 6, .42)
    shard('ember crystal splinter', (.05, .02, 1.36), (.03, .014, 1.23), .022, CRYSTAL, 4, .4)
    shard('ember crystal splinter', (-.046, -.03, 1.33), (-.026, -.02, 1.225), .019, CRYSTAL, 4, .4)
    for a, z in [(.6, .97), (2.7, 1.03), (4.6, .99)]:
        d = Vector((math.cos(a), math.sin(a), 0))*.034
        shard('ember shaft vein', d+Vector((0, 0, z+.055)), d+Vector((0, 0, z-.05)), .007, EMBER, 4, .4, .5, d)
    charm('staff reliquary', (-.046, .012, 1.125), .13, 'vial')
    export_single('ember_staff')


# ------------------------------------------------------------ projectiles

def vfx(name, color, glow, roughness=.5):
    return hero.material(name, color, 0., roughness, glow)


def bone_arrow():
    reset()
    # 0.9 long; head leads on -Y.
    edge = vfx('bone arrow pale edge', (.78, .72, .56), .55)
    fletch = vfx('bone arrow teal fletching', (.12, .55, .45), .6)
    tube('bone arrow shaft', [(0, -.31, 0), (0, .05, 0), (0, .435, 0)], [.019, .021, .017], BONE, 6)
    shard('bone arrow broadhead', (0, -.45, 0), (0, -.27, 0), .058, edge, 4, .72, .3, (1, 0, 0))
    tube('bone arrow nock', [(0, .42, 0), (0, .45, 0)], [.024, .02], BONE, 6)
    for i in range(3):
        a = math.pi/2 + i*TAU/3
        d = Vector((math.cos(a), 0, math.sin(a)))
        mesh_object('arrow vane', [d*.02+Vector((0, .2, 0)), d*.068+Vector((0, .3, 0)),
                    d*.068+Vector((0, .405, 0)), d*.02+Vector((0, .42, 0))], [(0, 1, 2, 3)],
                    fletch, closed=False, crease=0)
    export_single('bone_arrow')


def ember_bolt():
    reset()
    # About 0.34 across and 0.46 along travel.
    core = vfx('ember bolt core', (.98, .36, .06), 2.2, .3)
    crust = vfx('ember bolt cinder', (.42, .09, .02), .5, .8)
    shard('ember bolt heart', (0, -.23, 0), (0, .2, 0), .088, core, 5, .36)
    for i, (reach, lift, size) in enumerate([(.15, .0, .05), (.14, .9, .045), (.155, 2.2, .04),
                                             (.13, 3.4, .046), (.15, 4.6, .042)]):
        a = lift + .5
        d = Vector((math.cos(a), 0, math.sin(a)))
        nose = d*(reach*.55) + Vector((0, -.14+.03*i, 0))
        tail = d*reach + Vector((0, .1+.026*i, 0))
        shard('ember bolt splinter', nose, tail, size, crust if i % 2 else core, 4, .35)
    export_single('ember_bolt')


def ash_lance():
    reset()
    # 1.7 long: a cracked ash casing around one continuous ember core.
    core = vfx('ash lance ember core', (.98, .34, .05), 2., .3)
    crust = vfx('ash lance crust', (.25, .23, .22), 0., .9)
    shard('ash lance ember core', (0, -.83, 0), (0, .8, 0), .034, core, 6, .3)
    shard('ash lance blade', (0, -.85, 0), (0, -.34, 0), .082, crust, 4, .62, .34, (1, 0, 0))
    shard('ash lance blade ember seam', (0, -.8, 0), (0, -.4, 0), .03, core, 4, .6, 1.25, (1, 0, 0))
    for s in [-1, 1]:
        shard('ash lance barb', (s*.03, -.42, 0), (s*.115, -.27, 0), .022, crust, 4, .3, .5, (0, 0, 1))
    for y0, y1, r in [(-.3, -.06, .05), (-.01, .22, .046), (.27, .47, .04), (.52, .68, .033)]:
        tube('ash lance casing', [(0, y0, 0), (0, y0+.03, 0), (0, y1-.03, 0), (0, y1, 0)],
             [r*.7, r, r*.94, r*.62], crust, 6, crease=30)
    for i, y in enumerate([.72, .78, .83]):
        a = i*2.1
        shard('ash lance tail flake', (.03*math.cos(a), y-.07, .03*math.sin(a)),
              (.012*math.cos(a), min(y+.03, .85), .012*math.sin(a)), .016, crust, 4, .4)
    export_single('ash_lance')


def sunder_wave():
    reset()
    # 1.8 wide, 0.5 deep, convex leading edge on -Y; a low forged ridge, no slab.
    edge = vfx('sunder wave edge', (.80, .76, .62), 1.1, .35)
    wake = vfx('sunder wave wake', (.62, .20, .05), .7, .5)
    n = 28
    lead, ridge, trail = [], [], []
    for i in range(n+1):
        u = i/n*2-1
        x = .9*u
        front = -.25 + .5*u*u
        width = .215*(1-u*u)**.85
        lead.append((x, front, 0.))
        ridge.append((x, front+width*.3, .05*(1-u*u)**.5))
        trail.append((x, front+width, 0.))
    for name, a, b, mat in [('sunder wave leading edge', lead, ridge, edge),
                            ('sunder wave trailing bevel', ridge, trail, wake)]:
        faces = [(i, i+1, n+2+i, n+1+i) for i in range(n)]
        mesh_object(name, a+b, faces, mat, closed=False, crease=0)
    mesh_object('sunder wave underside', lead+trail, [(i, n+1+i, n+2+i, i+1) for i in range(n)],
                wake, closed=False, crease=0)
    for x, y, length in [(-.3, .06, .13), (.02, .0, .16), (.34, .07, .12)]:
        shard('sunder wave trailing sliver', (x, y, .012), (x*1.04, y+length, .012), .02, wake, 4, .3, .25, (1, 0, 0))
    export_single('sunder_wave')


def ground_sigil():
    reset()
    # Outer edge is exactly radius 1.0 so runtime scale equals the blast radius.
    line = vfx('ground sigil ember line', (.98, .36, .07), 1.6, .4)
    z = .02

    def arc(r_in, r_out, a0, a1, segs):
        for i in range(segs):
            a, b = a0+(a1-a0)*i/segs, a0+(a1-a0)*(i+1)/segs
            flat('sigil broken ring', [(r_in*math.cos(a), r_in*math.sin(a), z),
                 (r_out*math.cos(a), r_out*math.sin(a), z), (r_out*math.cos(b), r_out*math.sin(b), z),
                 (r_in*math.cos(b), r_in*math.sin(b), z)], line)

    for i in range(4):
        start = i*TAU/4 + math.radians(9)
        arc(.955, 1.0, start, start+math.radians(72), 12)
    for i in range(12):
        a = i*TAU/12 + math.radians(45)
        reach = .17 if i % 3 == 0 else .085
        half = math.radians(1.5 if i % 3 == 0 else 1.)
        if i % 3 == 0:
            # Long ticks sit in the ring breaks and taper to an inward point.
            flat('sigil inward tick', [(math.cos(a-half*2), math.sin(a-half*2), z),
                 (math.cos(a+half*2), math.sin(a+half*2), z),
                 ((1-reach)*math.cos(a), (1-reach)*math.sin(a), z)], line)
        else:
            flat('sigil inward tick', [(.95*math.cos(a-half), .95*math.sin(a-half), z),
                 (.95*math.cos(a+half), .95*math.sin(a+half), z),
                 ((.95-reach)*math.cos(a+half), (.95-reach)*math.sin(a+half), z),
                 ((.95-reach)*math.cos(a-half), (.95-reach)*math.sin(a-half), z)], line)
    export_single('ground_sigil')


GENERATORS = {
    'hero_stalker': stalker_body, 'hero_stalker_arm': stalker_arm,
    'hero_stalker_leg': stalker_leg, 'hero_stalker_mantle': stalker_mantle,
    'hero_embercaller': embercaller_body, 'hero_embercaller_arm': embercaller_arm,
    'hero_embercaller_leg': embercaller_leg, 'hero_embercaller_mantle': embercaller_mantle,
    'bone_bow': bone_bow, 'ember_staff': ember_staff, 'bone_arrow': bone_arrow,
    'ember_bolt': ember_bolt, 'ash_lance': ash_lance, 'sunder_wave': sunder_wave,
    'ground_sigil': ground_sigil,
}
OUTPUTS = [name+'.glb' for name in GENERATORS]
PROJECTILES = ['bone_arrow', 'ember_bolt', 'ash_lance', 'sunder_wave', 'ground_sigil']


# ---------------------------------------------------------------- preview

def bevy(x, y, z):
    """Bevy/glTF Y-up position to Blender Z-up."""
    return Vector((x, -z, y))


def place(name, location=(0, 0, 0), scale=(1, 1, 1), rotation=Quaternion(), offset=Vector((0, 0, 0))):
    bpy.ops.object.select_all(action='DESELECT')
    bpy.ops.import_scene.gltf(filepath=str(OUT/(name+'.glb')))
    pivot = bpy.data.objects.new('preview '+name, None)
    bpy.context.collection.objects.link(pivot)
    for obj in list(bpy.context.selected_objects):
        if obj.parent is None:
            obj.parent = pivot
    pivot.rotation_mode = 'QUATERNION'
    pivot.location = Vector(location)+offset
    pivot.scale = scale
    pivot.rotation_quaternion = rotation
    return pivot


def assemble(prefix, weapon, offset=Vector((0, 0, 0)), turn=0.):
    """Runtime idle pose from player_limb_pose / player_visual_pose, in Blender axes."""
    root = bpy.data.objects.new('preview root '+prefix, None)
    bpy.context.collection.objects.link(root)
    parts = [place(prefix), place(prefix+'_mantle', bevy(0, 1.43, .03), MANTLE_SIZE)]
    hands = {-1: bevy(-.49, .89, .08), 1: bevy(.50, .94, .23)}
    for side in [-1, 1]:
        shoulder = bevy(side*.42, 1.43, .03)
        axis = shoulder-hands[side]
        parts.append(place(prefix+'_arm', (shoulder+hands[side])*.5, (.20, .18, axis.length),
                           Vector((0, 0, 1)).rotation_difference(axis)))
        lean = Quaternion((0, -1, 0), side*.04)
        hip = bevy(side*.18, .79, -.02)
        parts.append(place(prefix+'_leg', hip - lean @ Vector((0, 0, .34)), LEG_SIZE, lean))
    if weapon == 'sword':
        # player_weapon_rotation at idle: Ry(-yaw) * Rz(roll) * Rx(-pitch).
        grip = Quaternion((0, 0, 1), -.22) @ Quaternion((0, 1, 0), .95) @ Quaternion((1, 0, 0), .58)
    elif weapon == 'bone_bow':
        grip = Quaternion((0, 1, 0), .16) @ Quaternion((0, 0, 1), -.35)
    else:
        grip = Quaternion((0, 1, 0), .05)
    parts.append(place(weapon, hands[1], (.92, .92, .92), grip))
    for part in parts:
        part.parent = root
    root.location = offset
    root.rotation_euler = (0, 0, turn)
    return root


def stage(samples=32):
    scene = bpy.context.scene
    bpy.ops.mesh.primitive_plane_add(size=200)
    ground = bpy.context.object
    ground.name = 'preview ground'
    ground.data.materials.append(hero.material('preview ground', (.05, .052, .058), 0., .95))
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = samples
    scene.cycles.use_denoising = True
    if scene.world is None:
        scene.world = bpy.data.worlds.new('preview world')
    scene.world.use_nodes = False
    scene.world.color = (.20, .20, .20)
    for loc, power, size in [((3, -4, 6), 650, 4), ((-3, -1, 4), 450, 3), ((1, 3, 4), 850, 3)]:
        bpy.ops.object.light_add(type='AREA', location=loc)
        lamp = bpy.context.object
        lamp.data.energy = power
        lamp.data.shape = 'DISK'
        lamp.data.size = size
        lamp.rotation_euler = (Vector((0, 0, 1))-lamp.location).to_track_quat('-Z', 'Y').to_euler()
    bpy.ops.object.camera_add()
    camera = bpy.context.object
    scene.camera = camera
    camera.data.type = 'ORTHO'
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'
    return scene, camera


def shoot(scene, camera, path, eye, target, ortho, size):
    camera.location = Vector(eye)
    camera.rotation_euler = (Vector(target)-camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera.data.ortho_scale = ortho
    scene.render.resolution_x, scene.render.resolution_y = size
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)


def montage(paths, out, columns):
    import numpy as np
    tiles = []
    for path in paths:
        image = bpy.data.images.load(str(path), check_existing=False)
        w, h = image.size
        pixels = np.empty(w*h*4, dtype=np.float32)
        image.pixels.foreach_get(pixels)
        tiles.append(pixels.reshape(h, w, 4))
    rows = [np.concatenate(tiles[i:i+columns], axis=1) for i in range(0, len(tiles), columns)]
    sheet_pixels = np.concatenate(list(reversed(rows)), axis=0)
    result = bpy.data.images.new('preview sheet', width=sheet_pixels.shape[1], height=sheet_pixels.shape[0])
    result.pixels.foreach_set(sheet_pixels.ravel())
    result.filepath_raw = str(out)
    result.file_format = 'PNG'
    result.save()


CLASSES = [('knight', 'hero', 'sword'), ('stalker', 'hero_stalker', 'bone_bow'),
           ('embercaller', 'hero_embercaller', 'ember_staff')]
# camera_follow_translation's CAMERA_OFFSET (-9, 12, 13) in Blender axes.
GAME_EYE = bevy(-9, 12, 13)


def preview():
    PREVIEW.mkdir(parents=True, exist_ok=True)
    views = {'threequarter': (Vector((3.2, -6, 3.4)), 3.3), 'rear': (Vector((-3.2, 6, 3.4)), 3.3),
             'game': (GAME_EYE*.6, 3.3)}
    for label, prefix, weapon in CLASSES:
        reset()
        assemble(prefix, weapon)
        scene, camera = stage()
        for view, (eye, ortho) in views.items():
            shoot(scene, camera, PREVIEW/f'{label}-{view}.png', eye+Vector((0, 0, 1.1)), (0, 0, 1.1),
                  ortho, (620, 760))
    for view in views:
        montage([PREVIEW/f'{label}-{view}.png' for label, _, _ in CLASSES], PREVIEW/f'classes-{view}.png', 3)
    # All three at in-game size: about 60 px tall from the gameplay camera.
    reset()
    right = Vector((0, 0, 1)).cross(GAME_EYE).normalized()
    for index, (_, prefix, weapon) in enumerate(CLASSES):
        assemble(prefix, weapon, right*(index-1)*-1.9)
    scene, camera = stage(48)
    shoot(scene, camera, PREVIEW/'classes-ingame-size.png', GAME_EYE*.6+Vector((0, 0, 1)), (0, 0, 1),
          7.2, (252, 120))
    import numpy as np
    small = bpy.data.images.load(str(PREVIEW/'classes-ingame-size.png'), check_existing=False)
    pixels = np.empty(252*120*4, dtype=np.float32)
    small.pixels.foreach_get(pixels)
    big = np.repeat(np.repeat(pixels.reshape(120, 252, 4), 4, axis=0), 4, axis=1)
    zoom = bpy.data.images.new('ingame zoom', width=1008, height=480)
    zoom.pixels.foreach_set(big.ravel())
    zoom.filepath_raw = str(PREVIEW/'classes-ingame-size-x4.png')
    zoom.file_format = 'PNG'
    zoom.save()
    # Projectiles: close-ups on top, one shared-scale line-up beside a sword below.
    for name in PROJECTILES:
        reset()
        place(name, (0, 0, .0 if name in ('sunder_wave', 'ground_sigil') else .35))
        scene, camera = stage()
        span = {'bone_arrow': 1.1, 'ember_bolt': .62, 'ash_lance': 2., 'sunder_wave': 2.1,
                'ground_sigil': 2.4}[name]
        height = .0 if name in ('sunder_wave', 'ground_sigil') else .35
        shoot(scene, camera, PREVIEW/f'projectile-{name}.png', Vector((2.4, -3.2, 3.6))+Vector((0, 0, height)),
              (0, 0, height), span, (420, 420))
    reset()
    for index, name in enumerate(PROJECTILES):
        place(name, ([-3.6, -2.3, -1.0, 1.0, 3.6][index], 0,
                     .0 if name in ('sunder_wave', 'ground_sigil') else .35))
    place('hero', (-5.0, 0, 0))
    scene, camera = stage()
    shoot(scene, camera, PREVIEW/'projectile-lineup.png', Vector((-.3, -6, 7))+Vector((0, 0, .3)),
          (-.3, 0, .3), 11.2, (2100, 560))
    montage([PREVIEW/f'projectile-{name}.png' for name in PROJECTILES], PREVIEW/'projectile-closeups.png', 5)
    montage([PREVIEW/'projectile-closeups.png', PREVIEW/'projectile-lineup.png'],
            PREVIEW/'projectiles-sheet.png', 1)
    print('PREVIEW', PREVIEW)


def main():
    args = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
    names = [a for a in args if not a.startswith('--')] or list(GENERATORS)
    if '--preview-only' not in args:
        for name in names:
            GENERATORS[name]()
        print('OUTPUTS', ' '.join(name+'.glb' for name in names))
    if '--preview' in args or '--preview-only' in args:
        preview()


if __name__ == '__main__':
    main()
