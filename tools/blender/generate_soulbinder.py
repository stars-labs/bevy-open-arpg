"""High-detail dark-fantasy Soulbinder, Crowned Spectral Wraith (soul_spirit),
and Ossuary Bone Guard (bone_sentry), plus soul_staff and soul_blade.

Same anchored part contract as generate_hero.py / generate_hero_classes.py:
  blender -b --factory-startup --python tools/blender/generate_soulbinder.py
  blender -b --factory-startup --python tools/blender/postprocess_glb_materials.py -- --only ...
Pass `-- --preview-only` after postprocessing to render comparison sheets.
"""
import importlib.util
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('class_art', HERE / 'generate_hero_classes.py')
a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a)
h = a.hero
TAU = math.tau

# ---------------------------------------------------------------- materials
VELVET = h.material('soulbinder petrol velvet', (.020, .095, .088), 0., .88)
PARCHMENT = h.material('ritual parchment cloth', (.48, .44, .35), 0., .84)
SILVER = h.material('funerary tarnished silver', (.38, .44, .42), .82, .32)
GOLD = h.GOLD
DARK = h.DARK
LEATHER = h.LEATHER
BONE = a.BONE
WOOD = a.WOOD
IRON = h.material('ossuary dark iron', (.12, .14, .15), .84, .36)

SPIRIT_ARMOR = h.material('spectral patinated armor', (.048, .22, .21), .80, .28)
SPIRIT_SHROUD = h.material('spectral wraith shroud', (.015, .072, .078), 0., .86)
GLOW = h.material('bound soul jade', (.16, .92, .74), 0., .26, 2.2)
CORE_GLOW = h.material('spectral core glow', (.46, 1.0, .90), 0., .20, 3.2)

OUTPUTS = [
    'hero_soulbinder.glb', 'hero_soulbinder_arm.glb', 'hero_soulbinder_leg.glb', 'hero_soulbinder_mantle.glb',
    'soul_spirit.glb', 'soul_spirit_arm.glb', 'soul_spirit_leg.glb', 'soul_spirit_mantle.glb',
    'bone_sentry.glb', 'bone_sentry_arm.glb', 'bone_sentry_leg.glb', 'bone_sentry_mantle.glb',
    'soul_staff.glb', 'soul_blade.glb',
]


# ---------------------------------------------------------------- sculpting helpers

def smooth_loft(name, rings, mat, sides=40, bevel=.008):
    return h.loft(name, rings, mat, sides=sides, bevel=bevel)


def sculpted_loft(name, ring_specs, mat, sides=44, crease=52.):
    """Loft with radial pleating/fluting: (z, rx, ry, cy, lobes, amp, wave)."""
    rings = [a.zring(z, rx, ry, cy, n=sides, lobes=lobes, amp=amp, wave=wave)
             for z, rx, ry, cy, lobes, amp, wave in ring_specs]
    return a.skin(name, rings, mat, closed=False, crease=crease)


def gothic_hood_ring(y, zc, rx, rz, n=48, brow=0., peak=0.055, drape=0.025, folds=0.):
    """Pointed gothic cowl arch facing Blender -Y: pointed peak on top, narrow draped sides,
    soft hanging cloth folds instead of a round helmet circle."""
    pts = []
    for i in range(n):
        ang = TAU * i / n
        ca, sa = math.cos(ang), math.sin(ang)
        # Sharpen top into a gothic pointed arch (when sa > 0)
        top_peak = peak * (max(0., sa) ** 3.2)
        # Pull lower sides slightly inward & downward like draped fabric
        side_taper = 1.0 - 0.14 * (max(0., sa) ** 1.6) + drape * max(0., -sa)
        fold = folds * math.cos(ang * 7.0) * (0.35 + 0.65 * abs(ca))
        x = (rx * side_taper + fold) * ca
        z = zc + rz * sa + top_peak
        yy = y - brow * (max(0., sa) ** 1.8) + fold * 0.45
        pts.append((x, yy, z))
    return pts


def sculpted_mask(name, mat, z_center=1.745, y_front=-.192):
    """High-resolution sculpted Venetian / reliquary pointed ivory funeral mask.
    Wide high temples & sculpted cheekbones tapering sharply to a gothic pointed chin,
    prominent nose keel, deeply carved dark eye hollows — sits cleanly forward of shadow."""
    rows, cols = 20, 26
    grid = []
    for r in range(rows):
        v = r / (rows - 1)  # 0 = brow/forehead top, 1 = pointed chin tip
        z = z_center + .168 - v * .325
        # Sculpted face width: brow -> wide cheekbones -> narrow tapered gothic chin
        if v < .36:
            half_w = .078 + .046 * math.sin((v / .36) * math.pi * .5)
        else:
            t = (v - .36) / .64
            half_w = .124 * ((1. - t) ** .72) + .016 * (1. - t)
        row = []
        for c in range(cols):
            u = (c / (cols - 1)) * 2. - 1.
            au = abs(u)
            x = u * half_w
            # Gentle curved wrap back around temples and jaw
            wrap = (au ** 1.75) * .082
            # Sculpted facial anatomy
            brow = .018 * math.exp(-((v - .22) / .085) ** 2) * (1. - .35 * au)
            nose = .030 * math.exp(-(u / .18) ** 2) * math.exp(-((v - .48) / .24) ** 2)
            cheek = .020 * math.exp(-((au - .56) / .22) ** 2) * math.exp(-((v - .47) / .15) ** 2)
            eye_hollow = .028 * math.exp(-((au - .40) / .16) ** 2) * math.exp(-((v - .35) / .085) ** 2)
            gaunt = .014 * math.exp(-((au - .52) / .20) ** 2) * math.exp(-((v - .70) / .14) ** 2)
            chin = .018 * math.exp(-(u / .34) ** 2) * math.exp(-((v - .93) / .12) ** 2)
            y = y_front + wrap - brow - nose - cheek - chin + eye_hollow + gaunt
            row.append((x, y, z))
        grid.append(row)
    return a.sheet(name, grid, mat, thick=.018)


def sculpted_pauldron_shell(name, side, cx, cy, cz, rx, ry, rz, mat, tilt=0.22, flair=0.04):
    """Curved gothic layered armor pauldron shell wrapping smoothly around the deltoid
    front-to-back and arching down the shoulder (neither a balloon sphere nor a flat roof)."""
    rows, cols = 12, 20
    grid = []
    for r in range(rows):
        v = r / (rows - 1)  # 0 = top shoulder apex, 1 = flared outer rim
        row = []
        for c in range(cols):
            u = (c / (cols - 1)) * 2. - 1.  # -1 front .. +1 back
            ang_f = u * math.pi * .54
            ridge = .016 * (1. - abs(u)) * (1. - .35 * v)
            # Smooth front-to-back arch wrapping over the deltoid
            wrap_drop = (.055 + .035 * v) * (u * u)
            lx = side * (v * rx + flair * (v ** 2.2) - .028 * (u * u) * v)
            ly = math.sin(ang_f) * ry * (.52 + .50 * v)
            lz = (1. - v ** 1.55) * rz - v * rx * tilt - wrap_drop + ridge * math.cos(ang_f)
            row.append((cx + lx, cy + ly, cz + lz))
        grid.append(row)
    return a.sheet(name, grid, mat, thick=.020)


