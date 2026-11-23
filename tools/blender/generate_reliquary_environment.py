"""Authored high-detail dark-fantasy wilderness and sanctum props.
Run with Blender --background --factory-startup --python this_file.
Deterministic seed 41.
"""
import math
import random
from pathlib import Path
import bmesh
import bpy
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'assets/models'
PREVIEW = ROOT / 'target/validation/asset-replacement'
PREVIEW.mkdir(parents=True, exist_ok=True)
rng = random.Random(41)


def material(name, color, metallic=0.0, roughness=0.76, emission=0.0):
    m = bpy.data.materials.new(name)
    m.diffuse_color = (*color, 1.0)
    m.use_nodes = True
    b = m.node_tree.nodes.get('Principled BSDF')
    b.inputs['Base Color'].default_value = (*color, 1.0)
    b.inputs['Metallic'].default_value = metallic
    b.inputs['Roughness'].default_value = roughness
    b.inputs['Emission Color'].default_value = (*color, 1.0)
    b.inputs['Emission Strength'].default_value = emission
    return m


STONE = material('reliquary stone', (0.23, 0.24, 0.26), 0.04, 0.84)
DARK = material('hollow obsidian', (0.09, 0.105, 0.125), 0.08, 0.72)
BARK = material('charred staff wood', (0.145, 0.098, 0.072), 0.02, 0.88)
IRON = material('cold worn steel', (0.15, 0.175, 0.20), 0.74, 0.44)
GOLD = material('dull relic gold', (0.44, 0.30, 0.13), 0.68, 0.42)
BONE = material('aged bone', (0.58, 0.53, 0.44), 0.0, 0.72)
GLOW = material('bound ember', (0.94, 0.30, 0.06), 0.04, 0.35, 1.45)
CRYSTAL = material('hollow crystal', (0.10, 0.44, 0.52), 0.18, 0.22, 0.65)
CRYSTAL_CORE = material('hollow crystal', (0.20, 0.68, 0.78), 0.10, 0.16, 1.25)


def finish(obj, name, mat, smooth=False):
    obj.name = name
    obj.data.materials.append(mat)
    if smooth:
        for poly in obj.data.polygons:
            poly.use_smooth = True
    return obj


def _set_bm_smooth(obj, angle_deg=42.0):
    mesh = obj.data
    cos_thresh = math.cos(math.radians(angle_deg))
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bm.edges.ensure_lookup_table()
    for poly in bm.faces:
        poly.smooth = True
    for edge in bm.edges:
        if len(edge.link_faces) == 2:
            n0 = edge.link_faces[0].normal
            n1 = edge.link_faces[1].normal
            if n0.length > 1e-6 and n1.length > 1e-6 and n0.dot(n1) < cos_thresh:
                edge.smooth = False
    bm.to_mesh(mesh)
    bm.free()


def block(name, location, size, mat, bevel=0.025, rot=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rot)
    obj = bpy.context.object
    obj.dimensions = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    finish(obj, name, mat)
    if bevel:
        mod = obj.modifiers.new('chipped masonry edges', 'BEVEL')
        mod.width = min(bevel, min(size) * 0.28)
        mod.segments = 2
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def cylinder_seg(name, a, b, r1, r2, mat, sides=12, bevel_caps=False):
    a, b = Vector(a), Vector(b)
    diff = b - a
    length = diff.length
    if length < 1e-4:
        return None
    bpy.ops.mesh.primitive_cone_add(
        vertices=sides,
        radius1=r1,
        radius2=r2,
        depth=length,
        location=(a + b) * 0.5,
    )
    obj = bpy.context.object
    obj.rotation_euler = diff.to_track_quat('Z', 'Y').to_euler()
    finish(obj, name, mat)
    if bevel_caps:
        mod = obj.modifiers.new('cap bevel', 'BEVEL')
        mod.width = min(r1, r2, length) * 0.14
        mod.segments = 2
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def catmull_rom(points, samples_per_seg=4):
    if len(points) < 2:
        return [Vector(p) for p in points]
    pts = [Vector(p) for p in points]
    padded = [pts[0] + (pts[0] - pts[1])] + pts + [pts[-1] + (pts[-1] - pts[-2])]
    out = []
    for i in range(1, len(padded) - 2):
        p0, p1, p2, p3 = padded[i - 1], padded[i], padded[i + 1], padded[i + 2]
        steps = samples_per_seg
        for s in range(steps):
            t = s / steps
            t2 = t * t
            t3 = t2 * t
            pos = 0.5 * (
                (2.0 * p1)
                + (-p0 + p2) * t
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
            )
            out.append(pos)
    out.append(pts[-1])
    return out


