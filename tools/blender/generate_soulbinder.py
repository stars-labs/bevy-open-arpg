"""Soulbinder and two summoned retainers. Same anchored part contract as generate_hero.
blender -b --factory-startup --python tools/blender/generate_soulbinder.py
"""
import importlib.util
from pathlib import Path
import math
import bpy
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('class_art', HERE/'generate_hero_classes.py')
a = importlib.util.module_from_spec(spec); spec.loader.exec_module(a)
h = a.hero
CLOTH = h.material('soulbinder petrol velvet', (.025,.095,.09), 0,.85)
SILVER = h.material('funerary tarnished silver', (.32,.39,.37), .76,.38)
GLOW = h.material('bound soul jade', (.18,.78,.58), 0,.34,1.3)
SPIRIT = h.material('spectral patinated armor', (.06,.24,.21), .55,.36)


def body(prefix, summoned=False, bone=False):
    a.reset()
    mat = a.BONE if bone else SPIRIT if summoned else CLOTH
    h.loft('ribbed funerary cuirass', [(.74,.23,.16,0),(.95,.22,.16,0),(1.25,.29,.20,0),(1.48,.30,.18,0)], mat)
    for s in (-1,1):
        for i in range(4):
            z=1.04+i*.095
            a.limb('engraved rib '+str(i), [(s*.035,-.218,z),(s*.15,-.205,z+.045),(s*.28,-.10,z+.065)], [.023,.020,.006], SILVER if not bone else a.BONE,6)
        a.plate('split hip vestment',[(s*.04,-.19,.90),(s*.23,-.16,.91),(s*.26,-.21,.55),(s*.13,-.23,.48)],mat,.025)
    h.loft('neck gorget',[(1.43,.17,.15,0),(1.57,.16,.14,0)],SILVER)
    a.ellipsoid('ivory funeral mask',(0,-.015,1.73),(.175,.155,.22),a.BONE)
    for s in (-1,1):
        a.ellipsoid('shadowed eye socket',(s*.063,-.155,1.765),(.042,.019,.042),h.DARK)
        a.shard('jade eye slit',(s*.035,-.179,1.77),(s*.086,-.174,1.765),.009,GLOW,4)
        a.limb('curved crown horn',[(s*.15,.03,1.87),(s*.24,.08,2.00),(s*.19,.09,2.10)], [.03,.026,.002],SILVER,7)
    for i in range(5):
        a.limb('mask teeth',[((i-2)*.024,-.161,1.60),((i-2)*.024,-.172,1.665)],[.007,.009],SILVER,5)
    if not summoned:
        grid=[[(x*(.24+r*.022),.19+r*.025,1.45-r*.10-(.08 if abs(x)<.1 else 0)) for x in (-1,-.5,0,.5,1)] for r in range(9)]
        a.sheet('split funeral cape',grid,CLOTH,.02)
        for s in (-1,1):
            a.limb('cape chain',[(s*.20,.235,1.40),(s*.30,.38,.80),(s*.38,.40,.61)],[.014,.012,.004],SILVER,6)
    else:
        for s in (-1,1):
            a.shard('shoulder soul crest',(s*.28,.02,1.72),(s*.23,.02,1.40),.05,GLOW,5)
    h.hit_bones()
    a.export_part(prefix)


def arm(prefix,bone=False):
    a.reset()
    h.loft('jointed vambrace',[(-.26,.075,.075,0),(-.11,.086,.085,0),(.10,.079,.075,0),(.31,.093,.086,0)],a.BONE if bone else SPIRIT)
    for z in (-.22,-.06,.14,.28): a.hoop('silver arm binding',z,.091,.086,.008,SILVER)
    a.ellipsoid('articulated hand',(0,-.003,-.32),(.055,.055,.072),a.BONE)
    for s in (-1,1): a.shard('arm wing',(s*.10,.02,.29),(s*.08,.02,.10),.035,SILVER,4)
    h.normalize(a.ARM_SIZE,z_center=a.ARM_CENTER); a.export_part(prefix+'_arm')


def leg(prefix,bone=False):
    a.reset()
    h.loft('jointed greave',[(-.32,.064,.074,0),(-.12,.079,.08,0),(.10,.065,.065,0),(.35,.087,.082,0)],a.BONE if bone else SPIRIT)
    a.plate('silver shin keel',[(-.047,-.09,-.27),(.047,-.09,-.27),(.064,-.088,.01),(0,-.105,.15),(-.064,-.088,.01)],SILVER,.012)
    a.ellipsoid('pointed funerary boot',(0,-.067,-.36),(.08,.15,.052),h.LEATHER)
    h.normalize(a.LEG_SIZE); a.export_part(prefix+'_leg')


def mantle(prefix,bone=False):
    a.reset()
    a.skin('layered funerary shoulders',[a.zring(-.08,.53,.24),a.zring(.03,.39,.20),a.zring(.13,.23,.15)],a.BONE if bone else SILVER)
    for s in (-1,1):
        for i in range(3): a.shard('pauldron rib', (s*(.37+i*.06),.02,-.10-i*.025),(s*(.29+i*.05),.02,.06),.035,SPIRIT,5)
    a.shard('jade clasp',(0,-.245,-.12),(0,-.24,.05),.037,GLOW,5)
    h.normalize(a.MANTLE_SIZE); a.export_part(prefix+'_mantle')


def weapon(name,blade=False):
    a.reset()
    a.limb('ritual haft',[(0,0,-.38),(0,0,.65),(0,0,1.23 if not blade else .70)],[.024,.026,.019],h.LEATHER,10)
    for z in (-.26,-.16,-.06,.10,.54): a.hoop('grip silver binding',z,.029,.029,.007,SILVER)
    if blade:
        a.plate('crescent soul blade',[(0,0,.26),(.09,0,.45),(.20,0,.86),(.10,0,1.22),(-.03,0,1.36),(.01,0,.85),(-.03,0,.46)],SILVER,.04)
        a.shard('blade jade inset',(.09,-.03,1.03),(.04,-.03,.57),.017,GLOW,4)
    else:
        for s in (-1,1): a.limb('staff horn',[(0,0,1.04),(s*.17,0,1.25),(s*.16,0,1.48),(s*.06,0,1.62)],[.035,.027,.018,.002],a.BONE,8)
        a.shard('suspended soul crystal',(0,0,1.52),(0,0,1.17),.075,GLOW,6)
    a.export_single(name)


if __name__=='__main__':
    for prefix,bone,summoned in [('hero_soulbinder',False,False),('soul_spirit',False,True),('bone_sentry',True,True)]:
        body(prefix,summoned,bone); arm(prefix,bone); leg(prefix,bone); mantle(prefix,bone)
    weapon('soul_staff'); weapon('soul_blade',True)
    a.reset(); a.assemble('hero_soulbinder','soul_staff')
    scene,cam=a.stage(16)
    preview=a.ROOT/'target/validation/soulbinder-art'; preview.mkdir(parents=True,exist_ok=True)
    a.shoot(scene,cam,preview/'soulbinder.png',(3,-6,3.4),(0,0,1.1),3.3,(620,760))