def ornate_medallion(name, loc, radius, mat, inset_mat=None):
    """Flush beveled gothic circular clasp medallion facing forward (-Y)."""
    cx, cy, cz = loc
    bpy.ops.mesh.primitive_cylinder_add(vertices=24, radius=radius, depth=.018, location=(cx, cy, cz))
    obj = bpy.context.object
    obj.rotation_euler = (math.pi / 2, 0, 0)
    h.finish(obj, name, mat, bevel=.004)
    # Beveled inner ring lying in the XZ plane (parallel to medallion face)
    rim_pts = [(cx + math.cos(TAU * i / 24) * radius * .86, cy - .010,
                cz + math.sin(TAU * i / 24) * radius * .86) for i in range(24)]
    a.tube(name + ' rim', rim_pts, [.006] * 24, mat, sides=8, side=(0, 1, 0), closed=True)
    if inset_mat:
        a.ellipsoid(name + ' gem', (cx, cy - .012, cz), (radius * .42, .012, radius * .42), inset_mat, 16, 10)
    return obj


def miniature_skull(name, loc, scale, mat):
    cx, cy, cz = loc
    a.ellipsoid(name + ' cranium', (cx, cy, cz + .018 * scale), (.046 * scale, .042 * scale, .046 * scale), mat, 18, 12)
    a.ellipsoid(name + ' jaw', (cx, cy - .012 * scale, cz - .022 * scale), (.028 * scale, .028 * scale, .026 * scale), mat, 14, 10)
    for s in (-1, 1):
        a.ellipsoid(name + ' socket', (cx + s * .017 * scale, cy - .036 * scale, cz + .012 * scale),
                    (.012 * scale, .010 * scale, .012 * scale), DARK, 10, 8)
    a.shard(name + ' nose', (cx, cy - .042 * scale, cz + .002 * scale),
            (cx, cy - .040 * scale, cz - .014 * scale), .007 * scale, DARK, 4)


def ribbed_cloak(name, mat, z_top=1.49, z_bot=.28, y_top=.20, y_bot=.47, w_top=.27, w_bot=.48,
                 cols=20, rows=14, folds=6, fold_amp=.034, tatter=0., split=0.):
    grid = []
    for r in range(rows):
        t = r / (rows - 1)
        row = []
        for c in range(cols + 1):
            u = (c / cols) * 2. - 1.
            au = abs(u)
            width = w_top + (w_bot - w_top) * (t ** .85)
            x = u * width
            wave = math.cos(u * folds * math.pi) * fold_amp * (0.25 + 0.75 * t)
            y = y_top + (y_bot - y_top) * (t ** 1.15) + wave + .042 * (1. - au * au) * (1. - t * .5)
            scallop = math.cos(u * (folds + 1) * math.pi) * (.032 + tatter * .078) * (t ** 2)
            center_cut = split * math.exp(-(u / .22) ** 2) * (t ** 1.6)
            z = z_top - (z_top - z_bot) * t + scallop + center_cut
            row.append((x, y, z))
        grid.append(row)
    return a.sheet(name, grid, mat, thick=.018)


def fingered_hand(name, z_wrist, mat, accent_mat, claw=False):
    a.ellipsoid(name + ' palm', (0, -.005, z_wrist - .032), (.046, .035, .042), mat, 16, 12)
    a.hoop(name + ' wrist band', z_wrist, .052, .046, .008, accent_mat)
    for i in range(4):
        fx = -.031 + i * .021
        flen = (.060 if i in (1, 2) else .050) + (.020 if claw else 0.)
        pts = [
            (fx, -.016, z_wrist - .060),
            (fx * 1.04, -.030, z_wrist - .060 - flen * .55),
            (fx * 1.02, -.022, z_wrist - .060 - flen),
        ]
        a.limb(f'{name} finger {i}', pts, [.011, .009, .003 if claw else .006],
               accent_mat if claw else mat, sides=8, steps=3)
    a.limb(name + ' thumb', [(.038, -.012, z_wrist - .034), (.052, -.032, z_wrist - .064),
           (.040, -.040, z_wrist - .088)], [.013, .010, .0055],
           accent_mat if claw else mat, sides=8, steps=3)


# ========================================================================
# 1. HERO SOULBINDER — Hooded Funerary Necromancer-Priest
# ========================================================================

