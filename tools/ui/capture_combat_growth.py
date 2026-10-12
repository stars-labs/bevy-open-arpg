#!/usr/bin/env python3
"""Real native combat input, video and telemetry in an isolated dev-tools fixture."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time


def run_case(binary, output, case):
    output.mkdir(parents=True, exist_ok=True)
    # Keep fixture saves/profiles in the evidence directory.
    asset_link = output / 'assets'
    if not asset_link.exists():
        asset_link.symlink_to(Path(__file__).resolve().parents[2] / 'assets', target_is_directory=True)
    read_fd, write_fd = os.pipe()
    with (output / 'xvfb.log').open('w') as xlog:
        server = subprocess.Popen(['Xvfb', '-displayfd', str(write_fd), '-screen', '0',
                                   '1280x720x24', '-nolisten', 'tcp'], pass_fds=(write_fd,), stdout=xlog, stderr=xlog)
        os.close(write_fd)
        with os.fdopen(read_fd) as pipe:
            display = pipe.readline().strip()
        if not display:
            raise RuntimeError(f'Xvfb could not create a display; see {output / "xvfb.log"}')
        env = dict(os.environ, DISPLAY=':' + display, WINIT_UNIX_BACKEND='x11',
                   XDG_CONFIG_HOME=str(output / 'config'), BEVY_OPEN_ARPG_COMBAT_REVIEW=case)
        env.pop('WAYLAND_DISPLAY', None)
        soulbinder = case.startswith('soulbinder-')
        sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
        env['LD_LIBRARY_PATH'] = os.pathsep.join([str(binary.parent / 'deps'), env.get('LD_LIBRARY_PATH', ''),
                                                *map(str, (sysroot / 'lib/rustlib').glob('*/lib'))])
        log_path = output / 'game.log'
        with log_path.open('w') as log:
            game = subprocess.Popen([str(binary), '--x11', '--no-audio'], env=env, cwd=output, stdout=log, stderr=log)
            video = None
            try:
                def xdo(*args, check=True):
                    return subprocess.run(['xdotool', *map(str, args)], env=env, check=check,
                                          text=True, capture_output=True)
                window = None
                for _ in range(160):
                    if game.poll() is not None:
                        raise RuntimeError(f'{case}: game exited; see {log_path}')
                    result = xdo('search', '--pid', game.pid, '--name', 'Bevy Open ARPG', check=False)
                    if result.returncode == 0 and result.stdout.strip():
                        window = result.stdout.splitlines()[0]
                        break
                    time.sleep(.25)
                if window is None:
                    raise RuntimeError('No game window')
                xdo('windowfocus', window)
                xdo('windowsize', window, 1280, 720)
                xdo('windowmove', window, 0, 0)
                time.sleep(5)
                xdo('mousemove', '--window', window, 240, 370)
                xdo('mousedown', 1)
                time.sleep(.1)
                xdo('mouseup', 1)
                time.sleep(.5)
                # Confirm the class picker, introduced after the original recorder.
                hero = 'soulbinder' if soulbinder else case.split('-')[1] if case.startswith('vfx-') else 'warden'
                for _ in range({'warden': 0, 'stalker': 1, 'embercaller': 2, 'soulbinder': 3}[hero]):
                    xdo('key', 'Right')
                    time.sleep(.15)
                xdo('keydown', 'Return')
                time.sleep(.12)
                xdo('keyup', 'Return')
                for _ in range(100):
                    match = re.search(r'COMBAT_REVIEW_READY \S+ (\d+) (\d+)', log_path.read_text())
                    if match:
                        break
                    time.sleep(.2)
                else:
                    raise RuntimeError(f'Fixture did not become ready: {case}')
                time.sleep(.5)
                xdo('mousemove', '--window', window, *map(int, match.groups()))
                def capture(name):
                    subprocess.run(['import', '-display', env['DISPLAY'], '-window', window,
                                    str(output / f'{name}.png')], check=True, timeout=20)
                capture('before')
                with (output / 'video.log').open('w') as vlog:
                    video = subprocess.Popen(['ffmpeg', '-y', '-f', 'x11grab', '-framerate', '60',
                                              '-video_size', '1280x720', '-i', env['DISPLAY'] + '+0,0',
                                              '-t', '14' if soulbinder else '2.5', '-c:v', 'libx264', '-preset', 'ultrafast',
                                              '-crf', '20', '-pix_fmt', 'yuv420p', str(output / 'contact.mp4')],
                                             stdout=vlog, stderr=vlog)
                    time.sleep(.35)
                    kind = case.split(':')[0]
                    key = 'r' if kind in ('vigor', 'expose', 'blood') else 'q' if kind in ('cleanse', 'reap') else 'e' if kind in ('ember', 'frost') else None
                    if case.startswith('vfx-'):
                        key = case.split('-')[2]
                    if soulbinder:
                        def click(button=1):
                            xdo('mousedown', button)
                            time.sleep(.07)
                            xdo('mouseup', button)
                        if case == 'soulbinder-idle':
                            pass
                        elif case == 'soulbinder-empty':
                            xdo('mousemove', '--window', window, 1100, 130)
                            click()
                        else:
                            click()
                            time.sleep(1.2)
                            if case in ('soulbinder-volley', 'soulbinder-bone', 'soulbinder-spirit', 'soulbinder-pack'):
                                xdo('key', 'e')
                            elif case == 'soulbinder-resonance':
                                click()
                                time.sleep(1.0)
                                click()
                            elif case in ('soulbinder-recall', 'soulbinder-exchange'):
                                xdo('key', 'q')
                            elif case == 'soulbinder-bulwark':
                                xdo('key', 'e')
                            elif case in ('soulbinder-overdrive', 'soulbinder-sacrifice'):
                                xdo('key', 'r')
                            elif case == 'soulbinder-pause':
                                xdo('key', 'Escape')
                                time.sleep(1.5)
                                xdo('key', 'Escape')
                    else:
                        if key:
                            xdo('keydown', key)
                            time.sleep(.09)
                            xdo('keyup', key)
                        else:
                            xdo('mousedown', 1)
                            time.sleep(.07)
                            xdo('mouseup', 1)
                        if kind == 'focus':
                            time.sleep(.55)
                            xdo('keydown', 'r')
                            time.sleep(.09)
                            xdo('keyup', 'r')
                    video.wait(timeout=20)
                    if video.returncode:
                        raise RuntimeError('Video capture failed')
                capture('after')
                text = log_path.read_text()
                if 'panicked' in text or 'ERROR' in text:
                    raise RuntimeError(f'Runtime error: {case}')
                if case.startswith('vfx-') and 'SKILL_VFX_RELEASE' not in text:
                    raise AssertionError(f'{case}: skill released no visual silhouette')
                rows = []
                for line in text.splitlines():
                    match = re.search(r'COMBAT_REVIEW_FRAME \S+ ((?:[-\d.]+\s*){9})', line)
                    if match:
                        values = list(map(float, match[1].split()))
                        rows.append(dict(zip(['time','speed','swing','x','y','z','health','fury','barrier'], values)))
                (output / 'telemetry.json').write_text(json.dumps(rows, indent=2))
                if not rows or (not soulbinder and min(row['health'] for row in rows) >= 500):
                    raise AssertionError(f'{case}: real input did not hit target')
                subprocess.run(['ffmpeg', '-y', '-ss', '0.25', '-i', str(output / 'contact.mp4'), '-t', '12' if soulbinder else '1.0',
                                '-vf', 'fps=12,scale=960:-1' if soulbinder else 'fps=30,scale=960:-1', str(output / 'frame-%03d.png')],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
                summary = dict(case=case, damage=rows[0]['health']-min(r['health'] for r in rows),
                               max_fury=max(r['fury'] for r in rows), max_barrier=max(r['barrier'] for r in rows),
                               recoil=max(abs(r['z']-rows[0]['z']) for r in rows),
                               minimum_time_speed=min(r['speed'] for r in rows))
                if soulbinder:
                    summary['soulbinder'] = True
                (output / 'result.json').write_text(json.dumps(summary, indent=2))
                print(json.dumps(summary), flush=True)
            finally:
                if video is not None and video.poll() is None:
                    video.terminate()
                    video.wait(timeout=5)
                game.terminate()
                try:
                    game.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    game.kill()
                    game.wait()
                server.terminate()
                server.wait(timeout=5)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/debug/bevy-open-arpg')
    parser.add_argument('--output', default='target/validation/combat-growth')
    parser.add_argument('--cases', nargs='+', default=['wrath:0', 'wrath:1', 'wrath:4', 'wrath:8',
                        'vigor:0', 'vigor:1', 'vigor:4', 'vigor:8', 'focus:0', 'focus:1', 'focus:4', 'focus:8',
                        'expose', 'blood', 'cleanse', 'reap', 'ember', 'frost'])
    args = parser.parse_args()
    for case in args.cases:
        run_case(Path(args.binary).resolve(), Path(args.output).resolve() / case.replace(':', '-'), case)