def spline_tube(
    name,
    points,
    radii,
    mat,
    sides=12,
    samples_per_seg=3,
    flute_amp=0.0,
    flute_freq=5,
    twist=0.0,
    cap_start=True,
    cap_end=True,
):
    """Build a continuous curved organic mesh tube along control points."""
    sampled = catmull_rom(points, samples_per_seg=samples_per_seg)
    n_pts = len(sampled)
    bm = bmesh.new()
    rings = []

    prev_up = Vector((1.0, 0.0, 0.0))
    for i, center in enumerate(sampled):
        u = i / max(1, n_pts - 1)
        # Piecewise linear interpolation over input radii
        rf = u * (len(radii) - 1)
        ri = min(int(rf), len(radii) - 2)
        rt = rf - ri
        r = radii[ri] * (1.0 - rt) + radii[ri + 1] * rt

        if i == 0:
            tangent = (sampled[1] - sampled[0]).normalized()
        elif i == n_pts - 1:
            tangent = (sampled[-1] - sampled[-2]).normalized()
        else:
            tangent = (sampled[i + 1] - sampled[i - 1]).normalized()
        if tangent.length < 1e-6:
            tangent = Vector((0.0, 0.0, 1.0))

        # Parallel transport frame to prevent kinks
        if abs(tangent.dot(prev_up)) > 0.94:
            ref = Vector((0.0, 1.0, 0.0)) if abs(tangent.y) < 0.8 else Vector((1.0, 0.0, 0.0))
            side_vec = tangent.cross(ref).normalized()
        else:
            side_vec = prev_up - tangent * prev_up.dot(tangent)
            side_vec.normalize()
        binorm = tangent.cross(side_vec).normalized()
        prev_up = side_vec

        ring = []
        for s in range(sides):
            ang = s * math.tau / sides + u * twist
            flute = 1.0 + flute_amp * math.sin(s * flute_freq * math.tau / sides + u * 4.5)
            rad = max(0.004, r * flute)
            offset = (side_vec * math.cos(ang) + binorm * math.sin(ang)) * rad
            ring.append(bm.verts.new(center + offset))
        rings.append(ring)

    for i in range(n_pts - 1):
        r0, r1 = rings[i], rings[i + 1]
        for s in range(sides):
            sn = (s + 1) % sides
            bm.faces.new((r0[s], r0[sn], r1[sn], r1[s]))

    if cap_start:
        c0 = bm.verts.new(sampled[0])
        for s in range(sides):
            sn = (s + 1) % sides
            bm.faces.new((rings[0][sn], rings[0][s], c0))
    if cap_end:
        c1 = bm.verts.new(sampled[-1])
        for s in range(sides):
            sn = (s + 1) % sides
            bm.faces.new((rings[-1][s], rings[-1][sn], c1))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    finish(obj, name, mat, smooth=True)
    return obj


def sculpted_crag(name, loc, scale, mat=STONE, subdivisions=3, seed_offset=0, strata=5.5, sharpness=0.22):
    """Angular faceted dark-fantasy slate/basalt crag with planar rock faces."""
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=subdivisions, radius=1.0, location=loc)
    obj = bpy.context.object
    p1 = 1.7 + seed_offset * 0.91
    p2 = 2.9 + seed_offset * 1.37
    p3 = 4.1 + seed_offset * 0.53
    # Deterministic cleavage plane normals to carve chiseled rock faces
    planes = [
        (Vector((0.68, 0.42, 0.60)).normalized(), 0.66 + 0.08 * math.sin(p1)),
        (Vector((-0.62, 0.51, 0.58)).normalized(), 0.64 + 0.08 * math.cos(p2)),
        (Vector((0.18, -0.74, 0.64)).normalized(), 0.68 + 0.07 * math.sin(p3)),
        (Vector((-0.55, -0.58, 0.52)).normalized(), 0.72 + 0.06 * math.cos(p1)),
        (Vector((0.76, -0.30, 0.25)).normalized(), 0.74 + 0.07 * math.sin(p2)),
    ]
    for v in obj.data.vertices:
        co = v.co.copy()
        # Broad irregular displacement
        n_low = (
            math.sin(co.x * 2.4 + p1) * math.cos(co.y * 2.2 + p2) * 0.14
            + math.sin(co.z * 2.5 - co.x * 1.6 + p3) * 0.11
        )
        co *= 1.0 + n_low
        # Broaden base slightly and taper cleanly toward angular peak (no waist pinch!)
        taper = 1.14 - max(0.0, co.z + 0.35) * 0.34
        co.x *= taper
        co.y *= taper
        # Carve chiseled planar cleavage faces like broken quarry slate
        for normal, dist in planes:
            d = co.dot(normal)
            if d > dist:
                co -= normal * (d - dist) * 0.72
        # Subtle stepped fracture micro-detail
        fracture = math.sin(co.x * strata + co.z * 2.8 + p2) * sharpness * 0.28
        co.x += fracture
        co.y -= fracture * 0.6
        co.z = max(co.z, -0.38)
        v.co = co
    obj.scale = scale
    obj.rotation_euler.z = rng.random() * math.tau
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    finish(obj, name, mat)
    # Crisp angular rock shading (low auto-smooth angle preserves chiseled facets)
    _set_bm_smooth(obj, angle_deg=18.0)
    return obj