def soulbinder_body():
    a.reset()
    # Tailored pleated dark petrol-velvet surcoat & torso under-robe
    sculpted_loft('velvet funerary surcoat', [
        (.65, .258, .198, .005, 8, .030, .008),
        (.77, .232, .174, .002, 8, .022, .005),
        (.93, .208, .154, .000, 6, .012, .002),
        (1.14, .266, .185, -.005, 0, .0, .0),
        (1.36, .310, .200, -.004, 0, .0, .0),
        (1.49, .232, .160, .000, 0, .0, .0),
    ], VELVET, sides=44)

    # Shadowed inner chest cuirass behind the silver ribcage
    smooth_loft('inner funerary cuirass', [
        (.94, .204, .152, -.008),
        (1.15, .258, .184, -.014),
        (1.36, .298, .194, -.010),
        (1.48, .210, .150, -.004),
    ], DARK, sides=36, bevel=.006)

    # Sculpted gothic tarnished-silver sternum spine
    a.plate('sternum breastplate spine', [
        (0, -.212, .97), (.054, -.203, 1.06), (.070, -.211, 1.24),
        (.092, -.205, 1.38), (0, -.215, 1.47), (-.092, -.205, 1.38),
        (-.070, -.211, 1.24), (-.054, -.203, 1.06),
    ], SILVER, .026)
    a.limb('sternum raised keel', [
        (0, -.224, .98), (0, -.238, 1.18), (0, -.234, 1.36), (0, -.220, 1.46)
    ], [(.012, .010), (.021, .017), (.017, .014), (.010, .009)], SILVER, sides=10, steps=4)
    a.shard('sternum gold finial', (0, -.244, 1.41), (0, -.244, 1.23), .022, GOLD, 6)
    a.shard('sternum jade core', (0, -.252, 1.34), (0, -.252, 1.26), .012, GLOW, 6)

    # 6 pairs of anatomical sweeping tarnished-silver ribs
    for s in (-1, 1):
        for i in range(6):
            frac = i / 5.
            z = 1.00 + i * .072
            reach_x = .232 + .064 * math.sin(frac * math.pi * .78)
            front_y = -.210 - .014 * math.sin(frac * math.pi)
            mid_y = -.182 - .010 * math.sin(frac * math.pi)
            back_y = -.045 + .025 * frac
            a.limb(f'anatomical silver rib {s}_{i}', [
                (s * .038, front_y, z),
                (s * (reach_x * .58), mid_y, z + .026),
                (s * reach_x, back_y, z + .046),
                (s * (reach_x * .86), .112, z + .048),
            ], [(.016, .013), (.018, .014), (.014, .011), (.006, .005)], SILVER, sides=10, steps=4)
        a.limb(f'clavicle silver collar {s}', [
            (s * .04, -.205, 1.44), (s * .17, -.162, 1.47), (s * .27, -.05, 1.46)
        ], [.017, .015, .011], SILVER, sides=10, steps=4)

    # High gothic gorget & antique gold collar rim
    smooth_loft('funerary gorget', [
        (1.43, .190, .155, -.005),
        (1.52, .175, .148, -.005),
        (1.58, .165, .144, -.004),
    ], SILVER, sides=36, bevel=.006)
    a.hoop('gorget gold rim', 1.565, .170, .148, .008, GOLD, cy=-.004)

    # Layered ritual leather belt, gold buckle, hip skull & outer hanging censer
    smooth_loft('ritual waist belt', [(.885, .228, .170, 0), (.965, .232, .174, 0)], LEATHER, sides=40, bevel=.005)
    a.hoop('belt upper gold trim', .962, .236, .178, .007, GOLD)
    a.hoop('belt lower gold trim', .890, .232, .174, .007, GOLD)
    ornate_medallion('belt central clasp', (0, -.186, .925), .046, GOLD, GLOW)
    miniature_skull('hip reliquary skull', (.142, -.175, .915), .95, BONE)
    # Ornate censer hanging cleanly outside the left hip panel
    a.limb('hip censer chain', [(-.205, -.135, .90), (-.295, -.110, .74), (-.320, -.095, .60)],
           [.006, .006, .006], GOLD, sides=8, steps=4)
    a.ellipsoid('hip censer body', (-.320, -.095, .545), (.042, .042, .052), GOLD, 18, 12)
    a.hoop('hip censer band', .545, .046, .046, .007, SILVER, cx=-.320, cy=-.095)
    a.shard('hip censer finial', (-.320, -.095, .44), (-.320, -.095, .50), .018, GOLD, 6)
    a.ellipsoid('hip censer ember', (-.320, -.095, .545), (.032, .032, .018), GLOW, 12, 8)

    # Twin long embroidered parchment front ritual stoles + dark velvet side hip panels
    for s in (-1, 1):
        a.plate(f'velvet hip panel {s}', [
            (s * .13, -.172, .90), (s * .250, -.102, .89),
            (s * .288, -.122, .48), (s * .210, -.180, .40), (s * .142, -.196, .52),
        ], VELVET, .020)
        stole_grid = []
        for r in range(10):
            t = r / 9.
            z = .90 - t * .54
            y = -.186 - t * .046 + .010 * math.sin(t * math.pi)
            row = []
            for c in range(5):
                u = c / 4.
                x = s * (.038 + u * (.098 + t * .020))
                row.append((x, y - .006 * math.sin(u * math.pi), z))
            stole_grid.append(row)
        a.sheet(f'ritual parchment stole {s}', stole_grid, PARCHMENT, thick=.014)
        for edge_x in (.038, .142):
            a.limb(f'stole gold trim {s}_{edge_x:.2f}', [
                (s * edge_x, -.193, .89),
                (s * (edge_x + .008), -.220, .62),
                (s * (edge_x + .015), -.239, .37),
            ], [.005, .005, .004], GOLD, sides=6, steps=3)

    # SHARP GOTHIC POINTED DRAPED HOOD (sleeker, narrower, pointed arch — NOT a round circle!)
    hood_rings = [
        gothic_hood_ring(-.085, 1.745, .142, .178, n=48, brow=.00, peak=.030, drape=.015, folds=.004),
        gothic_hood_ring(-.185, 1.748, .162, .198, n=48, brow=.035, peak=.050, drape=.020, folds=.007),
        gothic_hood_ring(-.238, 1.752, .178, .218, n=48, brow=.075, peak=.072, drape=.028, folds=.009),
        gothic_hood_ring(-.170, 1.760, .212, .245, n=48, brow=.040, peak=.062, drape=.022, folds=.008),
        gothic_hood_ring(-.045, 1.772, .232, .258, n=48, brow=.010, peak=.045, drape=.015, folds=.006),
        gothic_hood_ring(.095, 1.778, .216, .240, n=48, brow=.00, peak=.035, drape=.010, folds=.005),
        gothic_hood_ring(.215, 1.768, .168, .185, n=48, brow=.00, peak=.025, drape=.006, folds=.003),
        gothic_hood_ring(.315, 1.742, .108, .118, n=48, brow=.00, peak=.015, drape=.002, folds=.002),
        gothic_hood_ring(.395, 1.700, .048, .052, n=48, brow=.00, peak=.006, drape=.000, folds=.000),
        gothic_hood_ring(.445, 1.665, .012, .014, n=48, brow=.00, peak=.000, drape=.000, folds=.000),
    ]
    a.skin('deep draped funerary hood', hood_rings, VELVET, closed=False, crease=58.)
    # Draped hood cowl folds cascading around neck/clavicles
    sculpted_loft('draped hood cowl collar', [
        (1.42, .325, .235, .010, 12, .046, .014),
        (1.50, .270, .205, .005, 12, .034, .010),
        (1.58, .210, .175, -.002, 10, .022, .006),
    ], VELVET, sides=48)
    # Gold embroidered gothic arch trim along hood opening
    a.tube('hood gold edge trim',
           gothic_hood_ring(-.236, 1.752, .179, .219, n=48, brow=.074, peak=.072, drape=.028, folds=.009),
           [.007] * 48, GOLD, sides=8, side=(0, 1, 0), closed=True)
    # Deep shadow backing BEHIND the ivory mask (never clips through the mask cheeks!)
    a.ellipsoid('hood inner shadow', (0, -.055, 1.745), (.138, .062, .172), DARK, 24, 16)

    # SCULPTED POINTED IVORY FUNERAL MASK — completely clear and crisp inside the hood!
    sculpted_mask('sculpted ivory funeral mask', BONE, z_center=1.745, y_front=-.192)
    # Deep dark recessed eye hollows + subtle eerie jade eye slits right on the mask sockets
    for s in (-1, 1):
        a.ellipsoid(f'mask eye hollow {s}', (s * .048, -.184, 1.798), (.026, .016, .019), DARK, 16, 10)
        a.shard(f'mask jade gaze {s}', (s * .026, -.200, 1.796), (s * .068, -.193, 1.804), .0065, GLOW, 6)
        # Stern upward-swept gothic V-brow filigree ornament
        a.limb(f'mask brow filigree {s}', [
            (0, -.224, 1.824), (s * .044, -.212, 1.838), (s * .086, -.184, 1.852)
        ], [.005, .0055, .003], GOLD, sides=6, steps=3)
        a.limb(f'mask cheek hairline {s}', [
            (s * .055, -.198, 1.755), (s * .038, -.208, 1.685), (s * .014, -.212, 1.622)
        ], [.004, .004, .0025], SILVER, sides=6, steps=3)

    # Delicate multi-pronged dark gold/iron antler crown framing the pointed hood peak!
    crown_band = [(math.sin(t) * .162, -.045 - math.cos(t) * .182, 1.945 - .025 * abs(math.sin(t)))
                  for t in [(-1.85 + 3.70 * i / 28) for i in range(29)]]
    a.tube('antler crown circlet', crown_band, [.009] * len(crown_band), GOLD, sides=8, side=(0, 0, 1))
    a.limb('crown central spire', [
        (0, -.225, 1.95), (0, -.232, 2.10), (0, -.218, 2.24)
    ], [.013, .0085, .002], GOLD, sides=8, steps=4)
    for s in (-1, 1):
        a.limb(f'crown main antler {s}', [
            (s * .105, -.175, 1.94),
            (s * .180, -.125, 2.06),
            (s * .230, -.058, 2.18),
            (s * .198, -.030, 2.28),
        ], [.015, .012, .008, .002], GOLD, sides=8, steps=4)
        a.limb(f'crown front fork {s}', [
            (s * .162, -.140, 2.04),
            (s * .172, -.192, 2.13),
            (s * .135, -.212, 2.19),
        ], [.0095, .0065, .002], GOLD, sides=7, steps=3)
        a.limb(f'crown outer fork {s}', [
            (s * .212, -.085, 2.13),
            (s * .278, -.060, 2.19),
            (s * .298, -.042, 2.24),
        ], [.0085, .0055, .002], GOLD, sides=7, steps=3)
        a.limb(f'crown inner spire {s}', [
            (s * .060, -.212, 1.95),
            (s * .078, -.216, 2.05),
            (s * .066, -.208, 2.12),
        ], [.009, .006, .002], SILVER, sides=7, steps=3)

    # Rich high-resolution pleated flowing petrol-velvet cloak + clasps + chains
    ribbed_cloak('layered funerary cloak', VELVET, z_top=1.48, z_bot=.25,
                 y_top=.20, y_bot=.48, w_top=.28, w_bot=.49,
                 cols=22, rows=15, folds=6, fold_amp=.036, tatter=.18, split=.06)
    for s in (-1, 1):
        a.limb(f'cloak chest chain {s}', [
            (s * .210, -.185, 1.43), (s * .108, -.222, 1.38), (0, -.234, 1.35)
        ], [.006, .006, .006], GOLD, sides=8, steps=4)
        a.limb(f'cloak back chain {s}', [
            (s * .210, .215, 1.41), (s * .315, .360, .86), (s * .375, .425, .58)
        ], [.009, .008, .004], SILVER, sides=8, steps=4)

    h.hit_bones()
    a.export_part('hero_soulbinder')


