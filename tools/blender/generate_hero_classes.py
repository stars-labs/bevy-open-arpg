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


def gothic_hood_ring(y, zc, rx, rz, n=48, brow=0., peak=.065, drape=.024, folds=.007):
    """Pointed gothic arch cowl facing -Y: sharp cathedral brow peak, draped sides."""
    pts = []
    for i in range(n):
        ang = TAU*i/n
        ca, sa = math.cos(ang), math.sin(ang)
        top_peak = peak*(max(0., sa)**3.0)
        side_taper = 1.0 - 0.14*(max(0., sa)**1.6) + drape*max(0., -sa)
        fold = folds*math.cos(ang*7.0)*(0.35 + 0.65*abs(ca))
        pts.append(((rx*side_taper + fold)*ca, y - brow*(max(0., sa)**1.8) + fold*0.45,
                    zc + rz*sa + top_peak))
    return pts


def pauldron_shell(name, side, cx, cy, cz, rx, ry, rz, mat, tilt=.24, flair=.04):
    """Curved gothic layered pauldron wrapping smoothly over the deltoid."""
    rows, cols = 12, 20
    grid = []
    for r in range(rows):
        v = r/(rows-1)
        row = []
        for c in range(cols):
            u = (c/(cols-1))*2. - 1.
            ang = u*math.pi*.54
            ridge = .016*(1. - abs(u))*(1. - .35*v)
            wrap_drop = (.055 + .035*v)*(u*u)
            lx = side*(v*rx + flair*(v**2.2) - .026*(u*u)*v)
            ly = math.sin(ang)*ry*(.52 + .50*v)
            lz = (1. - v**1.55)*rz - v*rx*tilt - wrap_drop + ridge*math.cos(ang)
            row.append((cx+lx, cy+ly, cz+lz))
        grid.append(row)
    return sheet(name, grid, mat, .020)