def basalt_prism(name, base, height, radius, tilt_xy, mat=DARK, sides=6):
    """Hexagonal fractured basalt pillar with angled sheared cap."""
    bm = bmesh.new()
    top_center = Vector((base[0] + tilt_xy[0], base[1] + tilt_xy[1], base[2] + height))
    base_center = Vector(base)
    rings = []
    segs = 4
    for k in range(segs + 1):
        t = k / segs
        c = base_center.lerp(top_center, t)
        rad = radius * (1.06 - 0.18 * t)
        ring = []
        for s in range(sides):
            ang = s * math.tau / sides + 0.15
            jitter = 1.0 + 0.08 * math.sin(s * 2.3 + k * 1.7)
            pt = Vector((
                c.x + math.cos(ang) * rad * jitter,
                c.y + math.sin(ang) * rad * jitter,
                c.z + (0.09 * height * math.cos(ang + 0.6) if k == segs else 0.0),
            ))
            ring.append(bm.verts.new(pt))
        rings.append(ring)
    for k in range(segs):
        for s in range(sides):
            sn = (s + 1) % sides
            bm.faces.new((rings[k][s], rings[k][sn], rings[k + 1][sn], rings[k + 1][s]))
    top_v = bm.verts.new(top_center + Vector((0.0, 0.0, 0.06 * height)))
    bot_v = bm.verts.new(base_center)
    for s in range(sides):
        sn = (s + 1) % sides
        bm.faces.new((rings[-1][s], rings[-1][sn], top_v))
        bm.faces.new((rings[0][sn], rings[0][s], bot_v))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    finish(obj, name, mat)
    return obj


def crystal_spire(name, base, tip, r_base, r_mid, mat=CRYSTAL, sides=6):
    """Sharp faceted arcane quartz spire with multi-stage crown."""
    base = Vector(base)
    tip = Vector(tip)
    axis = tip - base
    length = axis.length
    if length < 1e-4:
        return None
    quat = axis.normalized().to_track_quat('Z', 'Y')
    rot_m = quat.to_matrix()

    bm = bmesh.new()
    profile = [
        (0.00, r_base * 0.92, 0.0),
        (0.18, r_base * 1.04, 0.04),
        (0.68, r_mid, 0.06),
        (0.84, r_mid * 0.72, 0.08),
    ]
    rings = []
    for t, rad, twist in profile:
        ring = []
        for s in range(sides):
            ang = s * math.tau / sides + twist
            # Slightly asymmetric crystal faces look natural instead of sterile
            asym = 1.0 + 0.11 * math.cos(ang * 2.0 + 0.5)
            local = Vector((math.cos(ang) * rad * asym, math.sin(ang) * rad * asym, t * length))
            ring.append(bm.verts.new(base + rot_m @ local))
        rings.append(ring)

    for k in range(len(rings) - 1):
        for s in range(sides):
            sn = (s + 1) % sides
            bm.faces.new((rings[k][s], rings[k][sn], rings[k + 1][sn], rings[k + 1][s]))

    apex = bm.verts.new(tip)
    bot = bm.verts.new(base)
    for s in range(sides):
        sn = (s + 1) % sides
        bm.faces.new((rings[-1][s], rings[-1][sn], apex))
        bm.faces.new((rings[0][sn], rings[0][s], bot))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    finish(obj, name, mat)
    return obj


def weathered_skull(name, loc, scale=0.14, yaw=0.0):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=scale, location=loc)
    cranium = bpy.context.object
    for v in cranium.data.vertices:
        v.co.y *= 1.16
        v.co.z *= 0.92
        if v.co.z < -scale * 0.25:
            v.co.x *= 0.72
            v.co.y *= 0.85
    cranium.rotation_euler.z = yaw
    finish(cranium, name, BONE, smooth=True)
    # Jaw block
    jaw_offset = Vector((0.0, -scale * 0.52, -scale * 0.55))
    jaw_offset.rotate(Matrix.Rotation(yaw, 3, 'Z'))
    block(
        f'{name} jaw',
        Vector(loc) + jaw_offset,
        (scale * 0.92, scale * 0.82, scale * 0.58),
        BONE,
        bevel=scale * 0.14,
        rot=(0.0, 0.0, yaw),
    )