def soulbinder_arm():
    a.reset()
    sculpted_loft('velvet upper sleeve', [
        (.01, .090, .086, 0, 6, .025, .004),
        (.16, .100, .094, 0, 6, .022, .004),
        (.31, .096, .088, 0, 0, .0, .0),
    ], VELVET, sides=32)
    sculpted_loft('sleeve flared cuff', [
        (-.04, .112, .106, 0, 8, .035, .006),
        (.04, .094, .090, 0, 6, .020, .004),
    ], VELVET, sides=32)
    a.hoop('sleeve cuff gold trim', -.035, .114, .108, .007, GOLD)
    a.ellipsoid('silver elbow couter', (0, .040, .02), (.080, .070, .076), SILVER, 18, 12)
    smooth_loft('funerary silver vambrace', [
        (-.26, .062, .060, 0),
        (-.16, .076, .072, 0),
        (-.05, .084, .080, 0),
    ], SILVER, sides=32, bevel=.005)
    for z in (-.24, -.14, -.06):
        a.hoop(f'vambrace gold band {z:.2f}', z, .082 if z > -.18 else .067,
               .078 if z > -.18 else .064, .007, GOLD)
    a.limb('vambrace outer ridge', [(0, .078, -.24), (0, .094, -.12), (0, .084, -.03)],
           [.009, .013, .009], GOLD, sides=8, steps=3)
    fingered_hand('soulbinder hand', -.275, DARK, SILVER, claw=False)
    h.normalize(a.ARM_SIZE, z_center=a.ARM_CENTER)
    a.export_part('hero_soulbinder_arm')


def soulbinder_leg():
    a.reset()
    smooth_loft('funerary thigh hose', [
        (.08, .084, .082, 0), (.24, .094, .090, 0), (.37, .088, .084, 0)
    ], DARK, sides=28, bevel=.004)
    a.ellipsoid('silver knee poleyn', (0, -.044, .080), (.090, .084, .080), SILVER, 20, 14)
    a.shard('knee gold boss', (0, -.120, .116), (0, -.120, .042), .018, GOLD, 6)
    smooth_loft('funerary silver greave', [
        (-.29, .064, .066, 0), (-.16, .080, .078, 0), (.03, .090, .084, 0)
    ], SILVER, sides=32, bevel=.005)
    a.limb('shin gold keel', [(0, -.072, -.26), (0, -.096, -.10), (0, -.094, .04)],
           [.010, .014, .010], GOLD, sides=8, steps=3)
    a.hoop('ankle gold cuff', -.275, .070, .072, .008, GOLD)
    a.ellipsoid('funerary leather boot', (0, -.068, -.345), (.082, .156, .052), LEATHER, 22, 14)
    a.ellipsoid('boot silver toecap', (0, -.164, -.350), (.052, .076, .036), SILVER, 16, 10)
    h.normalize(a.LEG_SIZE)
    a.export_part('hero_soulbinder_leg')