def fingered_hand(name, z_wrist, mat, accent_mat):
    """Five-fingered dark-fantasy gloved/wrapped hand."""
    ellipsoid(name+' palm', (0, -.005, z_wrist-.032), (.046, .035, .042), mat, 16, 12)
    hoop(name+' wrist band', z_wrist, .052, .046, .008, accent_mat)
    for i in range(4):
        fx = -.031 + i*.021
        flen = .058 if i in (1, 2) else .048
        limb(f'{name} finger {i}', [
            (fx, -.016, z_wrist-.058),
            (fx*1.04, -.030, z_wrist-.058-flen*.55),
            (fx*1.02, -.022, z_wrist-.058-flen),
        ], [.011, .009, .006], mat, sides=8, steps=3)
    limb(name+' thumb', [(.038, -.012, z_wrist-.034), (.052, -.032, z_wrist-.064),
         (.040, -.040, z_wrist-.086)], [.013, .010, .0055], mat, sides=8, steps=3)


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
    # Tailored charcoal-leather hunter jerkin with high-segment contour
    hero.loft('hunter jerkin', [(.72, .232, .158, 0), (.93, .202, .138, 0), (1.16, .266, .168, 0),
              (1.38, .325, .186, 0), (1.50, .195, .148, 0)], SHADOW, sides=36, bevel=.008)
    # Intricate sculpted ossuary bone sternum + 5 sweeping bone ribs + cross-baldrics (matches portrait!)
    plate('bone sternum plate', [(0, -.216, 1.00), (.074, -.210, 1.10), (.068, -.206, 1.32),
          (0, -.216, 1.38), (-.068, -.206, 1.32), (-.074, -.210, 1.10)], BONE, .030)
    limb('sternum keel', [(0, -.228, 1.02), (0, -.242, 1.19), (0, -.230, 1.36)],
         [(.012, .010), (.022, .017), (.009, .009)], BONE, sides=10, steps=4)
    for s in [-1, 1]:
        for i in range(5):
            z = 1.05 + i*.068
            limb(f'harness tusk rib {s}_{i}', [
                (s*.042, -.212, z),
                (s*(.155 + .012*i), -.182, z + .028),
                (s*(.258 + .015*i), -.082, z + .046),
                (s*(.220 + .010*i), .085, z + .045),
            ], [.020, .018, .012, .005], BONE, sides=10, steps=4)
    # Crossed leather hunter bandolier & utility straps
    limb('hunter leather baldric', [(-.22, -.15, 1.46), (0, -.222, 1.18), (.22, -.16, .92)],
         [(.024, .010), (.026, .010), (.024, .010)], HIDE, sides=8, steps=4)
    hero.loft('hunter belt', [(.885, .244, .174, 0), (.955, .244, .174, 0)], HIDE, sides=36)
    hoop('hunter belt clasp trim', .952, .246, .176, .007, ASH)
    plate('bone belt toggle', [(-.052, -.212, .89), (.052, -.212, .89), (.064, -.212, .925),
          (.052, -.212, .955), (-.052, -.212, .955), (-.064, -.212, .925)], BONE, .028)
    plate('belt pouch', [(.115, -.212, .79), (.215, -.185, .79), (.225, -.185, .905),
          (.105, -.212, .905)], HIDE, .065)
    limb('pouch bone peg', [(.145, -.224, .87), (.19, -.212, .862)], [.011, .006], BONE, 8, (0, 0, 1), 2)
    # Twin throwing-knives tucked along left hip belt
    for idx, x in enumerate([-.13, -.19]):
        shard(f'belt throwing knife {idx}', (x, -.205, .69), (x-.015, -.205, .86), .016, BONE, 5)
        limb(f'belt knife hilt {idx}', [(x-.015, -.205, .86), (x-.025, -.205, .93)], [.010, .008], HIDE, 6, steps=2)
    # Layered tattered charcoal + teal waist tassets (leaves moving legs crisp)
    for s in [-1, 1]:
        plate(f'hide hip tasset {s}', [(s*.10, -.184, .89), (s*.248, -.112, .88),
              (s*.275, -.128, .54), (s*.195, -.172, .45), (s*.115, -.202, .58)], ASH, .022)
    plate('teal sash tail', [(-.095, -.214, .89), (-.008, -.224, .89), (.008, -.240, .54),
          (-.045, -.245, .42), (-.088, -.236, .58)], TEAL, .016)
    plate('rear hide tasset', [(-.20, .16, .89), (.20, .16, .89), (.18, .195, .58),
          (0, .208, .48), (-.18, .195, .58)], ASH, .022)
    # Draped layered dark-teal hunter neck scarf (rich pleated wraps!)
    skin('hunter wrapped scarf', [
        zring(1.40, .275, .210, .005, n=44, lobes=10, amp=.038, wave=.012),
        zring(1.49, .248, .195, -.005, n=44, lobes=10, amp=.030, wave=.010),
        zring(1.57, .208, .172, -.006, n=44, lobes=8, amp=.022, wave=.006),
    ], TEAL)
    # SHARP POINTED GOTHIC HUNTER COWL (dark charcoal hood with deep shadow + sharp pointed peak!)
    skin('deep hunter hood', [
        gothic_hood_ring(-.085, 1.745, .140, .174, n=48, brow=.00, peak=.032, drape=.016, folds=.005),
        gothic_hood_ring(-.190, 1.750, .164, .196, n=48, brow=.045, peak=.056, drape=.020, folds=.008),
        gothic_hood_ring(-.245, 1.755, .182, .218, n=48, brow=.095, peak=.078, drape=.026, folds=.010),
        gothic_hood_ring(-.172, 1.762, .214, .244, n=48, brow=.042, peak=.065, drape=.022, folds=.008),
        gothic_hood_ring(-.045, 1.772, .232, .258, n=48, brow=.010, peak=.048, drape=.016, folds=.006),
        gothic_hood_ring(.095, 1.780, .216, .240, n=48, brow=.00, peak=.038, drape=.010, folds=.005),
        gothic_hood_ring(.218, 1.775, .166, .184, n=48, brow=.00, peak=.028, drape=.005, folds=.003),
        gothic_hood_ring(.322, 1.755, .106, .118, n=48, brow=.00, peak=.016, drape=.002, folds=.002),
        gothic_hood_ring(.405, 1.720, .048, .052, n=48, brow=.00, peak=.006, drape=.000, folds=.000),
        gothic_hood_ring(.462, 1.690, .012, .014, n=48, brow=.00, peak=.000, drape=.000, folds=.000),
    ], ASH, crease=58.)
    # Deep teal inner hood lining trim
    tube('hood teal edge piping',
         gothic_hood_ring(-.242, 1.755, .182, .218, n=48, brow=.094, peak=.077, drape=.026, folds=.010),
         [.008]*48, TEAL, sides=8, side=(0, 1, 0), closed=True)
    ellipsoid('hood shadow', (0, -.068, 1.745), (.134, .062, .168), SHADOW, 24, 16)
    # Sculpted angular bone-and-charcoal predator half-mask + fierce hunter-jade eyes!
    plate('bone muzzle mask', [
        (-.084, -.186, 1.730), (.084, -.186, 1.730),
        (.076, -.182, 1.645), (.024, -.198, 1.592),
        (-.024, -.198, 1.592), (-.076, -.182, 1.645),
    ], BONE, .026)
    limb('mask nose ridge', [(0, -.206, 1.735), (0, -.212, 1.655), (0, -.202, 1.595)],
         [.012, .014, .008], ASH, 8, steps=3)
    for s in [-1, 1]:
        plate(f'mask breathing slit {s}', [(s*.024, -.194, 1.622), (s*.036, -.194, 1.622),
              (s*.050, -.192, 1.694), (s*.038, -.192, 1.694)], SHADOW, .012)
        shard(f'hunter eye glint {s}', (s*.026, -.172, 1.770), (s*.092, -.166, 1.796), .012, SIGHT, 6)
    # High-res tattered flowing multi-pleat dark-teal hunter cloak (replaces short coarse square)
    rag = [.05, .18, .03, .22, .08, .01, .19, .04, .21, .06, .16, .02, .18, .07, .12, .04, .14]
    cols, rows = len(rag)-1, 12
    grid = []
    for r in range(rows):
        t = r/(rows-1)
        row = []
        for c in range(cols+1):
            u = c/cols*2 - 1
            wave = .034*math.cos(u*5.*math.pi)*(0.25 + 0.75*t)
            row.append((-.03 + u*(.27 + .19*t), .21 + .24*t + wave,
                        1.47 - t*(.92 + rag[c])))
        grid.append(row)
    sheet('ragged teal hunter cloak', grid, TEAL, .018)
    # Quiver slung across the back; bone-fletched arrows rise cleanly over the draw shoulder
    foot, lip = Vector((.14, .40, .93)), Vector((-.21, .37, 1.57))
    axis = (lip-foot).normalized()
    tube('hide quiver', [foot-axis*.01, foot, foot.lerp(lip, .5), lip],
         [.022, .052, .062, .070], HIDE, 16)
    tube('quiver bone lip', [lip-axis*.03, lip+axis*.012], [.078, .082], BONE, 16)
    tube('quiver bone heel', [foot-axis*.022, foot+axis*.04], [.038, .060], BONE, 16)
    tube('quiver sling', [lip+Vector((.02, -.07, -.04)), Vector((-.16, .22, 1.47)),
         Vector((-.2, .0, 1.5))], [(.022, .008)]*3, HIDE, 8, (0, 0, 1))
    across = axis.cross(Vector((0, 1, 0))).normalized()
    for i, (dx, dy, reach) in enumerate([(-.03, -.02, .18), (.028, -.012, .22), (0, .03, .16), (-.012, .008, .24)]):
        base = lip + across*dx + Vector((0, dy, 0))
        tube(f'quivered bone shaft {i}', [base-axis*.05, base+axis*reach], [.009, .009], BONE, 8)
        for turn in [0, 1]:
            vane = across if turn == 0 else Vector((0, 1, 0))
            tube(f'quivered fletching {i}_{turn}',
                 [base+axis*(reach-.10), base+axis*(reach-.05), base+axis*(reach-.006)],
                 [(.008, .003), (.032, .003), (.022, .003)], SIGHT if i == 3 and turn == 0 else TEAL, 6, vane)
    hero.hit_bones()
    export_part('hero_stalker')