def rock():
    """Sculpted stratified basalt crag with obsidian outcrop and talus."""
    sculpted_crag('primary basalt ridge', (-0.06, 0.02, 0.54), (1.02, 0.78, 1.04), STONE, subdivisions=3, seed_offset=1)
    sculpted_crag('sheared slate crest', (0.48, 0.14, 0.34), (0.68, 0.55, 0.62), DARK, subdivisions=3, seed_offset=3, strata=7.0)
    sculpted_crag('buttress outcrop', (-0.56, -0.22, 0.24), (0.52, 0.44, 0.42), DARK, subdivisions=2, seed_offset=5)

    # Emergent hexagonal basalt columns wedged into the crag flank
    for idx, (bx, by, h, r, tx, ty) in enumerate([
        (0.28, -0.34, 0.88, 0.17, 0.10, -0.06),
        (0.46, -0.24, 0.62, 0.14, 0.12, -0.04),
        (-0.34, 0.36, 0.74, 0.16, -0.08, 0.07),
    ]):
        basalt_prism(f'basalt column {idx}', (bx, by, 0.0), h, r, (tx, ty), DARK if idx % 2 == 0 else STONE)

    # Half-buried relic masonry shard for dark-fantasy environmental storytelling
    shard = block('buried relic slab', (-0.18, -0.58, 0.16), (0.44, 0.18, 0.34), STONE, bevel=0.022, rot=(0.24, -0.18, 0.42))
    _ = shard

    # Scattered angular talus stones around base
    for i in range(8):
        a = i * math.tau / 8.0 + rng.uniform(-0.18, 0.18)
        dist = rng.uniform(0.76, 1.08)
        sculpted_crag(
            f'talus shard {i}',
            (dist * math.cos(a), dist * math.sin(a) * 0.84, 0.08),
            (rng.uniform(0.14, 0.24), rng.uniform(0.12, 0.20), rng.uniform(0.09, 0.17)),
            STONE if i % 3 else DARK,
            subdivisions=2,
            seed_offset=10 + i,
            strata=4.0,
        )