def soulbinder_mantle():
    a.reset()
    # Draped pleated petrol-velvet shoulder cowl-capelet
    sculpted_loft('draped velvet shoulder capelet', [
        (-.11, .52, .258, .012, 12, .038, .012),
        (-.01, .44, .228, .008, 12, .030, .009),
        (.08, .33, .188, .004, 10, .020, .006),
        (.15, .22, .148, .000, 0, .0, .0),
    ], VELVET, sides=48)
    a.hoop('mantle gold hem trim', -.105, .525, .262, .008, GOLD, cy=.012, lobes=12, amp=.036, wave=.012)
    # Sculpted curved gothic layered silver shoulder pauldrons (sleek arched plates!)
    for s in (-1, 1):
        sculpted_pauldron_shell(f'funerary pauldron upper {s}', s, s * .24, .010, .035, .26, .205, .105, SILVER, tilt=.28)
        sculpted_pauldron_shell(f'funerary pauldron lower {s}', s, s * .28, .012, -.025, .27, .195, .085, SILVER, tilt=.32)
        a.limb(f'pauldron gold crest {s}', [
            (s * .25, -.05, .125), (s * .42, .01, .095), (s * .52, .05, -.015)
        ], [.012, .015, .009], GOLD, sides=8, steps=4)
        ornate_medallion(f'mantle clasp {s}', (s * .225, -.208, -.020), .042, GOLD, GLOW)
    a.shard('mantle jade broach', (0, -.236, -.085), (0, -.228, .055), .028, GLOW, 6)
    h.normalize(a.MANTLE_SIZE)
    a.export_part('hero_soulbinder_mantle')


# ========================================================================
# 2. SOUL SPIRIT (契灵) — Ethereal Crowned Spectral Wraith Knight
#    Faithful to the ghostly crowned knight behind the Soulbinder in the portrait!
# ========================================================================

def spirit_body():
    a.reset()
    # Blazing inner spectral soul core (upper chest & ribcage ONLY — clean dark tassets below!)
    smooth_loft('spectral inner soul core', [
        (.93, .155, .118, -.005),
        (1.14, .218, .156, -.010),
        (1.38, .248, .165, -.008),
        (1.50, .148, .124, 0),
    ], CORE_GLOW, sides=36, bevel=.004)
    # Shadowed spectral void core inside pelvis (NO bright lower lump!)
    smooth_loft('spectral pelvis void', [
        (.66, .160, .120, 0), (.84, .195, .145, 0), (.95, .185, .138, 0)
    ], SPIRIT_SHROUD, sides=32, bevel=.004)

    # Angular gothic wraith-knight breastplate framing the glowing chest core
    for s in (-1, 1):
        a.plate(f'spectral breastplate wing {s}', [
            (s * .040, -.206, .99), (s * .162, -.192, 1.03),
            (s * .298, -.108, 1.24), (s * .315, -.072, 1.44),
            (s * .156, -.178, 1.46), (s * .046, -.210, 1.42),
        ], SPIRIT_ARMOR, .030)
        a.plate(f'spectral backplate wing {s}', [
            (s * .042, .175, 1.01), (s * .260, .132, 1.24),
            (s * .280, .102, 1.44), (s * .052, .162, 1.45),
        ], SPIRIT_ARMOR, .025)
        for i in range(4):
            z = 1.06 + i * .085
            a.limb(f'spectral chest energy rib {s}_{i}', [
                (s * .018, -.215, z), (s * .148, -.200, z + .035), (s * .290, -.092, z + .055)
            ], [.013, .011, .005], GLOW, sides=8, steps=3)

    # Layered gothic armored fauld waist plates + split spectral front tassets
    for i in range(3):
        z = .78 + i * .076
        sculpted_loft(f'spectral fauld plate {i}', [
            (z, .265 - i * .014, .188 - i * .010, 0, 6, .032, .007),
            (z + .054, .248 - i * .014, .174 - i * .010, 0, 6, .024, .005),
        ], SPIRIT_ARMOR, sides=36)
        a.hoop(f'spectral waist seam {i}', z - .006, .242 - i * .012, .170 - i * .008, .006, GLOW)
    # Dark spectral tattered front loin-tasset & flanking hip guards
    a.plate('spectral front tasset', [
        (-.085, -.192, .80), (.085, -.192, .80),
        (.062, -.218, .48), (0, -.226, .38), (-.062, -.218, .48),
    ], SPIRIT_SHROUD, .016)
    a.limb('spectral tasset rune spine', [(0, -.204, .78), (0, -.228, .44)], [.008, .003], GLOW, sides=6, steps=3)

    # High spiked gothic knight gorget
    sculpted_loft('spectral knight gorget', [
        (1.44, .210, .166, -.006, 6, .022, .005),
        (1.56, .185, .154, -.008, 6, .018, .004),
    ], SPIRIT_ARMOR, sides=36)
    a.hoop('spectral gorget glow rim', 1.56, .188, .158, .008, GLOW, cy=-.008)

    # SLEEK SCULPTED GOTHIC BARBUTE / GREATHELM (sharp faceted crown, NOT a boxy tin can!)
    sculpted_loft('wraith helm skull', [
        (1.58, .150, .162, -.008, 4, .020, .004),
        (1.73, .178, .192, -.010, 4, .026, .004),
        (1.86, .172, .186, -.006, 4, .020, .004),
        (1.96, .122, .135, .000, 4, .012, .002),
        (2.02, .052, .062, .005, 0, .0, .0),
    ], SPIRIT_ARMOR, sides=36)
    # Swept angular pointed cheek-visor plates meeting at a crisp central keel
    for s in (-1, 1):
        a.plate(f'wraith helm visor {s}', [
            (s * .014, -.240, 1.60), (s * .168, -.130, 1.65),
            (s * .184, -.118, 1.83), (s * .016, -.232, 1.85),
        ], SPIRIT_ARMOR, .026)
        a.limb(f'wraith brow ridge {s}', [
            (0, -.240, 1.845), (s * .105, -.202, 1.835), (s * .182, -.120, 1.825)
        ], [.014, .012, .009], SPIRIT_ARMOR, sides=8, steps=3)
        a.limb(f'wraith visor horizontal glow {s}', [
            (s * .020, -.242, 1.785), (s * .132, -.190, 1.792)
        ], [.012, .008], CORE_GLOW, sides=8, steps=2)
    # Iconic glowing vertical T-visor slit
    a.limb('wraith visor vertical glow', [
        (0, -.248, 1.625), (0, -.250, 1.815)
    ], [.010, .012], CORE_GLOW, sides=8, steps=3)

    # Tall Gothic Spectral King Crown forged onto the helm
    a.hoop('wraith crown band', 1.925, .165, .174, .014, SPIRIT_ARMOR, cy=-.006)
    a.hoop('wraith crown glow band', 1.938, .168, .176, .006, GLOW, cy=-.006)
    spire_angles = [0.0, 0.62, -0.62, 1.32, -1.32, 2.15, -2.15]
    spire_heights = [.28, .21, .21, .17, .17, .14, .14]
    for ang, sh in zip(spire_angles, spire_heights):
        bx = math.sin(ang) * .164
        by = -.006 - math.cos(ang) * .172
        tx = math.sin(ang) * (.182 + sh * .08)
        ty = -.006 - math.cos(ang) * (.190 + sh * .08)
        a.shard(f'wraith crown spire {ang:.2f}', (tx, ty, 1.93 + sh), (bx, by, 1.91),
                .022 if sh > .24 else .016, SPIRIT_ARMOR, 5)
        a.shard(f'wraith crown flame {ang:.2f}', (tx, ty, 1.955 + sh), (tx, ty, 1.91 + sh * .65),
                .009, CORE_GLOW, 5)

    # Billowing tattered spectral wraith shroud flaring behind
    ribbed_cloak('tattered spectral shroud', SPIRIT_SHROUD, z_top=1.46, z_bot=.24,
                 y_top=.18, y_bot=.58, w_top=.29, w_bot=.54,
                 cols=20, rows=14, folds=5, fold_amp=.046, tatter=.48, split=.20)

    # Flowing S-curved ethereal ectoplasmic wisps curling gracefully behind the shoulders
    for s in (-1, 1):
        a.limb(f'spectral wraith wisp upper {s}', [
            (s * .20, .14, 1.46),
            (s * .34, .26, 1.68),
            (s * .42, .42, 1.90),
            (s * .32, .56, 2.10),
        ], [.032, .026, .016, .002], GLOW, sides=8, steps=5)
        a.limb(f'spectral wraith wisp lower {s}', [
            (s * .18, .18, 1.05),
            (s * .34, .38, .74),
            (s * .44, .58, .44),
            (s * .36, .72, .22),
        ], [.032, .026, .016, .002], GLOW, sides=8, steps=5)
    a.limb('spectral dorsal wisp', [
        (0, .20, 1.38), (0, .38, 1.62), (0, .54, 1.88), (0, .66, 2.08)
    ], [.026, .022, .012, .002], CORE_GLOW, sides=8, steps=5)

    h.hit_bones()
    a.export_part('soul_spirit')