def stalker_arm():
    reset()
    hero.loft('hunter sleeve', [(-.29, .054, .052, 0), (-.03, .068, .064, 0), (.22, .080, .074, 0),
              (.36, .060, .058, 0)], SHADOW, sides=28, bevel=.005)
    hero.loft('teal sleeve binding', [(.08, .085, .079, 0), (.18, .087, .081, 0)], TEAL, sides=28)
    ellipsoid('hide elbow guard', (0, .028, -.015), (.074, .068, .076), HIDE, 18, 12)
    hero.loft('bone bracer', [(-.275, .064, .060, -.003), (-.20, .082, .075, -.006),
              (-.07, .086, .078, -.006)], BONE, sides=28, bevel=.005)
    for z, rx, ry in [(-.235, .078, .072), (-.11, .088, .080)]:
        hoop(f'bracer hide strap {z:.2f}', z, rx, ry, .009, HIDE, -.006)
    limb('bracer tusk ridge', [(0, -.084, -.25), (0, -.098, -.16), (0, -.088, -.06)],
         [.011, .016, .005], BONE, sides=8, steps=3)
    fingered_hand('archer glove', -.278, HIDE, ASH)
    hero.normalize(ARM_SIZE, z_center=ARM_CENTER)
    export_part('hero_stalker_arm')


