#!/usr/bin/env python3
"""Real native combat input, video and telemetry in an isolated dev-tools fixture."""
import argparse
import hashlib
import shutil
import json
import os
from pathlib import Path
import re
import subprocess
import time


def run_case(binary, output, case, audio=False, audio_source=None):
    output.mkdir(parents=True, exist_ok=True)
    (output / 'binary.sha256').write_text(hashlib.sha256(binary.read_bytes()).hexdigest() + '\n')
    (output / 'audio.json').write_text(json.dumps(dict(game_audio_enabled=audio, recorded=False, reason='No loopback source configured; video has no audio track.')))
    for tool, pattern in [('Xvfb', '*xvfb*/bin/Xvfb'), ('xdotool', '*xdotool*/bin/xdotool')]:
        if not shutil.which(tool):
            matches = list(Path('/nix/store').glob(pattern))
            if matches:
                os.environ['PATH'] = str(matches[0].parent) + os.pathsep + os.environ['PATH']
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
                   XDG_CONFIG_HOME=str(output / 'config'), BEVY_OPEN_ARPG_COMBAT_REVIEW=case, RUST_LOG='info',
                   BEVY_ASSET_ROOT=str(Path(__file__).resolve().parents[2]))
        env.pop('WAYLAND_DISPLAY', None)
        soulbinder = (case.startswith('soulbinder-') or case in ('feel-summon-isolation', 'pixelfx-summon'))
        sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
        env['LD_LIBRARY_PATH'] = os.pathsep.join([str(binary.parent / 'deps'), env.get('LD_LIBRARY_PATH', ''),
                                                *map(str, (sysroot / 'lib/rustlib').glob('*/lib'))])
        log_path = output / 'game.log'
        with log_path.open('w') as log:
            game = subprocess.Popen([str(binary), '--x11'] + ([] if audio else ['--no-audio']), env=env, cwd=output, stdout=log, stderr=log)
            video = None
            input_events = []
            capture_started = time.monotonic()
            try:
                def xdo(*args, check=True):
                    if args[0] in ('key', 'keydown', 'keyup', 'mousedown', 'mouseup'):
                        frames = re.findall(r'COMBAT_FEEL_FRAME \S+ ([\d.]+)', log_path.read_text())
                        input_events.append(dict(action=list(map(str, args)),
                                                 seconds_since_launch=time.monotonic() - capture_started,
                                                 preceding_game_frame_time=float(frames[-1]) if frames else None))
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
                time.sleep(18)
                xdo('mousemove', '--window', window, 240, 370)
                xdo('mousedown', 1)
                time.sleep(.1)
                xdo('mouseup', 1)
                time.sleep(.5)
                # Confirm the class picker, introduced after the original recorder.
                hero = 'soulbinder' if soulbinder else 'embercaller' if case in ('feel-ranged-hit', 'pixelfx-fireball', 'pixelfx-frostnova') else case.split('-')[1] if case.startswith('vfx-') else 'warden'
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
                    subprocess.run(['import', '-display', env['DISPLAY'], '-window', window, str(output / 'not-ready.png')], check=False)
                    raise RuntimeError(f'Fixture did not become ready: {case}')
                time.sleep(.5)
                aim = (900, 400) if case == 'pixelfx-heavy-empty' else tuple(map(int, match.groups()))
                xdo('mousemove', '--window', window, *aim)
                def capture(name):
                    subprocess.run(['import', '-display', env['DISPLAY'], '-window', window,
                                    str(output / f'{name}.png')], check=True, timeout=20)
                def audit_audio_route(stage):
                    sink_name = env.get('PULSE_SINK')
                    if not audio or not sink_name:
                        return
                    pactl = shutil.which('pactl')
                    if not pactl:
                        pactl = next(map(str, Path('/nix/store').glob('*pulseaudio*/bin/pactl')), None)
                    if not pactl:
                        raise RuntimeError('PULSE_SINK isolation requested but pactl unavailable')
                    def listing(kind):
                        return json.loads(subprocess.check_output([pactl, '--format=json', 'list', kind], env=env, text=True))
                    sinks = listing('sinks')
                    sink = next(item for item in sinks if item['name'] == sink_name)
                    inputs = listing('sink-inputs')
                    own = [item for item in inputs if str(item.get('properties', {}).get('application.process.id')) == str(game.pid)]
                    if not own:
                        raise RuntimeError(f'No audio sink-input belonging to game PID {game.pid}')
                    for item in own:
                        if item['sink'] != sink['index']:
                            subprocess.run([pactl, 'move-sink-input', str(item['index']), sink_name], env=env, check=True)
                    inputs = listing('sink-inputs')
                    routed = [item for item in inputs if item['sink'] == sink['index']]
                    if any(str(item.get('properties', {}).get('application.process.id')) != str(game.pid) for item in routed):
                        raise RuntimeError('Isolated audio sink contains another process')
                    evidence = dict(stage=stage, sink_name=sink_name, sink_index=sink['index'], game_pid=game.pid,
                                    game_inputs=[dict(index=item['index'], sink=item['sink'],
                                                      process_id=item.get('properties', {}).get('application.process.id')) for item in routed],
                                    foreign_stream_count=0)
                    routing_path = output / 'audio-routing.json'
                    history = json.loads(routing_path.read_text()) if routing_path.exists() else []
                    history.append(evidence)
                    routing_path.write_text(json.dumps(history, indent=2))
                audit_audio_route('before')
                capture('before')
                with (output / 'video.log').open('w') as vlog:
                    progress_path = output / 'video-progress.txt'
                    progress_path.unlink(missing_ok=True)
                    video = subprocess.Popen(['ffmpeg', '-y', '-progress', str(progress_path),
                                              '-stats_period', '0.1', '-f', 'x11grab', '-framerate', '60',
                                              '-video_size', '1280x720', '-i', env['DISPLAY'] + '+0,0',
                                              *(['-f', 'pulse', '-i', audio_source] if audio and audio_source else []),
                                              '-t', '10' if case == 'pixelfx-summon' else '7', *(['-c:a', 'aac'] if audio and audio_source else []), '-c:v', 'libx264', '-preset', 'ultrafast',
                                              '-crf', '20', '-pix_fmt', 'yuv420p', str(output / 'contact.mp4')],
                                             stdout=vlog, stderr=vlog)
                    for _ in range(100):
                        if video.poll() is not None:
                            raise RuntimeError('Video capture ended before its first encoded frame')
                        if progress_path.exists():
                            encoded = re.findall(r'^frame=(\d+)$', progress_path.read_text(), re.MULTILINE)
                            if encoded and int(encoded[-1]) >= 1:
                                break
                        time.sleep(.1)
                    else:
                        raise RuntimeError('Video capture produced no encoded frame')
                    (output / 'video-ready.json').write_text(json.dumps(dict(
                        encoded_frames=int(encoded[-1]),
                        seconds_since_launch=time.monotonic() - capture_started)))
                    time.sleep(.3)
                    def click(button=1):
                        xdo('mousedown', button)
                        time.sleep(.06)
                        xdo('mouseup', button)
                    if case == 'feel-empty':
                        xdo('mousemove', '--window', window, 900, 400)
                        click()
                        for key in ('e', 'q', 'r'):
                            time.sleep(.6)
                            xdo('key', key)
                    elif case == 'pixelfx-heavy-whiff':
                        xdo('keydown', 's')
                        time.sleep(.2)
                        xdo('key', 'r')
                        time.sleep(.45)
                        xdo('keyup', 's')
                    elif case in ('feel-heavy', 'feel-multi', 'pixelfx-heavy', 'pixelfx-heavy-empty', 'pixelfx-summon'):
                        xdo('key', 'r')
                    elif case == 'pixelfx-frostnova':
                        xdo('key', 'e')
                    elif case == 'feel-summon-isolation':
                        click()
                        time.sleep(.8)
                        xdo('key', 'r')
                        time.sleep(.2)
                        xdo('keydown', 'd')
                        time.sleep(.7)
                        xdo('keyup', 'd')
                    elif case == 'feel-evade-cancel':
                        click()
                        time.sleep(.025)
                        xdo('key', 'Shift_L')
                    elif case == 'feel-recovery-chain':
                        click()
                        time.sleep(.2)
                        click(3)
                        time.sleep(.25)
                        xdo('key', 'e')
                    else:
                        click()
                    video.wait(timeout=20)
                    if video.returncode:
                        raise RuntimeError('Video capture failed')
                audit_audio_route('after')
                if audio and audio_source:
                    levels = subprocess.run(['ffmpeg', '-hide_banner', '-i', str(output / 'contact.mp4'),
                                             '-map', '0:a:0', '-af', 'astats=metadata=1:reset=0', '-f', 'null', '-'],
                                            text=True, capture_output=True, check=True).stderr
                    (output / 'audio-levels.txt').write_text(levels)
                    peaks = re.findall(r'Peak level dB: ([-\d.inf]+)', levels)
                    rms = re.findall(r'RMS level dB: ([-\d.inf]+)', levels)
                    (output / 'audio.json').write_text(json.dumps(dict(game_audio_enabled=True, recorded=True,
                        source=audio_source, auditioned=False, decoded_pcm_peak_db=peaks[-1] if peaks else None,
                        decoded_pcm_rms_db=rms[-1] if rms else None)))
                capture('after')
                text = log_path.read_text()
                if 'panicked' in text or 'ERROR' in text:
                    raise RuntimeError(f'Runtime error: {case}')
                rows = []
                for line in text.splitlines():
                    match = re.search(r'COMBAT_REVIEW_FRAME \S+ ((?:[-\d.]+\s*){9})', line)
                    if match:
                        values = list(map(float, match[1].split()))
                        rows.append(dict(zip(['time','speed','swing','x','y','z','health','fury','barrier'], values)))
                feel_rows = []
                fields = ['time','speed','swing','hero_x','hero_y','hero_z','hero_health','target_health','resource','hero_scale','target_scale','camera_x','camera_y','camera_z']
                for line in text.splitlines():
                    match = re.search(r'COMBAT_FEEL_FRAME \S+ (.*)', line)
                    if match:
                        feel_rows.append(dict(zip(fields, map(float, match[1].split()))))
                minion_rows = []
                for line in text.splitlines():
                    match = re.search(r'COMBAT_FEEL_MINION \S+ ([\d.]+) (\S+) (.*)', line)
                    if match:
                        row = dict(time=float(match[1]), entity=match[2])
                        row.update(zip(['x', 'z', 'health', 'animation_scale'], map(float, match[3].split())))
                        minion_rows.append(row)
                feedback_events = []
                for line in text.splitlines():
                    match = re.search(r'COMBAT_FEEL_(CONTACT|SHAKE) \S+ ([\d.]+) (.*)', line)
                    if match:
                        feedback_events.append(dict(kind=match[1], time=float(match[2]), details=match[3]))
                (output / 'feedback-events.json').write_text(json.dumps(feedback_events, indent=2))
                (output / 'minion-telemetry.json').write_text(json.dumps(minion_rows, indent=2))
                (output / 'feel-telemetry.json').write_text(json.dumps(feel_rows, indent=2))
                (output / 'telemetry.json').write_text(json.dumps(rows, indent=2))
                if not rows:
                    raise AssertionError(f'{case}: fixture emitted no telemetry')
                subprocess.run(['ffmpeg', '-y', '-ss', '0.25', '-i', str(output / 'contact.mp4'), '-t', '6',
                                '-vf', 'fps=12,scale=960:-1', str(output / 'frame-%03d.png')],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
                summary = dict(case=case, damage=rows[0]['health']-min(r['health'] for r in rows),
                               max_fury=max(r['fury'] for r in rows), max_barrier=max(r['barrier'] for r in rows),
                               recoil=max(abs(r['z']-rows[0]['z']) for r in rows),
                               minimum_time_speed=min(r['speed'] for r in rows))
                contacts = []
                for previous, current in zip(feel_rows, feel_rows[1:]):
                    if current['target_health'] < previous['target_health']:
                        contacts.append(dict(time=current['time'], damage=previous['target_health']-current['target_health']))
                summary['contacts'] = contacts
                summary['feedback_events'] = feedback_events
                summary['feel_frames'] = len(feel_rows)
                summary['global_speed_always_one'] = all(abs(row['speed'] - 1) < .0001 for row in feel_rows) if feel_rows else None
                summary['minimum_hero_animation_scale'] = min((r['hero_scale'] for r in feel_rows), default=None)
                summary['minimum_target_animation_scale'] = min((r['target_scale'] for r in feel_rows), default=None)
                if soulbinder:
                    summary['soulbinder'] = True
                (output / 'result.json').write_text(json.dumps(summary, indent=2))
                print(json.dumps(summary), flush=True)
            finally:
                (output / 'input-events.json').write_text(json.dumps(input_events, indent=2))
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
    parser.add_argument('--output', default='target/validation/combat-feel/after')
    parser.add_argument('--audio', action='store_true', help='Enable game audio; recording remains silent without configured loopback.')
    parser.add_argument('--audio-source', help='PulseAudio loopback source, e.g. a sink monitor; captures game/system audio.')
    parser.add_argument('--cases', nargs='+', default=['empty', 'basic-hit', 'critical', 'kill', 'heavy', 'multi', 'summon-isolation', 'evade-cancel', 'recovery-chain', 'edge-hit', 'ranged-hit'])
    args = parser.parse_args()
    for case in args.cases:
        case = case if case.startswith('feel-') else 'feel-' + case
        run_case(Path(args.binary).resolve(), Path(args.output).resolve() / case, case, args.audio, args.audio_source)
