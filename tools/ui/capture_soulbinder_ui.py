#!/usr/bin/env python3
"""Isolated real-window screenshots of the four-class picker and summon talent tree."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time


def capture(binary, output):
    output.mkdir(parents=True,exist_ok=True)
    asset_link=output/'assets'
    if not asset_link.exists(): asset_link.symlink_to(Path(__file__).resolve().parents[2]/'assets',target_is_directory=True)
    r,w=os.pipe()
    with (output/'xvfb.log').open('w') as xlog:
        server=subprocess.Popen(['Xvfb','-displayfd',str(w),'-screen','0','1280x720x24','-nolisten','tcp'],pass_fds=(w,),stdout=xlog,stderr=xlog)
        os.close(w)
        with os.fdopen(r) as pipe: display=pipe.readline().strip()
        env=dict(os.environ,DISPLAY=':'+display,WINIT_UNIX_BACKEND='x11',XDG_CONFIG_HOME=str(output/'config'),BEVY_OPEN_ARPG_COMBAT_REVIEW='soulbinder-idle')
        env.pop('WAYLAND_DISPLAY',None)
        sysroot=Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())
        env['LD_LIBRARY_PATH']=os.pathsep.join([str(binary.parent/'deps'),env.get('LD_LIBRARY_PATH',''),*map(str,(sysroot/'lib/rustlib').glob('*/lib'))])
        with (output/'game.log').open('w') as log:
            game=subprocess.Popen([str(binary),'--x11','--no-audio'],cwd=output,env=env,stdout=log,stderr=log)
            def xdo(*args,check=True): return subprocess.run(['xdotool',*map(str,args)],env=env,check=check,capture_output=True,text=True)
            try:
                window=None
                for _ in range(160):
                    if game.poll() is not None: raise RuntimeError('Game exited before window creation')
                    result=xdo('search','--pid',game.pid,'--name','Bevy Open ARPG',check=False)
                    if result.returncode==0 and result.stdout.strip(): window=result.stdout.splitlines()[0];break
                    time.sleep(.25)
                if window is None: raise RuntimeError('No game window')
                xdo('windowfocus',window);xdo('windowmove',window,0,0);xdo('windowsize',window,1280,720)
                time.sleep(5)
                def screenshot(name):
                    time.sleep(.7)
                    subprocess.run(['import','-display',env['DISPLAY'],'-window',window,str(output/(name+'.png'))],check=True,timeout=20)
                def click(x,y):
                    xdo('mousemove','--window',window,x,y);xdo('mousedown',1);time.sleep(.1);xdo('mouseup',1);time.sleep(.4)
                click(240,370)
                for _ in range(3): xdo('key','Right');time.sleep(.15)
                screenshot('class-picker-1280')
                xdo('windowsize',window,960,720);screenshot('class-picker-960')
                xdo('windowsize',window,1280,720);time.sleep(.4);xdo('key','Return')
                for _ in range(100):
                    if 'COMBAT_REVIEW_READY' in (output/'game.log').read_text():break
                    time.sleep(.2)
                else: raise RuntimeError('Fixture not ready')
                xdo('key','k');screenshot('skills-1280')
                # Skill-panel tab row sits below the large section heading.
                click(410,124);screenshot('talents-1280')
                click(420,365);screenshot('talents-no-points')
                click(610,205);screenshot('talents-reset-near-enemy')
                xdo('windowsize',window,960,720);screenshot('talents-960')
                xdo('mousemove','--window',window,550,400);xdo('click','--repeat',3,'--delay',80,5);screenshot('talents-960-bottom')
                text=(output/'game.log').read_text()
                if 'panicked' in text or 'ERROR' in text: raise RuntimeError('Runtime errors in UI capture')
                (output/'result.json').write_text(json.dumps({'screenshots':[p.name for p in output.glob('*.png')],'panic':False,'fixture':'soulbinder-idle','input':'right x3, Return, K, real mouse click'},indent=2))
            finally:
                game.terminate()
                try: game.wait(timeout=5)
                except subprocess.TimeoutExpired:game.kill();game.wait()
                server.terminate();server.wait(timeout=5)


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args();capture(args.binary.resolve(),args.output.resolve())