def stalker_leg():
    reset()
    hero.loft('hunter trouser', [(-.30, .060, .066, 0), (.02, .074, .078, 0), (.30, .090, .090, 0),
              (.40, .094, .086, 0)], SHADOW, sides=28, bevel=.004)
    hero.loft('teal thigh wrap', [(.16, .090, .092, 0), (.24, .093, .093, 0)], TEAL, sides=28)
    hero.loft('tall soft hide boot', [(-.385, .066, .070, 0), (-.25, .073, .079, 0),
              (-.07, .085, .089, 0), (-.03, .093, .096, 0)], HIDE, sides=28, bevel=.005)
    for z, rx, ry in [(-.27, .075, .081), (-.15, .082, .087)]:
        hoop(f'boot binding strap {z:.2f}', z, rx, ry, .009, ASH)
    plate('bone knee cap', [(-.058, -.108, -.03), (.058, -.108, -.03), (.072, -.112, .06),
          (0, -.120, .140), (-.072, -.112, .06)], BONE, .036)
    limb('bone shin splint', [(0, -.092, -.07), (0, -.094, -.20), (0, -.082, -.33)],
         [(.024, .012), (.018, .011), (.008, .006)], BONE, sides=8, steps=3)
    ellipsoid('soft boot foot', (0, -.065, -.38), (.082, .152, .048), HIDE, 22, 14)
    limb('boot toe seam', [(-.05, -.16, -.356), (0, -.20, -.360), (.05, -.16, -.356)],
         [.008, .009, .008], ASH, sides=8, side=(0, 0, 1), steps=3)
    hero.normalize(LEG_SIZE)
    export_part('hero_stalker_leg')