def spirit_arm():
    a.reset()
    smooth_loft('spectral arm plate', [
        (-.26, .066, .064, 0), (-.10, .090, .084, 0),
        (.08, .082, .078, 0), (.31, .094, .088, 0),
    ], SPIRIT_ARMOR, sides=28, bevel=.005)
    a.shard('spectral elbow fin', (0, .142, .06), (0, .045, -.05), .034, SPIRIT_ARMOR, 5)
    a.limb('spectral arm energy vein', [
        (0, -.072, -.25), (0, -.094, -.08), (0, -.090, .12), (0, -.096, .28)
    ], [.009, .013, .011, .007], GLOW, sides=8, steps=4)
    fingered_hand('spectral clawed gauntlet', -.275, SPIRIT_ARMOR, CORE_GLOW, claw=True)
    h.normalize(a.ARM_SIZE, z_center=a.ARM_CENTER)
    a.export_part('soul_spirit_arm')


def spirit_leg():
    a.reset()
    smooth_loft('spectral thigh armor', [
        (.06, .080, .078, 0), (.24, .094, .090, 0), (.36, .086, .082, 0)
    ], SPIRIT_ARMOR, sides=28, bevel=.005)
    a.shard('spectral knee spike', (0, -.126, .125), (0, -.066, .020), .032, SPIRIT_ARMOR, 5)
    smooth_loft('spectral wraith shin', [
        (-.30, .024, .032, .024), (-.15, .062, .064, .010), (.05, .082, .080, 0)
    ], SPIRIT_ARMOR, sides=28, bevel=.004)
    a.shard('spectral mist tail core', (0, .052, -.38), (0, -.015, -.04), .044, GLOW, 6)
    a.limb('spectral shin streak', [(0, -.052, -.28), (0, -.086, -.08), (0, -.086, .12)],
           [.006, .013, .009], CORE_GLOW, sides=6, steps=3)
    h.normalize(a.LEG_SIZE)
    a.export_part('soul_spirit_leg')


def spirit_mantle():
    a.reset()
    # Sculpted arched bladed spectral knight pauldrons with upward-swept wings!
    for s in (-1, 1):
        sculpted_pauldron_shell(f'spectral knight pauldron upper {s}', s, s * .22, .010, .055, .28, .215, .115, SPIRIT_ARMOR, tilt=.22, flair=.05)
        sculpted_pauldron_shell(f'spectral knight pauldron lower {s}', s, s * .27, .012, -.015, .29, .205, .090, SPIRIT_ARMOR, tilt=.28, flair=.05)
        a.shard(f'spectral shoulder wing outer {s}',
                (s * .62, .06, .24), (s * .34, .01, .00), .054, SPIRIT_ARMOR, 5)
        a.shard(f'spectral shoulder wing inner {s}',
                (s * .50, .05, .30), (s * .28, .01, .05), .038, GLOW, 5)
        a.limb(f'spectral pauldron vein {s}', [
            (s * .24, -.10, .11), (s * .44, -.01, .14), (s * .56, .05, .22)
        ], [.011, .015, .005], CORE_GLOW, sides=8, steps=4)
    a.shard('spectral collar crystal', (0, -.220, -.07), (0, -.210, .07), .038, CORE_GLOW, 6)
    h.normalize(a.MANTLE_SIZE)
    a.export_part('soul_spirit_mantle')


# ========================================================================
# 3. BONE SENTRY (骨卫) — Armored Ossuary Skeleton Guardian
# ========================================================================