def tree():
    """Gnarled twisted deadwood with fluted bark, buttress roots, and iron soul lantern."""
    trunk_pts = [
        (0.00, 0.00, 0.00),
        (0.06, 0.02, 0.42),
        (0.16, 0.07, 0.96),
        (-0.08, 0.11, 1.56),
        (-0.14, 0.04, 2.18),
        (0.09, 0.06, 2.74),
        (0.28, 0.12, 3.22),
        (0.38, 0.16, 3.55),
    ]
    trunk_rad = [0.33, 0.25, 0.21, 0.185, 0.15, 0.11, 0.06, 0.015]
    spline_tube(
        'gnarled trunk',
        trunk_pts,
        trunk_rad,
        BARK,
        sides=14,
        samples_per_seg=3,
        flute_amp=0.14,
        flute_freq=6,
        twist=1.4,
    )

    # Flared gnarled buttress roots gripping the soil
    for i in range(6):
        a = i * math.tau / 6.0 + (0.22 if i % 2 else -0.14)
        reach = rng.uniform(0.68, 0.95)
        root_pts = [
            (0.07 * math.cos(a), 0.07 * math.sin(a), 0.42),
            (0.28 * math.cos(a + 0.08), 0.28 * math.sin(a + 0.08), 0.22),
            (0.56 * math.cos(a - 0.06), 0.56 * math.sin(a - 0.06), 0.09),
            (reach * math.cos(a + 0.05), reach * math.sin(a + 0.05), 0.01),
        ]
        spline_tube(
            f'buttress root {i}',
            root_pts,
            [0.145, 0.10, 0.055, 0.016],
            BARK,
            sides=9,
            samples_per_seg=2,
            flute_amp=0.08,
            flute_freq=4,
        )

    # Sinuous primary boughs + forked secondary & tertiary dead twigs
    lantern_hook = None
    bough_specs = [
        (0.95, 0.35, 1.18, 0.48, 0.105),
        (1.22, 2.55, 1.26, 0.62, 0.100),
        (1.52, 4.45, 1.14, 0.56, 0.095),
        (1.88, 1.38, 1.05, 0.72, 0.085),
        (2.16, 3.62, 1.12, 0.78, 0.078),
        (2.50, 5.45, 0.92, 0.68, 0.068),
        (2.78, 2.05, 0.78, 0.58, 0.054),
    ]
    for idx, (z, ang, reach, rise, r0) in enumerate(bough_specs):
        # Sample approximate trunk centerline
        cx = 0.08 * math.sin(z * 1.4)
        cy = 0.07
        p0 = Vector((cx, cy, z))
        p1 = Vector((cx + reach * 0.44 * math.cos(ang), cy + reach * 0.44 * math.sin(ang), z + rise * 0.22))
        p2 = Vector((cx + reach * 0.78 * math.cos(ang + 0.18), cy + reach * 0.78 * math.sin(ang + 0.18), z + rise * 0.68))
        p3 = Vector((cx + reach * math.cos(ang + 0.10), cy + reach * math.sin(ang + 0.10), z + rise))
        spline_tube(
            f'primary bough {idx}',
            [p0, p1, p2, p3],
            [r0, r0 * 0.72, r0 * 0.42, r0 * 0.14],
            BARK,
            sides=9,
            samples_per_seg=2,
            flute_amp=0.07,
            flute_freq=4,
            twist=0.6,
        )
        if idx == 0:
            lantern_hook = p1.lerp(p2, 0.55)

        # Forked secondary twig off elbow
        fork_ang = ang - 0.58 if idx % 2 == 0 else ang + 0.55
        f1 = p1 + Vector((0.30 * math.cos(fork_ang), 0.30 * math.sin(fork_ang), 0.24))
        f2 = f1 + Vector((0.26 * math.cos(fork_ang + 0.22), 0.26 * math.sin(fork_ang + 0.22), 0.34))
        spline_tube(
            f'secondary twig {idx}',
            [p1, f1, f2],
            [r0 * 0.52, r0 * 0.26, 0.008],
            BARK,
            sides=7,
            samples_per_seg=2,
        )

        # Crown sub-twig off p2
        t_ang = ang + (0.45 if idx % 2 == 0 else -0.45)
        t1 = p2 + Vector((0.24 * math.cos(t_ang), 0.24 * math.sin(t_ang), 0.28))
        spline_tube(
            f'crown twig {idx}',
            [p2, t1],
            [r0 * 0.32, 0.006],
            BARK,
            sides=6,
            samples_per_seg=1,
        )

    # Weathered iron soul-lantern hanging from lower primary bough
    if lantern_hook is not None:
        hx, hy, hz = lantern_hook.x, lantern_hook.y, lantern_hook.z
        cylinder_seg('lantern chain', (hx, hy, hz), (hx, hy, hz - 0.26), 0.012, 0.012, IRON, sides=6)
        # Lantern cap & base
        cylinder_seg('lantern roof', (hx, hy, hz - 0.32), (hx, hy, hz - 0.22), 0.11, 0.025, IRON, sides=6)
        cylinder_seg('lantern base', (hx, hy, hz - 0.52), (hx, hy, hz - 0.47), 0.075, 0.095, IRON, sides=6)
        # 4 iron cage bars
        for s in range(4):
            a = s * math.tau / 4.0 + 0.4
            ox, oy = math.cos(a) * 0.075, math.sin(a) * 0.075
            cylinder_seg(f'lantern rib {s}', (hx + ox, hy + oy, hz - 0.48), (hx + ox, hy + oy, hz - 0.32), 0.01, 0.01, IRON, sides=6)
        # Glowing bound ember core
        bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=0.052, location=(hx, hy, hz - 0.40))
        core = bpy.context.object
        core.scale = (0.85, 0.85, 1.25)
        finish(core, 'lantern ember core', GLOW, smooth=True)

    # Small obsidian stones tucked between the roots
    for i in range(4):
        a = i * 1.65 + 0.4
        sculpted_crag(
            f'root stone {i}',
            (0.42 * math.cos(a), 0.42 * math.sin(a), 0.07),
            (0.18, 0.15, 0.11),
            DARK,
            subdivisions=2,
            seed_offset=30 + i,
        )