def stalker_mantle():
    reset()
    # Pleated dark-teal hunter shoulder cowl + dark charcoal leather yoke
    skin('teal shoulder cowl', [zring(-.118, .482, .242, .012, n=48, lobes=11, amp=.036, wave=.020),
         zring(-.03, .445, .232, .010, n=48, lobes=11, amp=.026, wave=.014),
         zring(.05, .318, .204, .005, n=48), zring(.115, .232, .178, n=48),
         zring(.15, .206, .166, n=48)], TEAL)
    skin('hide collar yoke', [zring(.03, .348, .216, .005, n=44), zring(.085, .270, .192, n=44),
         zring(.128, .228, .178, n=44)], ASH)
    # Sculpted jagged ossuary bone-carapace pauldron on bow arm (matches portrait!)
    pauldron_shell('bone carapace upper', 1, .22, .008, .055, .29, .215, .105, BONE, tilt=.24, flair=.05)
    pauldron_shell('bone carapace lower', 1, .28, .010, -.015, .29, .198, .082, BONE, tilt=.30, flair=.04)
    for y in [-.095, 0.0, .095]:
        limb(f'shoulder bone tusk {y:.2f}', [(.38, y, .05), (.50, y*1.25, .12), (.56, y*1.4, .22)],
             [.020, .015, .003], BONE, sides=8, side=(0, 1, 0), steps=4)
    limb('scale lashing', [(.28, -.165, -.02), (.42, -.175, .02), (.52, -.11, -.04)],
         [(.020, .008)]*3, HIDE, sides=8, side=(0, 0, 1), steps=3)
    # Lighter dark-leather guard on draw arm
    pauldron_shell('draw arm leather guard', -1, -.26, .010, .025, .22, .180, .072, HIDE, tilt=.28, flair=.02)
    limb('bone cowl toggle', [(-.072, -.236, .0), (0, -.252, .006), (.072, -.236, .0)],
         [.007, .020, .007], BONE, sides=8, side=(0, 0, 1), steps=3)
    for s in [-1, 1]:
        limb(f'toggle cord {s}', [(s*.03, -.238, .0), (s*.07, -.226, .06), (s*.10, -.196, .10)],
             [.007, .007, .006], HIDE, sides=6, side=(0, 0, 1), steps=2)
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
    # High-poly pleated deep-violet liturgical robe
    skin('layered violet robe', [zring(z, rx, ry, 0, 56, 14, amp, wave)
         for z, rx, ry, amp, wave in ROBE], VIOLET)
    hoop('robe hem gilding', .30, .373, .343, .011, GOLD, lobes=14, amp=.066, wave=.026)
    # Open-fronted tattered charcoal ash coat
    gap, cols, rows = math.radians(78), 36, 12
    grid = []
    for r in range(rows):
        f = r/(rows-1)
        row = []
        for c in range(cols+1):
            t = c/cols
            angle = -math.pi/2 + gap/2 + t*(TAU-gap)
            hem = .54 - .18*math.sin(math.pi*t) - (.042 if c % 2 else 0.)
            z = 1.46 + (hem-1.46)*f
            rx, ry = robe_at(z)
            swell = .03 + .022*f
            row.append(((rx+swell)*math.cos(angle), (ry+swell)*math.sin(angle), z))
        grid.append(row)
    sheet('open ash coat', grid, COAT, .020)
    for c in [0, cols]:
        limb('coat front gilding', [grid[r][c] for r in range(rows)], [.011]*rows, GOLD, 8, (0, 1, 0), 2)
    # Ornate draped liturgical stole with radiant solar-ember sunburst pectoral medallion (matches portrait!)
    grid = []
    for r in range(10):
        z = 1.42 - r*.096
        _, ry = robe_at(z)
        half = .072 + .004*r
        grid.append([(u*half, -ry-.016-.004*abs(u), z - (.055 if r == 9 and u == 0 else 0))
                     for u in [-1, 0, 1]])
    sheet('ash stole', grid, COAT, .016)
    for u in [0, 2]:
        limb('stole gilding', [grid[r][u] for r in range(10)], [.008]*10, GOLD, 6, (0, 1, 0), 2)
    shard('stole gold tassel', (0, grid[9][1][1]-.004, .44), (0, grid[9][1][1]-.004, .56), .024, GOLD, 6, .7)
    sy = -robe_at(1.24)[1]-.034
    # Radiant 12-ray gothic solar-ember starburst medallion on chest!
    tube('solar sigil outer ring', [(.078*math.cos(TAU*i/28), sy, 1.24+.078*math.sin(TAU*i/28)) for i in range(28)],
         [.008]*28, GOLD, 8, (0, 1, 0), closed=True)
    tube('solar sigil ember ring', [(.052*math.cos(TAU*i/24), sy-.004, 1.24+.052*math.sin(TAU*i/24)) for i in range(24)],
         [.007]*24, EMBER, 6, (0, 1, 0), closed=True)
    shard('solar sigil core crystal', (0, sy-.008, 1.285), (0, sy-.008, 1.195), .022, CRYSTAL, 6)
    for i in range(12):
        ang = TAU*i/12
        r0, r1 = .078, .135 if i % 2 == 0 else .108
        shard(f'solar sigil ray {i}',
              (r1*math.cos(ang), sy-.004, 1.24+r1*math.sin(ang)),
              (r0*math.cos(ang), sy+.002, 1.24+r0*math.sin(ang)), .008, GOLD, 4)
    # Cord girdle with hanging reliquary charms + dangling solar ring
    bx, by = robe_at(.98)
    hero.loft('hide girdle', [(.95, bx+.052, by+.052, 0), (1.01, bx+.052, by+.052, 0)], HIDE, sides=36)
    hoop('girdle gilding', 1.012, bx+.054, by+.054, .009, GOLD)
    for angle, drop, kind in [(-2.38, .17, 'vial'), (-2.02, .25, 'ember'), (-.80, .15, 'tag'),
                              (-1.13, .22, 'vial'), (-.42, .19, 'ember')]:
        charm('reliquary', ((bx+.062)*math.cos(angle), (by+.062)*math.sin(angle), .955), drop, kind)
    # TALL REGAL GOTHIC CATHEDRAL-ARCH COWL HOOD (replaces cartoon cone hat!)
    skin('tall gothic violet cowl', [
        gothic_hood_ring(-.075, 1.745, .142, .182, n=48, brow=.00, peak=.055, drape=.016, folds=.005),
        gothic_hood_ring(-.182, 1.752, .165, .204, n=48, brow=.035, peak=.095, drape=.022, folds=.008),
        gothic_hood_ring(-.238, 1.758, .180, .225, n=48, brow=.080, peak=.135, drape=.028, folds=.010),
        gothic_hood_ring(-.165, 1.766, .212, .248, n=48, brow=.040, peak=.118, drape=.022, folds=.008),
        gothic_hood_ring(-.042, 1.778, .228, .260, n=48, brow=.010, peak=.092, drape=.015, folds=.006),
        gothic_hood_ring(.095, 1.784, .208, .236, n=48, brow=.00, peak=.065, drape=.010, folds=.004),
        gothic_hood_ring(.215, 1.774, .158, .178, n=48, brow=.00, peak=.042, drape=.005, folds=.003),
        gothic_hood_ring(.315, 1.746, .098, .112, n=48, brow=.00, peak=.022, drape=.002, folds=.002),
        gothic_hood_ring(.395, 1.705, .044, .048, n=48, brow=.00, peak=.008, drape=.000, folds=.000),
        gothic_hood_ring(.442, 1.675, .012, .014, n=48, brow=.00, peak=.000, drape=.000, folds=.000),
    ], VIOLET, crease=58.)
    tube('cowl gold edge trim',
         gothic_hood_ring(-.236, 1.758, .181, .226, n=48, brow=.079, peak=.134, drape=.028, folds=.010),
         [.008]*48, GOLD, sides=8, side=(0, 1, 0), closed=True)
    # Shadowed inner cowl depth + sculpted gaunt ash-pale priestly visage & molten ember eyes!
    ellipsoid('cowl inner shadow', (0, -.055, 1.745), (.134, .060, .172), SHADOW, 24, 16)
    # Sculpted gaunt aristocratic priestly mask/visage (high cheekbones, sharp brow, narrow jaw)
    v_rows, v_cols = 16, 20
    v_grid = []
    for r in range(v_rows):
        v = r/(v_rows-1)
        z = 1.748 + .156 - v*.308
        half_w = (.076 + .042*math.sin((v/.36)*math.pi*.5)) if v < .36 else (.118*((1.-(v-.36)/.64)**.72) + .018*(1.-(v-.36)/.64))
        row = []
        for c in range(v_cols):
            u = (c/(v_cols-1))*2. - 1.
            au = abs(u)
            wrap = (au**1.75)*.078
            brow = .018*math.exp(-((v-.22)/.085)**2)*(1.-.35*au)
            nose = .028*math.exp(-(u/.17)**2)*math.exp(-((v-.48)/.24)**2)
            cheek = .020*math.exp(-((au-.55)/.22)**2)*math.exp(-((v-.47)/.15)**2)
            eye_h = .026*math.exp(-((au-.40)/.16)**2)*math.exp(-((v-.35)/.085)**2)
            gaunt = .016*math.exp(-((au-.50)/.20)**2)*math.exp(-((v-.70)/.14)**2)
            y = -.184 + wrap - brow - nose - cheek + eye_h + gaunt
            row.append((u*half_w, y, z))
        v_grid.append(row)
    sheet('ashpriest gaunt visage', v_grid, BONE, .016)
    for s in [-1, 1]:
        ellipsoid(f'priest eye hollow {s}', (s*.046, -.176, 1.798), (.025, .015, .018), SHADOW, 14, 10)
        shard(f'ember eye glint {s}', (s*.026, -.192, 1.796), (s*.066, -.186, 1.804), .007, CRYSTAL, 6)
    # Delicate gold brow halo-circlet with radiant sun-crest resting across the forehead!
    brow_arc = [(math.sin(t)*.155, -.045 - math.cos(t)*.182, 1.895 - .018*abs(math.sin(t)))
                for t in [(-1.65 + 3.30*i/26) for i in range(27)]]
    tube('ashpriest brow circlet', brow_arc, [.008]*len(brow_arc), GOLD, sides=8, side=(0, 0, 1))
    tube('forehead solar halo', [(.028*math.cos(TAU*i/16), -.226, 1.895+.028*math.sin(TAU*i/16)) for i in range(16)],
         [.005]*16, GOLD, sides=6, side=(0, 1, 0), closed=True)
    shard('circlet ember gem', (0, -.230, 1.918), (0, -.230, 1.872), .012, CRYSTAL, 6)
    for i in range(7):
        ang = (i - 3)*.44
        bx = math.sin(ang)*.165
        by = -.040 - math.cos(ang)*.186
        rise = .16 - .022*abs(i - 3)
        shard(f'circlet solar ray {i}', (bx*1.12, by*1.06, 1.90 + rise), (bx, by, 1.88), .012, GOLD, 5)
    # FLOATING ORBITING MOLTEN EMBER CRYSTALS around the priest (iconic silhouette from portrait!)
    for idx, (cx, cy, cz, sz) in enumerate([
        (-.44, -.16, 1.24, .048),
        (-.56, -.02, 1.44, .038),
        (-.36, .18, 1.66, .034),
        (.38, .18, 1.62, .032),
    ]):
        shard(f'orbiting ember crystal {idx}', (cx, cy, cz+sz*1.4), (cx, cy, cz-sz*1.2), sz, CRYSTAL, 6, belly=.48)
        hoop(f'orbiting crystal halo {idx}', cz, sz*1.35, sz*1.35, .005, GOLD, cy=cy, cx=cx)
    # High-res pleated liturgical violet back mantle panel
    cols, rows = 14, 12
    grid = []
    for r in range(rows):
        t = r/(rows-1)
        z = 1.46 - t*1.16
        _, ry = robe_at(max(z, .27))
        half = .18 + .16*t
        row = []
        for c in range(cols+1):
            u = c/cols*2 - 1
            wave = .028*math.cos(u*4.*math.pi)*(0.3 + 0.7*t)
            tatter = .055*math.cos(u*5.*math.pi)*(t**2)
            row.append((u*half, ry + .055 + .14*t + wave, z + tatter))
        grid.append(row)
    sheet('violet liturgical back mantle', grid, VIOLET, .016)
    hero.hit_bones()
    export_part('hero_embercaller')