def bone_sentry_body():
    a.reset()
    smooth_loft('ossuary shadow core', [
        (.72, .160, .122, 0), (1.02, .170, .128, 0),
        (1.30, .215, .150, 0), (1.48, .165, .125, 0),
    ], DARK, sides=28, bevel=.004)
    for i in range(8):
        z = .86 + i * .086
        a.ellipsoid(f'ossuary vertebra {i}', (0, .135, z), (.046, .040, .032), BONE, 14, 10)
        a.shard(f'ossuary dorsal spine {i}', (0, .205, z + .012), (0, .145, z - .005), .018, BONE, 5)
    a.plate('ossuary bone sternum', [
        (0, -.198, 1.00), (.056, -.192, 1.12), (.070, -.196, 1.36),
        (0, -.204, 1.44), (-.070, -.196, 1.36), (-.056, -.192, 1.12),
    ], BONE, .032)
    for s in (-1, 1):
        for i in range(5):
            z = 1.02 + i * .082
            rx = .232 + .055 * math.sin((i / 4.) * math.pi * .8)
            a.limb(f'ossuary bone rib {s}_{i}', [
                (s * .042, -.195, z), (s * (rx * .62), -.178, z + .022),
                (s * rx, -.035, z + .040), (s * (rx * .75), .125, z + .035),
            ], [(.022, .016), (.024, .018), (.020, .014), (.012, .010)], BONE, sides=10, steps=4)
        # Crossed dark-iron chest harness straps over the ribs
        a.limb(f'ossuary iron harness {s}', [
            (s * .20, -.155, 1.44), (0, -.215, 1.18), (-s * .18, -.175, .94)
        ], [.016, .016, .014], IRON, sides=8, steps=4)
    smooth_loft('ossuary iron waist belt', [(.86, .242, .180, 0), (.95, .245, .182, 0)], IRON, sides=36, bevel=.006)
    a.plate('ossuary belt iron clasp', [
        (-.065, -.205, .86), (.065, -.205, .86), (.058, -.205, .95), (-.058, -.205, .95)
    ], IRON, .026)
    a.shard('ossuary belt jade gem', (0, -.222, .935), (0, -.222, .875), .015, GLOW, 6)
    for s in (-1, 1):
        a.plate(f'ossuary bone hip guard {s}', [
            (s * .07, -.188, .88), (s * .245, -.125, .87),
            (s * .265, -.145, .56), (s * .165, -.200, .48), (s * .085, -.208, .58),
        ], IRON, .025)

    # Exposed sculpted anatomical bleached Skull + Jaw + Spiked Iron War Circlet
    smooth_loft('ossuary neck vertebrae', [(1.46, .078, .078, 0), (1.59, .068, .072, 0)], BONE, sides=22, bevel=.004)
    a.ellipsoid('ossuary skull cranium', (0, .015, 1.785), (.148, .172, .162), BONE, 30, 18)
    a.ellipsoid('ossuary skull face', (0, -.092, 1.720), (.110, .092, .108), BONE, 24, 14)
    a.ellipsoid('ossuary jawbone', (0, -.102, 1.615), (.084, .080, .058), BONE, 22, 12)
    for s in (-1, 1):
        a.ellipsoid(f'ossuary cheekbone {s}', (s * .088, -.122, 1.725), (.042, .036, .030), BONE, 14, 10)
        a.ellipsoid(f'ossuary eye socket {s}', (s * .052, -.164, 1.765), (.035, .024, .030), DARK, 16, 10)
        a.ellipsoid(f'ossuary eye fire {s}', (s * .052, -.180, 1.765), (.016, .012, .014), GLOW, 12, 8)
    a.shard('ossuary nasal hollow', (0, -.194, 1.735), (0, -.190, 1.682), .017, DARK, 4)
    for i in range(6):
        tx = (i - 2.5) * .021
        a.limb(f'ossuary tooth {i}', [(tx, -.174, 1.618), (tx, -.182, 1.662)], [.0065, .0075], BONE, sides=6, steps=2)
    a.hoop('ossuary brow iron band', 1.842, .156, .174, .016, IRON, cy=.005)
    for s in (-1, 1):
        a.shard(f'ossuary brow spike {s}', (s * .160, -.122, 2.02), (s * .130, -.112, 1.84), .025, IRON, 5)
    a.shard('ossuary brow center spike', (0, -.188, 2.04), (0, -.170, 1.84), .026, IRON, 5)

    ribbed_cloak('ossuary burial shroud', SPIRIT_SHROUD, z_top=1.44, z_bot=.50,
                 y_top=.18, y_bot=.38, w_top=.24, w_bot=.38,
                 cols=16, rows=12, folds=4, fold_amp=.030, tatter=.42, split=.10)

    h.hit_bones()
    a.export_part('bone_sentry')


def bone_sentry_arm():
    a.reset()
    smooth_loft('ossuary humerus bone', [(.04, .060, .058, 0), (.28, .070, .066, 0)], BONE, sides=22, bevel=.004)
    a.ellipsoid('ossuary elbow iron cap', (0, .035, .02), (.086, .078, .080), IRON, 18, 12)
    for s in (-1, 1):
        a.limb(f'ossuary forearm bone {s}', [(s * .025, 0, -.25), (s * .028, 0, .02)], [.023, .025], BONE, sides=10, steps=2)
    smooth_loft('ossuary iron bracer', [(-.24, .075, .073, 0), (-.08, .086, .082, 0)], IRON, sides=26, bevel=.005)
    a.shard('ossuary arm bone spike', (0, .122, -.10), (0, .068, -.16), .026, BONE, 5)
    fingered_hand('ossuary skeletal hand', -.270, BONE, IRON, claw=True)
    h.normalize(a.ARM_SIZE, z_center=a.ARM_CENTER)
    a.export_part('bone_sentry_arm')


def bone_sentry_leg():
    a.reset()
    smooth_loft('ossuary femur', [(.08, .062, .062, 0), (.35, .076, .074, 0)], BONE, sides=22, bevel=.004)
    a.ellipsoid('ossuary knee bone cap', (0, -.045, .075), (.090, .086, .082), IRON, 18, 12)
    smooth_loft('ossuary shin greave', [(-.29, .068, .068, 0), (.02, .090, .086, 0)], BONE, sides=26, bevel=.005)
    a.limb('ossuary shin iron keel', [(0, -.074, -.26), (0, -.096, -.06), (0, -.090, .05)],
           [.012, .015, .012], IRON, sides=8, steps=3)
    a.ellipsoid('ossuary iron boot', (0, -.068, -.348), (.086, .154, .054), IRON, 20, 12)
    h.normalize(a.LEG_SIZE)
    a.export_part('bone_sentry_leg')


def bone_sentry_mantle():
    a.reset()
    # Layered arched bone-plate pauldrons bound with dark iron + jagged bone tusks (NO balloon balls!)
    for s in (-1, 1):
        sculpted_pauldron_shell(f'ossuary bone pauldron upper {s}', s, s * .22, .010, .050, .28, .210, .105, BONE, tilt=.25, flair=.04)
        sculpted_pauldron_shell(f'ossuary iron pauldron lower {s}', s, s * .28, .012, -.020, .28, .195, .085, IRON, tilt=.30, flair=.04)
        a.limb(f'ossuary pauldron iron strap {s}', [
            (s * .22, -.09, .11), (s * .42, .00, .08), (s * .52, .04, -.03)
        ], [.014, .016, .010], IRON, sides=8, steps=4)
        for i in range(3):
            a.shard(f'ossuary shoulder tusk {s}_{i}',
                    (s * (.48 + i * .07), -.05 + i * .05, .24 - i * .03),
                    (s * (.34 + i * .05), -.01 + i * .02, .04),
                    .028, BONE, 6)
    a.shard('ossuary collar iron spike', (0, -.215, -.07), (0, -.210, .06), .028, IRON, 5)
    h.normalize(a.MANTLE_SIZE)
    a.export_part('bone_sentry_mantle')


# ========================================================================
# 4. WEAPONS — Ornate Gnarled Soul Staff & Blazing Spectral Greatsword
# ========================================================================