def crystals():
    """Dramatic multi-tier faceted arcane crystal cluster erupting from obsidian bedrock."""
    # Central towering spire + radiant inner core
    crystal_spire('central crystal spire', (0.0, 0.0, 0.06), (0.14, 0.05, 1.98), 0.22, 0.20, CRYSTAL, sides=6)
    crystal_spire('inner glowing core', (0.04, -0.02, 0.18), (0.12, 0.03, 1.72), 0.12, 0.11, CRYSTAL_CORE, sides=6)

    # Secondary angled crystal ring (7 spires)
    ring_specs = [
        (0.30, 0.20, 1.38, 0.16, 0.14, 0.34),
        (0.28, 1.12, 1.16, 0.14, 0.12, 0.38),
        (0.34, 2.08, 1.48, 0.17, 0.15, 0.30),
        (0.32, 3.04, 1.08, 0.14, 0.11, 0.42),
        (0.29, 3.98, 1.34, 0.15, 0.13, 0.35),
        (0.35, 4.92, 0.96, 0.13, 0.10, 0.46),
        (0.31, 5.72, 1.22, 0.14, 0.12, 0.36),
    ]
    for i, (rad, ang, h, r0, r1, lean) in enumerate(ring_specs):
        bx, by = math.cos(ang) * rad, math.sin(ang) * rad
        tx, ty = bx + math.cos(ang) * lean, by + math.sin(ang) * lean
        mat = CRYSTAL_CORE if i in (0, 2, 4) else CRYSTAL
        crystal_spire(f'secondary crystal {i}', (bx, by, 0.05), (tx, ty, h), r0, r1, mat, sides=6)

    # Low outward crystal splinters around the base
    for i in range(6):
        ang = i * math.tau / 6.0 + 0.52
        bx, by = math.cos(ang) * 0.48, math.sin(ang) * 0.44
        tx, ty = bx + math.cos(ang) * 0.34, by + math.sin(ang) * 0.34
        crystal_spire(f'basal crystal shard {i}', (bx, by, 0.04), (tx, ty, rng.uniform(0.46, 0.76)), 0.09, 0.07, CRYSTAL, sides=6)

    # Three tiny hovering arcane crystal splinters around the upper spire
    for i, (ang, dist, z, h) in enumerate([(0.6, 0.46, 1.56, 0.28), (2.7, 0.42, 1.72, 0.24), (4.6, 0.48, 1.38, 0.22)]):
        px, py = math.cos(ang) * dist, math.sin(ang) * dist
        crystal_spire(f'floating crystal shard {i}', (px, py, z - h * 0.5), (px + 0.02, py, z + h * 0.5), 0.042, 0.036, CRYSTAL_CORE, sides=6)

    # Forged relic gold rune collar gripping the lower base
    for i in range(6):
        ang = i * math.tau / 6.0
        bx, by = math.cos(ang) * 0.32, math.sin(ang) * 0.32
        block(
            f'rune collar clasp {i}',
            (bx, by, 0.22),
            (0.14, 0.07, 0.16),
            GOLD,
            bevel=0.012,
            rot=(0.18, 0.0, ang + math.pi * 0.5),
        )

    # Fractured obsidian bedrock base
    for i in range(7):
        ang = i * math.tau / 7.0 + 0.18
        dist = 0.38 if i % 2 == 0 else 0.52
        sculpted_crag(
            f'crystal bedrock {i}',
            (math.cos(ang) * dist, math.sin(ang) * dist * 0.92, 0.10),
            (0.28, 0.24, 0.20 if i % 2 == 0 else 0.15),
            DARK,
            subdivisions=2,
            seed_offset=50 + i,
            strata=6.0,
        )


def ruin():
    """Broken Gothic cathedral arch with fluted triple-shaft piers, ribbed archivolt, and iron spikes."""
    # Stepped plinths + fluted gothic triple-shaft piers
    for side in (-1, 1):
        px = side * 0.42
        block(f'pier sub-plinth {side}', (px, 0.0, 0.045), (0.40, 0.54, 0.09), DARK, bevel=0.02)
        block(f'pier plinth {side}', (px, 0.0, 0.12), (0.33, 0.45, 0.08), STONE, bevel=0.018)

        # Core pier courses (asymmetric ruin break height: left taller than right)
        rows = 4 if side == -1 else 3
        for row in range(rows):
            jitter_x = (0.008 if row % 2 else -0.008) * side
            block(
                f'pier course {side}_{row}',
                (px + jitter_x, 0.0, 0.24 + row * 0.165),
                (0.24, 0.36, 0.155),
                STONE if (row + (1 if side > 0 else 0)) % 3 else DARK,
                bevel=0.016,
            )

        # Clustered Gothic engaged colonnettes (front, back, inner face)
        col_top = 0.20 + rows * 0.165
        for oy in (-0.15, 0.15):
            cylinder_seg(f'outer colonnette {side}_{oy}', (px, oy, 0.14), (px, oy, col_top), 0.052, 0.048, STONE, sides=10)
        cylinder_seg(f'inner colonnette {side}', (px - side * 0.11, 0.0, 0.14), (px - side * 0.11, 0.0, col_top), 0.048, 0.044, DARK, sides=10)

        # Carved gothic capital moulding
        block(f'pier capital {side}', (px, 0.0, col_top + 0.035), (0.30, 0.42, 0.075), STONE, bevel=0.018)
        block(f'capital trim {side}', (px, 0.0, col_top + 0.075), (0.33, 0.46, 0.035), DARK, bevel=0.01)

    # Pointed Gothic ribbed arch voussoirs (left spring rises past apex; right wing snapped)
    arch_center_z = 0.84
    arch_rx = 0.42
    arch_rz = 0.56
    for i in range(11):
        # Skip broken right-upper span (i = 2, 3) to create dramatic silhouette
        if i in (2, 3):
            continue
        t = i / 10.0
        ang = math.pi * t
        # Pointed gothic profile via slight pinched crest
        px = arch_rx * math.cos(ang)
        pz = arch_center_z + arch_rz * (math.sin(ang) ** 0.86)
        pitch = ang - math.pi * 0.5
        mat = STONE if i % 2 == 0 else DARK
        block(f'arch voussoir {i}', (px, 0.0, pz), (0.145, 0.34, 0.145), mat, bevel=0.014, rot=(0.0, pitch, 0.0))
        # Inner gothic moulding rib
        block(f'arch inner rib {i}', (px * 0.84, 0.0, arch_center_z + (pz - arch_center_z) * 0.84), (0.075, 0.15, 0.095), DARK, bevel=0.01, rot=(0.0, pitch, 0.0))

    # Gilded iron relic bracket & faint glowing ember rune carved into the taller pier
    block('pier relic iron clamp', (-0.42, -0.19, 0.52), (0.28, 0.045, 0.055), IRON, bevel=0.008)
    block('pier carved rune', (-0.42, -0.195, 0.42), (0.08, 0.02, 0.14), GLOW, bevel=0.004)

    # Wrought-iron gothic fence spears jutting from the lower broken side
    for k in range(4):
        fx = 0.62 + k * 0.11
        h = 0.48 - k * 0.07
        lean = 0.06 * k
        cylinder_seg(f'iron fence bar {k}', (fx, 0.0, 0.02), (fx + lean, 0.0, h), 0.015, 0.012, IRON, sides=6)
        cylinder_seg(f'iron fence spear {k}', (fx + lean, 0.0, h), (fx + lean * 1.15, 0.0, h + 0.10), 0.024, 0.002, IRON, sides=4)
    block('iron fence rail', (0.78, 0.0, 0.24), (0.42, 0.032, 0.035), IRON, bevel=0.006, rot=(0.0, 0.14, 0.0))

    # Fallen masonry blocks, sculpted rubble, and a weathered skull at the base
    block('fallen keystone', (0.14, -0.26, 0.07), (0.22, 0.18, 0.14), STONE, bevel=0.016, rot=(0.2, 0.35, 0.6))
    for i in range(6):
        sculpted_crag(
            f'ruin rubble {i}',
            (rng.uniform(-0.72, 0.76), rng.uniform(-0.42, 0.38), 0.05),
            (rng.uniform(0.11, 0.20), rng.uniform(0.10, 0.18), rng.uniform(0.06, 0.11)),
            DARK if i % 2 == 0 else STONE,
            subdivisions=2,
            seed_offset=70 + i,
        )
    weathered_skull('ruin skull', (-0.22, -0.32, 0.095), scale=0.085, yaw=0.65)


