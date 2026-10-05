"""Original, opaque single-layer contact effects. No cores, rods or billboards.

Coordinates use ground-centred Z-up. Runtime owns motion and brief lifetimes.
"""
import json
import math
from pathlib import Path
import struct
import sys
import bpy
sys.path.insert(0, str(Path(__file__).resolve().parent))
import generate_reliquary_props as art

NAMES = ['contact_steel', 'contact_expose', 'contact_blood', 'contact_cleanse',
         'contact_reap', 'contact_ember', 'contact_frost', 'proc_vigor', 'proc_focus']
COLORS = [(0.70,0.64,0.48),(0.72,0.47,0.14),(0.36,0.018,0.012),
          (0.80,0.64,0.29),(0.38,0.035,0.07),(0.86,0.22,0.025),
          (0.27,0.62,0.76),(0.66,0.48,0.18),(0.16,0.58,0.61)]


def polygon(name, vertices, mat):
    mesh=bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], [tuple(range(len(vertices)))])
    obj=bpy.data.objects.new(name,mesh)
    bpy.context.collection.objects.link(obj)
    art.finish(obj,name,mat)


def stroke(name, points, width, mat):
    # Connected flat etched ribbon with pointed ends, never cylindrical rods.
    for i,(a,b) in enumerate(zip(points,points[1:])):
        dx,dy=b[0]-a[0],b[1]-a[1]
        size=max(math.hypot(dx,dy),.001)
        nx,ny=-dy/size*width,dx/size*width
        wa=0.0 if i==0 else 1.0
        wb=0.0 if i==len(points)-2 else 1.0
        polygon(name,[(a[0]+nx*wa,a[1]+ny*wa,a[2]),
                     (b[0]+nx*wb,b[1]+ny*wb,b[2]),
                     (b[0]-nx*wb,b[1]-ny*wb,b[2]),
                     (a[0]-nx*wa,a[1]-ny*wa,a[2])],mat)


def arc(name, radius, phase, sweep, width, mat, height=.035):
    points=[(math.cos(phase+sweep*i/12)*radius,
             math.sin(phase+sweep*i/12)*radius,height) for i in range(13)]
    stroke(name,points,width,mat)


def flakes(name, mat, count, length, width, red=False):
    for i in range(count):
        a=.4+i*2.399
        r=.12+i*.035
        x,y=math.cos(a)*r,math.sin(a)*r
        z=.04+(i%3)*.095
        # Unequal, double-sided slivers; only blood points downward.
        polygon(name,[(x-width,y,z),(x+width*.4,y+width,z+.025),
                      (x+math.cos(a)*length,y+math.sin(a)*length,z+(-.15 if red else .10)),
                      (x,y-width,z-.012)],mat)


def build(name,index):
    art.reset()
    mat=art.material(name+' etched opaque fragments',COLORS[index],glow=.24 if index==2 else .65)
    mat.use_backface_culling=False
    if name=='contact_steel': flakes('forged steel splinters',mat,5,.18,.028)
    elif name=='contact_blood': flakes('falling wound droplets',mat,4,.08,.037,True)
    elif name=='contact_ember': flakes('rising cinder scales',mat,6,.105,.030)
    elif name=='contact_expose':
        for side in [-1,1]:
            stroke('fractured golden seam',[(side*.04,-.20,.02),(side*.18,-.08,.09),
                    (side*.11,.07,.16),(side*.32,.18,.22)],.025,mat)
    elif name=='contact_cleanse':
        for i in range(3): arc('closing consecration arc',.48,i*math.tau/3,.95,.023,mat)
    elif name=='contact_reap':
        for i in range(3):
            stroke('returning blood thread',[(-.13+i*.10,-.24,.03),
                   (-.20+i*.09,-.02,.10),(-.07+i*.07,.22,.06)],.024,mat)
    elif name=='contact_frost':
        for i in range(6):
            a=i*math.tau/6
            points=[(math.cos(a+turn)*r,math.sin(a+turn)*r,.035)
                    for r,turn in [(.09,0),(.23,.11),(.32,-.05),(.49,.03)]]
            stroke('ice fracture vein',points,.022,mat)
    elif name=='proc_vigor':
        for side in [-1,1]:
            stroke('broken guardian chevron',[(side*.08,-.38,.04),(side*.32,-.20,.04),
                   (side*.38,.10,.04),(side*.20,.25,.04)],.030,mat)
    else:
        for i in range(2): arc('inward focus current',.43+i*.08,i*math.pi,1.65,.026,mat)
    return art.export(name, semantic_name=name+' sparse contact artwork')


def main():
    report=[build(name,i) for i,name in enumerate(NAMES)]
    for name in NAMES:
        data=(art.OUT/f'{name}.glb').read_bytes()
        size=struct.unpack_from('<I',data,12)[0]
        doc=json.loads(data[20:20+size])
        assert all(m.get('alphaMode','OPAQUE')=='OPAQUE' for m in doc['materials'])
        assert len(doc['meshes'])==1 and len(doc['meshes'][0]['primitives'])==1
    out=art.ROOT/'target/validation/combat-contacts'
    out.mkdir(parents=True,exist_ok=True)
    (out/'models.json').write_text(json.dumps(report,indent=2)+'\n')
    print('Verified 9 opaque, single-mesh, single-material contact effects.')

if __name__=='__main__': main()