def soul_staff():
    a.reset()
    haft_pts = [
        (0, 0, -.42), (.012, -.008, -.10), (-.010, .010, .30),
        (.014, -.006, .70), (-.008, .008, 1.06), (0, 0, 1.24),
    ]
    a.limb('gnarled soul staff haft', haft_pts,
           [.020, .026, .028, .026, .030, .034], WOOD, sides=16, steps=5)
    smooth_loft('staff leather grip', [(-.24, .031, .031, 0), (.18, .031, .031, 0)], LEATHER, sides=22, bevel=.003)
    for z in (-.38, -.25, -.14, -.02, .10, .20, .58, .92, 1.08):
        a.hoop(f'staff binding {z:.2f}', z, .034, .034, .008, GOLD if z > .5 else SILVER)
    a.shard('staff bottom ferrule', (0, 0, -.54), (0, 0, -.38), .024, SILVER, 8)
    for s in (-1, 1):
        a.limb(f'staff antler cradle outer {s}', [
            (0, 0, 1.08), (s * .14, -.02, 1.24), (s * .21, .01, 1.44),
            (s * .14, .02, 1.62), (s * .04, 0, 1.72),
        ], [.032, .026, .020, .012, .003], BONE, sides=12, steps=5)
        a.limb(f'staff antler spur {s}', [
            (s * .18, 0, 1.38), (s * .27, -.02, 1.48), (s * .25, -.01, 1.57)
        ], [.014, .009, .002], BONE, sides=8, steps=3)
        a.limb(f'staff silver brace {s}', [
            (0, 0, 1.14), (s * .09, .02, 1.32), (s * .07, 0, 1.50)
        ], [.012, .010, .004], SILVER, sides=8, steps=3)
    a.shard('suspended soul crystal', (0, 0, 1.60), (0, 0, 1.20), .074, GLOW, 8, belly=.46)
    a.shard('soul crystal inner heart', (0, 0, 1.54), (0, 0, 1.26), .044, CORE_GLOW, 6, belly=.50)
    a.hoop('soul crystal halo ring', 1.40, .118, .118, .007, GOLD)
    a.limb('staff censer chain', [(.13, 0, 1.22), (.16, 0, 1.06)], [.005, .005], GOLD, sides=6, steps=2)
    a.ellipsoid('staff censer orb', (.16, 0, 1.01), (.028, .028, .034), GOLD, 16, 10)
    a.export_single('soul_staff')


def soul_blade():
    a.reset()
    smooth_loft('spectral sword grip', [(-.28, .026, .024, 0), (.16, .029, .025, 0)], LEATHER, sides=20, bevel=.003)
    for z in (-.22, -.12, -.02, .08):
        a.hoop(f'sword grip ring {z:.2f}', z, .031, .028, .006, SILVER)
    a.ellipsoid('spectral pommel', (0, 0, -.32), (.048, .042, .052), SPIRIT_ARMOR, 16, 10)
    a.shard('spectral pommel crystal', (0, 0, -.41), (0, 0, -.32), .026, CORE_GLOW, 6)
    for s in (-1, 1):
        a.limb(f'spectral crossguard wing {s}', [
            (0, 0, .19), (s * .14, 0, .22), (s * .26, 0, .31), (s * .29, 0, .37)
        ], [(.024, .034), (.020, .028), (.014, .020), (.004, .005)], SPIRIT_ARMOR, sides=8, steps=4)
    a.shard('crossguard soul jewel', (0, -.032, .25), (0, -.032, .15), .024, CORE_GLOW, 6)
    # Centered double-beveled gothic greatsword blade (spans y = -0.015 .. +0.015)
    a.plate('spectral greatsword outer blade', [
        (-.088, -.015, .22), (.088, -.015, .22),
        (.098, -.014, .54), (.072, -.012, 1.18),
        (0, -.010, 1.56),
        (-.072, -.012, 1.18), (-.098, -.014, .54),
    ], SPIRIT_ARMOR, .028)
    # Blazing luminous spectral jade fuller protruding cleanly on BOTH faces of the blade!
    for face_y in (-.018, .016):
        a.limb(f'spectral blade energy fuller {face_y:.2f}', [
            (0, face_y, .26), (0, face_y, .72), (0, face_y, 1.18), (0, face_y, 1.46)
        ], [(.024, .010), (.022, .010), (.014, .008), (.003, .003)], CORE_GLOW, sides=8, steps=4)
    # Glowing spectral rune edges along the ricasso
    for s in (-1, 1):
        a.shard(f'spectral blade fang {s}', (s * .148, 0, .48), (s * .076, 0, .34), .020, GLOW, 5)
    a.export_single('soul_blade')


# ========================================================================
# PREVIEW RENDERER
# ========================================================================

def render_previews():
    preview = a.ROOT / 'target/validation/soulbinder-art'
    preview.mkdir(parents=True, exist_ok=True)

    a.reset()
    a.assemble('hero_soulbinder', 'soul_staff')
    scene, cam = a.stage(24)
    a.shoot(scene, cam, preview / 'soulbinder_front.png', (0.6, -5.8, 1.9), (0, 0, 1.12), 2.55, (720, 900))
    a.shoot(scene, cam, preview / 'soulbinder_iso.png', (3.0, -5.5, 3.6), (0, 0, 1.05), 2.85, (720, 900))

    a.reset()
    a.assemble('soul_spirit', 'soul_blade')
    scene, cam = a.stage(24)
    a.shoot(scene, cam, preview / 'spirit_iso.png', (3.0, -5.5, 3.6), (0, 0, 1.05), 2.85, (720, 900))

    a.reset()
    def place_char(prefix, weap, offset):
        before = set(bpy.context.scene.objects)
        a.assemble(prefix, weap)
        for obj in bpy.context.scene.objects:
            if obj not in before and obj.parent is None:
                obj.location += Vector(offset)
    place_char('hero_soulbinder', 'soul_staff', (0.0, 0.0, 0.0))
    place_char('soul_spirit', 'soul_blade', (-1.55, 0.65, 0.16))
    place_char('bone_sentry', 'soul_blade', (1.55, 0.45, 0.0))
    scene, cam = a.stage(28)
    a.shoot(scene, cam, preview / 'trio_isometric.png', (2.6, -7.2, 6.2), (0, 0.2, 1.0), 5.2, (1280, 820))
    print('PREVIEWS RENDERED TO', preview)


if __name__ == '__main__':
    args = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
    if '--preview-only' not in args:
        soulbinder_body()
        soulbinder_arm()
        soulbinder_leg()
        soulbinder_mantle()

        spirit_body()
        spirit_arm()
        spirit_leg()
        spirit_mantle()

        bone_sentry_body()
        bone_sentry_arm()
        bone_sentry_leg()
        bone_sentry_mantle()

        soul_staff()
        soul_blade()
    if '--no-preview' not in args:
        render_previews()