def gate():
    """Monumental Gothic sanctum obelisk pillar with carved masonry, iron cage brazier, and glowing sigil."""
    # Multi-tiered stepped obsidian dais
    for z, w, d, h, bev, mat in [
        (0.13, 1.38, 1.06, 0.26, 0.045, DARK),
        (0.32, 1.12, 0.86, 0.16, 0.035, STONE),
        (0.48, 0.90, 0.68, 0.16, 0.030, DARK),
        (2.16, 0.66, 0.52, 3.24, 0.042, DARK),
        (3.86, 0.86, 0.70, 0.18, 0.035, STONE),
        (4.02, 0.72, 0.58, 0.14, 0.025, DARK),
    ]:
        block(f'sanctum tier {z:.2f}', (0.0, 0.0, z), (w, d, h), mat, bevel=bev)

    # Corner iron bollards on dais
    for sx in (-1, 1):
        for sy in (-1, 1):
            cylinder_seg(
                f'dais bollard {sx}_{sy}',
                (sx * 0.54, sy * 0.40, 0.24),
                (sx * 0.54, sy * 0.40, 0.68),
                0.065,
                0.042,
                IRON,
                sides=8,
            )
            cylinder_seg(
                f'dais spike {sx}_{sy}',
                (sx * 0.54, sy * 0.40, 0.68),
                (sx * 0.54, sy * 0.40, 0.84),
                0.045,
                0.005,
                GOLD,
                sides=6,
            )

    # Fluted gothic iron buttresses + riveted relic bronze bands
    for side in (-1, 1):
        block(f'iron buttress {side}', (side * 0.36, 0.0, 2.15), (0.12, 0.58, 3.22), IRON, bevel=0.02)
        cylinder_seg(f'flanking colonnette {side}', (side * 0.43, -0.22, 0.55), (side * 0.43, -0.22, 3.78), 0.045, 0.04, STONE, sides=10)
        for z in (0.85, 1.55, 2.25, 2.95, 3.55):
            block(f'gold band {side}_{z:.2f}', (side * 0.36, -0.31, z), (0.17, 0.075, 0.14), GOLD, bevel=0.015)
            block(f'gold band back {side}_{z:.2f}', (side * 0.36, 0.31, z), (0.17, 0.075, 0.14), GOLD, bevel=0.015)

    # Recessed gothic seal tablets (front & back) + incised ember runes
    for face in (-1, 1):
        block(f'seal tablet {face}', (0.0, face * 0.268, 2.16), (0.44, 0.032, 2.84), STONE, bevel=0.012)
        block(f'tablet frame top {face}', (0.0, face * 0.282, 3.58), (0.50, 0.045, 0.08), GOLD, bevel=0.01)
        for i in range(7):
            z = 1.05 + i * 0.34
            rune = block(f'ember rune {face}_{i}', (0.0, face * 0.292, z), (0.24, 0.022, 0.028), GLOW, bevel=0.005)
            rune.rotation_euler.y = (-1 if i % 2 else 1) * 0.46
        # Vertical glowing spine line linking runes
        block(f'ember spine {face}', (0.0, face * 0.288, 2.08), (0.028, 0.018, 2.20), GLOW, bevel=0.004)

    # Gothic twin-horn iron crown & cage cradling a faceted bound cinder crystal
    for side in (-1, 1):
        horn_pts = [
            (side * 0.30, 0.0, 4.02),
            (side * 0.36, 0.0, 4.38),
            (side * 0.22, 0.0, 4.74),
            (side * 0.09, 0.0, 5.02),
        ]
        spline_tube(f'gothic horn finial {side}', horn_pts, [0.11, 0.085, 0.055, 0.014], IRON, sides=8, samples_per_seg=2)
        # Inner gold accent barb
        cylinder_seg(
            f'crown barb {side}',
            (side * 0.22, 0.0, 4.36),
            (side * 0.10, 0.0, 4.52),
            0.032,
            0.008,
            GOLD,
            sides=6,
        )

    # Suspended faceted bound ember crystal
    crystal_spire('bound cinder top', (0.0, 0.0, 4.34), (0.0, 0.0, 4.76), 0.13, 0.09, GLOW, sides=6)
    crystal_spire('bound cinder bot', (0.0, 0.0, 4.34), (0.0, 0.0, 4.08), 0.13, 0.07, GLOW, sides=6)