def embercaller_arm():
    reset()
    skin('violet bell sleeve', [zring(-.262, .136, .120, n=32, lobes=10, amp=.040, wave=.010),
         zring(-.17, .122, .108, n=32, lobes=10, amp=.024, wave=.006), zring(.02, .094, .088, n=32),
         zring(.25, .088, .084, n=32), zring(.36, .066, .066, n=32)], VIOLET)
    hoop('sleeve cuff gilding', -.245, .135, .119, .008, GOLD, lobes=10, amp=.035, wave=.010)
    hero.loft('ash sleeve cap', [(.12, .099, .093, 0), (.25, .096, .090, 0), (.36, .080, .078, 0)], COAT, sides=28)
    hoop('sleeve cap gilding', .128, .100, .094, .008, GOLD)
    hero.loft('dark under sleeve', [(-.300, .050, .048, 0), (-.245, .062, .058, 0)], SHADOW, sides=24)
    fingered_hand('ash caster hand', -.278, BONE, GOLD)
    hero.normalize(ARM_SIZE, z_center=ARM_CENTER)
    export_part('hero_embercaller_arm')


def embercaller_leg():
    reset()
    hero.loft('ashpriest hose', [(-.30, .058, .064, 0), (.02, .072, .076, 0), (.31, .086, .086, 0),
              (.40, .090, .084, 0)], SHADOW, sides=26)
    skin('violet under robe wrap', [zring(-.235, .083, .088, n=28, lobes=6, wave=.012),
         zring(-.05, .084, .087, n=28), zring(.21, .092, .092, n=28)], VIOLET)
    hero.loft('soft ash boot', [(-.39, .064, .068, 0), (-.30, .071, .077, 0), (-.20, .079, .084, 0)], HIDE, sides=26)
    hoop('boot gold anklet', -.27, .075, .081, .008, GOLD)
    ellipsoid('pointed slipper', (0, -.07, -.386), (.078, .156, .044), HIDE, 20, 12)
    shard('gilded slipper toe', (0, -.262, -.368), (0, -.15, -.39), .040, GOLD, 6, .7, .62, (1, 0, 0))
    hero.normalize(LEG_SIZE)
    export_part('hero_embercaller_leg')


def embercaller_mantle():
    reset()
    # High-poly pleated liturgical shoulder shawl + draped gothic violet capelet + gold trim
    skin('ash shoulder shawl', [zring(-.15, .53, .266, .012, n=48, lobes=12, amp=.038, wave=.026),
         zring(-.05, .47, .248, .010, n=48, lobes=12, amp=.028, wave=.016),
         zring(.04, .32, .206, .005, n=48), zring(.10, .232, .178, n=48)], COAT)
    hoop('shawl hem gilding', -.142, .531, .267, .0095, GOLD, .012, lobes=12, amp=.036, wave=.026)
    skin('violet upper capelet', [zring(-.035, .40, .236, .008, n=48, lobes=10, amp=.028, wave=-.014),
         zring(.03, .314, .208, .004, n=48), zring(.09, .244, .184, n=48), zring(.125, .222, .172, n=48)], VIOLET)
    hoop('capelet hem gilding', -.03, .401, .237, .007, GOLD, .008, lobes=10, amp=.026, wave=-.014)
    # Draped gothic layered gilded shoulder wings
    for s in [-1, 1]:
        pauldron_shell(f'liturgical shoulder wing {s}', s, s*.25, .012, .035, .25, .195, .080, VIOLET, tilt=.26, flair=.03)
        limb(f'liturgical shoulder gold trim {s}', [
            (s*.26, -.08, .095), (s*.42, .00, .065), (s*.51, .06, -.035)
        ], [.010, .012, .008], GOLD, sides=8, steps=4)
        shard(f'shawl gold drop {s}', (s*.52, .012, -.25), (s*.52, .012, -.14), .022, GOLD, 6, .7)
    limb('gold clasp chain', [(-.15, -.236, .0), (0, -.266, -.038), (.15, -.236, .0)],
         [.009, .009, .009], GOLD, sides=8, side=(0, 0, 1), steps=4)
    shard('ember clasp stone', (0, -.276, -.088), (0, -.268, .0), .028, CRYSTAL, 6, .55, .5, (1, 0, 0))
    hero.normalize(MANTLE_SIZE)
    export_part('hero_embercaller_mantle')


# ---------------------------------------------------------------- weapons