BUILDERS = {
    'scenery_rock': rock,
    'scenery_dead_tree': tree,
    'scenery_crystal': crystals,
    'scenery_ruin': ruin,
    'sanctum_gate': gate,
}

PREVIEWS = []
for name, builder in BUILDERS.items():
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    builder()
    # One mesh primitive per material; baked geometry keeps streamed scenes inexpensive.
    for mat in list(bpy.data.materials):
        objs = [o for o in bpy.context.scene.objects if o.type == 'MESH' and o.active_material == mat]
        if not objs:
            continue
        bpy.ops.object.select_all(action='DESELECT')
        for o in objs:
            o.select_set(True)
        bpy.context.view_layer.objects.active = objs[0]
        bpy.ops.object.join()
        o = bpy.context.object
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.mesh.select_all(action='SELECT')
        bpy.ops.uv.smart_project(island_margin=0.02)
        bpy.ops.object.mode_set(mode='OBJECT')

    total_tris = sum(
        sum(max(0, len(p.vertices) - 2) for p in o.data.polygons)
        for o in bpy.context.scene.objects
        if o.type == 'MESH'
    )
    print(f'[env] {name}.glb -> {total_tris} tris')
    bpy.ops.export_scene.gltf(filepath=str(OUT / f'{name}.glb'), export_format='GLB', export_yup=True)
    for o in list(bpy.context.scene.objects):
        clone = o.copy()
        clone.data = o.data.copy()
        PREVIEWS.append((name, clone))

bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)
for name, obj in PREVIEWS:
    bpy.context.collection.objects.link(obj)
    obj.location.x += list(BUILDERS).index(name) * 3.8 - 7.6
bpy.ops.mesh.primitive_plane_add(size=50)
bpy.context.object.data.materials.append(DARK)
bpy.ops.object.camera_add(location=(10, -19, 12))
cam = bpy.context.object
cam.rotation_euler = (Vector((0, 0, 1.35)) - cam.location).to_track_quat('-Z', 'Y').to_euler()
cam.data.type = 'ORTHO'
cam.data.ortho_scale = 22
bpy.context.scene.camera = cam
for loc, power, size in [((0, -6, 10), 2200, 12), ((6, 5, 8), 1800, 10)]:
    bpy.ops.object.light_add(type='AREA', location=loc)
    o = bpy.context.object
    o.data.energy = power
    o.data.shape = 'DISK'
    o.data.size = size
    o.rotation_euler = (-o.location).to_track_quat('-Z', 'Y').to_euler()
s = bpy.context.scene
s.render.engine = 'CYCLES'
s.cycles.samples = 28
s.render.resolution_x = 1600
s.render.resolution_y = 800
s.render.resolution_percentage = 100
s.world.color = (0.14, 0.14, 0.15)
s.render.filepath = str(PREVIEW / 'environment.png')
bpy.ops.render.render(write_still=True)