def bone_bow():
    reset()
    # Stave on +Z, back toward -Y (forward); glowing hunter-sight bowstring at +Y!
    upper = [(-.012, .12), (.03, .36), (.13, .62), (.218, .82), (.24, .93), (.205, 1.0), (.14, 1.05)]
    spine = [(0, y, -z) for y, z in reversed(upper)] + [(0, -.012, 0)] + [(0, y, z) for y, z in upper]
    half = [(.036, .034), (.05, .024), (.046, .019), (.038, .016), (.03, .014), (.022, .012), (.012, .008)]
    limb('bone recurve stave', spine, list(reversed(half)) + [(.034, .036)] + half, BONE, 12)
    for s in [-1, 1]:
        for z, r in [(.36, .058), (.62, .054), (.82, .045)]:
            y = dict((round(zz, 2), yy) for yy, zz in upper)[z]
            ellipsoid(f'stave vertebra knuckle {s}_{z:.2f}', (0, y, s*z), (r, .03, .034), BONE, 18, 12)
        limb(f'riser tusk {s}', [(0, -.01, s*.14), (0, -.085, s*.20), (0, -.165, s*.30)],
             [.024, .017, .004], BONE, 8)
        for z in [.135, .185]:
            hoop(f'sinew riser lashing {s}_{z:.2f}', s*z, .043, .04, .008, SINEW, -.006)
        hoop(f'sinew nock lashing {s}', s*.985, .03, .018, .007, SINEW, .215)
        hoop(f'teal grip ribbon {s}', s*.105, .041, .043, .009, TEAL, -.012)
        tube(f'sinew bowstring loop {s}', [(0, .252, s*.93), (0, .23, s*.995), (0, .156, s*1.046)],
             [.008, .008, .008], SIGHT, 8)
    tube('hide bow grip', [(0, -.012, -.1), (0, -.014, 0), (0, -.012, .1)], [.039, .041, .039], HIDE, 14)
    # Luminous spectral hunter-sight bowstring (matches portrait!)
    tube('taut spectral bowstring', [(0, .252, -.93), (0, .252, .93)], [.008, .008], SIGHT, 8)
    tube('string serving', [(0, .252, -.07), (0, .252, .07)], [.012, .012], TEAL, 8)
    export_single('bone_bow')


def ember_staff():
    reset()
    # 2.52 long: foot at -1.008, grip at the origin (40 % up), ornate lantern cage tips at 1.54.
    wobble = [(0, 0, -.9), (.012, -.008, -.6), (-.01, .006, -.3), (0, 0, -.1), (0, 0, .16),
              (.011, .006, .42), (-.013, .009, .7), (.01, -.006, .95), (0, 0, 1.13)]
    limb('charred staff shaft', wobble, [.027, .03, .031, .034, .034, .031, .03, .031, .036], WOOD, 14)
    for x, y, z, r in [(.03, -.01, -.52, .03), (-.03, .012, .58, .028), (.028, .014, .86, .026)]:
        limb('shaft burl', [(x*.3, y*.3, z-.05), (x, y, z), (x*.4, y*.4, z+.06)],
             [.012, r, .01], WOOD, 8)
    tube('gilded foot spike', [(0, 0, -1.008), (0, 0, -.955), (0, 0, -.9), (0, 0, -.87)],
         [.004, .024, .032, .03], GOLD, 12)
    tube('violet grip wrap', [(0, 0, -.1), (0, 0, .02), (0, 0, .15)], [.039, .041, .039], VIOLET, 14)
    for z in [-.105, .155]:
        hoop(f'grip gilding {z:.2f}', z, .041, .041, .009, GOLD)
    tube('gilded head collar', [(0, 0, 1.09), (0, 0, 1.12), (0, 0, 1.17), (0, 0, 1.185)],
         [.036, .048, .054, .042], GOLD, 14)
    # 6-ribbed gothic iron-and-gold ember brazier cage cradling a blazing molten core!
    for i in range(6):
        ang = i*TAU/6
        d = Vector((math.cos(ang), math.sin(ang), 0))
        path = [d*r + Vector((0, 0, z)) for r, z in
                [(.040, 1.16), (.118, 1.22), (.162, 1.33), (.142, 1.44), (.082, 1.52), (.028, 1.57)]]
        limb(f'gothic brazier rib {i}', path, [.018, .018, .016, .013, .009, .003], WOOD, 8, d.cross(Vector((0, 0, 1))))
        hoop(f'brazier gold band {i}', 1.33, .013, .013, .006, GOLD, .162*d.y, cx=.162*d.x)
    hoop('brazier equatorial ring', 1.33, .165, .165, .009, GOLD)
    shard('caged ember crystal', (0, 0, 1.50), (0, 0, 1.19), .088, CRYSTAL, 8, .44)
    shard('ember crystal splinter a', (.058, .022, 1.39), (.034, .014, 1.23), .026, CRYSTAL, 6, .4)
    shard('ember crystal splinter b', (-.052, -.032, 1.36), (-.030, -.020, 1.225), .024, EMBER, 6, .4)
    for ang, z in [(.6, .97), (2.7, 1.03), (4.6, .99)]:
        d = Vector((math.cos(ang), math.sin(ang), 0))*.034
        shard(f'ember shaft vein {ang:.1f}', d+Vector((0, 0, z+.06)), d+Vector((0, 0, z-.055)), .008, CRYSTAL, 4, .4, .5, d)
    charm('staff reliquary', (-.052, .014, 1.125), .14, 'ember')
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
