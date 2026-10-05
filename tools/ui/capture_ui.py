#!/usr/bin/env python3
"""Capture the real native UI on an isolated X11 display (Xvfb, xdotool, ImageMagick)."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import time
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/debug/bevy-open-arpg')
    parser.add_argument('--output', default='target/validation')
    review = parser.add_mutually_exclusive_group()
    review.add_argument('--motion-review', action='store_true',
                        help='Capture rear/front/side walks, evade and attack frames')
    review.add_argument('--hit-review', action='store_true',
                        help='Walk to the first Ashbone Guard and capture incoming hits without attacking')
    review.add_argument('--combat-review', action='store_true',
                        help='Check language switching, empty R cast, enemy hover and paid R attack')
    review.add_argument('--panels-review', action='store_true')
    review.add_argument('--progression-review', action='store_true',
                        help='Dev-tools fixture: real XP level-up pipeline and skill/talent interaction')
    review.add_argument('--hud-review', action='store_true')
    review.add_argument('--swing-review', action='store_true')
    review.add_argument('--controls-review', action='store_true')
    review.add_argument('--settings-review', action='store_true')
    review.add_argument('--defeat-review', action='store_true')
    review.add_argument('--death-vfx-review', action='store_true',
                        help='Capture a real kill sequence, fallen models and dropped pickups')
    args = parser.parse_args()
    for command in ('Xvfb', 'xdotool', 'import'):
        if not shutil.which(command):
            raise SystemExit(f'Missing {command}; add Xvfb, xdotool and ImageMagick to PATH.')
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    read_fd, write_fd = os.pipe()
    with (output / 'capture-xvfb.log').open('w') as xlog:
        server = subprocess.Popen(['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '2560x1440x24', '-nolisten', 'tcp'], pass_fds=(write_fd,), stdout=xlog, stderr=xlog)
        os.close(write_fd)
        try:
            with os.fdopen(read_fd) as display_pipe:
                display = display_pipe.readline().strip()
            if not display:
                raise RuntimeError('Xvfb failed to create a display')
            env = dict(os.environ, DISPLAY=f':{display}', WINIT_UNIX_BACKEND='x11')
            env['XDG_CONFIG_HOME'] = str(output / 'config')
            if args.panels_review or args.progression_review:
                (output / 'config/org.stars-labs.bevy-open-arpg/preferences.toml').unlink(missing_ok=True)
            if args.progression_review:
                env['BEVY_OPEN_ARPG_REVIEW_LEVEL_UP'] = '1'
            env.pop('WAYLAND_DISPLAY', None)
            binary = Path(args.binary).resolve()
            # All-feature builds enable Bevy's dynamic linking, normally set up by Cargo.
            library_paths = [str(binary.parent / 'deps'), env.get('LD_LIBRARY_PATH', '')]
            sysroot = subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip()
            library_paths.extend(str(path) for path in (Path(sysroot) / 'lib' / 'rustlib').glob('*/lib'))
            env['LD_LIBRARY_PATH'] = os.pathsep.join(path for path in library_paths if path)
            with (output / 'capture-game.log').open('w') as game_log:
                game = subprocess.Popen([str(binary), '--x11', '--no-audio'], env=env, stdout=game_log, stderr=game_log)
                try:
                    def xdo(*values, check=True):
                        return subprocess.run(['xdotool', *map(str, values)], env=env, check=check, text=True, capture_output=True)

                    window = None
                    for _ in range(120):
                        if game.poll() is not None:
                            raise RuntimeError('Game exited during startup; see capture-game.log')
                        result = xdo('search', '--pid', game.pid, '--name', 'Bevy Open ARPG', check=False)
                        if result.returncode == 0 and result.stdout.strip():
                            window = result.stdout.splitlines()[0]
                            break
                        time.sleep(0.25)
                    if window is None:
                        raise RuntimeError('Game window was not created within 30 seconds')
                    xdo('windowfocus', window)

                    def capture(name, wait=1.0):
                        time.sleep(wait)
                        if game.poll() is not None:
                            raise RuntimeError(f'Game exited before {name}; see capture-game.log')
                        subprocess.run(['import', '-display', env['DISPLAY'], '-window', window, '-define', 'png:compression-level=1', str(output / f'{name}.png')], check=True, timeout=15)
                        print(f'Captured {name}', flush=True)

                    capture('menu-desktop', 5)
                    if args.combat_review or args.settings_review:
                        xdo('key', 'F8')
                        capture('menu-no-language-hotkey')
                        xdo('mousemove', '--window', window, 1240, 40)
                        xdo('click', 1)
                        capture('settings-chinese')
                        xdo('mousemove', '--window', window, 640, 380)
                        xdo('click', 1)
                        capture('settings-english')
                        preferences = Path(env.get('XDG_CONFIG_HOME', str(Path.home() / '.config'))) / 'org.stars-labs.bevy-open-arpg/preferences.toml'
                        assert tomllib.loads(preferences.read_text())['runtime']['language'] == 'en'
                        xdo('mousemove', '--window', window, 640, 305)
                        xdo('click', 1)
                        capture('settings-chinese-restored')
                        assert tomllib.loads(preferences.read_text())['runtime']['language'] == 'zh-CN'
                        xdo('key', 'Escape')
                        capture('menu-chinese-restored')
                    xdo('windowsize', window, 390, 844)
                    capture('menu-narrow')
                    xdo('windowsize', window, 1280, 720)
                    time.sleep(0.5)
                    xdo('mousemove', '--window', window, 240, 505)
                    capture('menu-details')
                    xdo('mousemove', '--window', window, 240, 370)
                    capture('menu-hover')
                    xdo('click', 1)
                    # Starting a new run now opens the class picker first.
                    time.sleep(0.5)
                    xdo('keydown', 'Return')
                    time.sleep(0.15)
                    xdo('keyup', 'Return')
                    capture('gameplay', 8)
                    xdo('windowsize', window, 1920, 1080)
                    capture('gameplay-wide')
                    if args.progression_review:
                        def press(key):
                            xdo('keydown', key)
                            time.sleep(.12)
                            xdo('keyup', key)
                            time.sleep(.25)
                        def click(x, y):
                            xdo('mousemove', '--window', window, x, y)
                            xdo('mousedown', 1)
                            time.sleep(.15)
                            xdo('mouseup', 1)
                        capture('level-up-rewards', 1.1)
                        capture('unspent-point', 4)
                        click(960, 855)
                        capture('talents-from-reminder')
                        # Desktop talent rank positions are verified from the captured layout.
                        xdo('windowsize', window, 1280, 720)
                        capture('talents-desktop')
                        click(630, 412)
                        capture('talent-point-spent')
                        press('k')
                        capture('reminder-cleared')
                        press('k')
                        click(295, 121)
                        capture('skills-desktop')
                        click(800, 293)
                        capture('skill-rune-selected')
                        xdo('mousemove', '--window', window, 850, 450)
                        xdo('click', '--repeat', 8, '--delay', 50, 5)
                        capture('skills-scrolled')
                        xdo('click', '--repeat', 16, '--delay', 50, 4)
                        xdo('windowsize', window, 600, 900)
                        capture('skills-narrow')
                        xdo('mousemove', '--window', window, 350, 430)
                        xdo('click', '--repeat', 10, '--delay', 50, 5)
                        capture('skills-narrow-scrolled')
                        xdo('click', '--repeat', 20, '--delay', 50, 4)
                        xdo('windowsize', window, 1280, 720)
                        press('k')
                        press('Escape')
                        click(40, 40)
                        time.sleep(.5)
                        click(640, 395)
                        press('Escape')
                        press('Escape')
                        press('k')
                        capture('skills-english')
                        return
                    if args.panels_review:
                        def press_panel(key):
                            xdo('keydown', key)
                            time.sleep(.12)
                            xdo('keyup', key)
                            time.sleep(.3)
                        for width, height, label in [(1920, 1080, 'wide'), (1280, 720, 'desktop'), (600, 900, 'narrow')]:
                            xdo('windowsize', window, width, height)
                            press_panel('i')
                            capture(f'inventory-{label}')
                            if label == 'desktop':
                                xdo('mousemove', '--window', window, 965, 77)
                                xdo('click', 1)
                                capture('inventory-close-button')
                                press_panel('i')
                            press_panel('i')
                            press_panel('k')
                            panel_left = (width - min(780, width * .92)) / 2
                            xdo('mousemove', '--window', window, int(panel_left + 155), 121)
                            xdo('click', 1)
                            capture(f'talents-{label}')
                            if label == 'desktop':
                                xdo('mousemove', '--window', window, 970, 68)
                                xdo('click', 1)
                                capture('talents-close-button')
                                press_panel('k')
                            if label == 'narrow':
                                xdo('mousemove', '--window', window, 300, 400)
                                xdo('click', '--repeat', 12, '--delay', 50, 5)
                                capture('talents-narrow-scrolled')
                                xdo('click', '--repeat', 12, '--delay', 50, 4)
                            press_panel('k')
                        xdo('windowsize', window, 1280, 720)
                        for key, direction in [('w', 'north'), ('d', 'east'), ('s', 'south'), ('a', 'west')]:
                            xdo('keydown', key)
                            time.sleep(.45)
                            xdo('keyup', key)
                            capture(f'map-{direction}', .3)
                        press_panel('Escape')
                        xdo('mousemove', '--window', window, 40, 40)
                        xdo('click', 1)
                        capture('settings-chinese')
                        xdo('mousemove', '--window', window, 640, 395)
                        xdo('click', 1)
                        capture('settings-english')
                        press_panel('Escape')
                        press_panel('Escape')
                        press_panel('i')
                        capture('inventory-english')
                        press_panel('i')
                        press_panel('k')
                        capture('talents-english')
                        return
                    if args.hud_review:
                        xdo('key', 'j')
                        capture('journal-wide')
                        xdo('key', 'j')
                        xdo('windowsize', window, 1280, 720)
                        capture('hud-desktop')
                        xdo('mousemove', '--window', window, 394, 644)
                        capture('skill-tooltip')
                        xdo('mousemove', '--window', window, 562, 574)
                        capture('potion-tooltip')
                        xdo('mousemove', '--window', window, 1180, 80)
                        capture('map-legend')
                        xdo('mousemove', '--window', window, 500, 400)
                        xdo('key', 'j')
                        capture('journal-desktop')
                        xdo('windowsize', window, 1280, 540)
                        capture('journal-short-before-scroll')
                        xdo('mousemove', '--window', window, 1100, 250)
                        xdo('click', '--repeat', 8, '--delay', 50, 5)
                        capture('journal-scrolled')
                        xdo('windowsize', window, 1280, 720)
                        time.sleep(.5)
                        xdo('mousemove', '--window', window, 1200, 120)
                        xdo('click', 1)
                        capture('journal-close-button')
                        xdo('windowsize', window, 600, 900)
                        capture('hud-narrow')
                        xdo('key', 'j')
                        capture('journal-narrow')
                        xdo('key', 'j')
                        xdo('windowsize', window, 1280, 720)
                        xdo('key', 'Escape')
                        capture('hud-paused')
                        xdo('mousemove', '--window', window, 40, 40)
                        xdo('click', 1)
                        capture('hud-settings')
                        xdo('key', 'Escape')
                        xdo('key', 'Escape')
                        return
                    if args.swing_review:
                        for view, point in [('side', (1300, 530)), ('front', (960, 780)), ('rear', (960, 330))]:
                            xdo('mousemove', '--window', window, *point)
                            xdo('mousedown', 1)
                            try:
                                for frame in range(10):
                                    capture(f'swing-{view}-{frame:02d}', .045)
                            finally:
                                xdo('mouseup', 1)
                            capture(f'swing-{view}-rest', .6)
                        return
                    if args.controls_review:
                        xdo('mousemove', '--window', window, 1200, 500)
                        capture('controls-before')
                        xdo('click', 3)
                        capture('right-move', 2)
                        xdo('mousemove', '--window', window, 1050, 550)
                        xdo('mousedown', 1)
                        try:
                            for frame in range(6):
                                capture(f'left-strike-{frame:02d}', .06)
                        finally:
                            xdo('mouseup', 1)
                        capture('left-stationary', 1)
                        xdo('key', 'r')
                        capture('r-empty-cast')
                        return
                    if args.death_vfx_review:
                        xdo('keydown', 'w', 'a')
                        time.sleep(4.6)
                        xdo('keyup', 'w', 'a')
                        xdo('mousemove', '--window', window, 910, 510)
                        xdo('mousedown', 1)
                        try:
                            for frame in range(24):
                                if frame == 6:
                                    xdo('mouseup', 1)
                                capture(f'death-vfx-{frame:02d}', .06)
                        finally:
                            xdo('mouseup', 1)
                        capture('remains-cleanup', 5)
                        return
                    if args.settings_review:
                        xdo('windowsize', window, 1280, 720)
                        xdo('key', 'Escape')
                        capture('paused')
                        xdo('mousemove', '--window', window, 40, 40)
                        xdo('click', 1)
                        capture('pause-settings')
                        xdo('key', 'Escape')
                        capture('pause-settings-closed')
                        xdo('key', 'Escape')
                        capture('resumed')
                        return
                    if args.defeat_review:
                        xdo('keydown', 'w', 'a')
                        try:
                            time.sleep(4.6)
                        finally:
                            xdo('keyup', 'w', 'a')
                        xdo('mousemove', '--window', window, 910, 510)
                        xdo('mousedown', 1)
                        try:
                            for frame in range(10):
                                capture(f'basic-hit-{frame:02d}', .05)
                        finally:
                            xdo('mouseup', 1)
                        capture('defeat-desktop', 18)
                        xdo('windowsize', window, 390, 844)
                        capture('defeat-narrow')
                        xdo('mousemove', '--window', window, 195, 550)
                        xdo('click', 1)
                        capture('retry-click', 2)
                        xdo('windowsize', window, 1920, 1080)
                        xdo('keydown', 'w', 'a')
                        time.sleep(4.6)
                        xdo('keyup', 'w', 'a')
                        capture('second-defeat', 35)
                        xdo('mousemove', '--window', window, 960, 727)
                        xdo('click', 1)
                        capture('return-menu-click', 2)
                        return
                    if args.combat_review:
                        xdo('mousemove', '--window', window, 1100, 600)
                        capture('rupture-empty-before', .1)
                        xdo('key', 'r')
                        capture('rupture-empty-after', .15)
                        xdo('keydown', 'w', 'a')
                        try:
                            time.sleep(4.6)
                        finally:
                            xdo('keyup', 'w', 'a')
                        xdo('mousemove', '--window', window, 910, 510)
                        capture('enemy-hover', .15)
                        xdo('key', 'r')
                        for frame in range(8):
                            capture(f'rupture-hit-{frame:02d}', .045)
                        log = (output / 'capture-game.log').read_text()
                        if 'panicked' in log or 'ERROR' in log:
                            raise RuntimeError('Runtime errors detected; see capture-game.log')
                        return
                    if args.hit_review:
                        # Fresh Chapter I: spawn (0, 12.8), guard approach (-19.2, -8).
                        # Approach diagonally, then let the real enemy AI attack.
                        # No damage injection, healing, attacks or evade invulnerability.
                        capture('health-hit-before', .1)
                        xdo('keydown', 'w', 'a')
                        try:
                            time.sleep(4.6)
                        finally:
                            xdo('keyup', 'w', 'a')
                        capture('health-hit-approach', .1)
                        for frame in range(12):
                            capture(f'health-hit-{frame:02d}', .12)
                        log = (output / 'capture-game.log').read_text()
                        if 'panicked' in log or 'ERROR' in log:
                            raise RuntimeError('Runtime errors detected; see capture-game.log')
                        return
                    if args.motion_review:
                        for key, view in [('w', 'walk'), ('s', 'walk-front'), ('a', 'walk-side')]:
                            xdo('keydown', key)
                            try:
                                for frame in range(8):
                                    capture(f'{view}-{frame:02d}', .09)
                            finally:
                                xdo('keyup', key)
                        xdo('keydown', 'd', 'Shift_L')
                        try:
                            for frame in range(4):
                                capture(f'evade-{frame:02d}', .04)
                        finally:
                            xdo('keyup', 'd', 'Shift_L')
                        xdo('mousemove', '--window', window, 1080, 500)
                        xdo('key', 'r')
                        for frame in range(8):
                            capture(f'attack-{frame:02d}', .06)
                    xdo('windowsize', window, 1280, 720)
                    xdo('keydown', 'w')
                    try:
                        capture('walking', 1.2)
                    finally:
                        xdo('keyup', 'w')
                    xdo('key', 'Escape')
                    xdo('mousemove', '--window', window, 1134, 440)
                    xdo('click', 1)
                    capture('inventory')
                    xdo('key', 'i')
                    xdo('mousemove', '--window', window, 1181, 440)
                    xdo('click', 1)
                    capture('talents')
                    xdo('key', 'k')
                    xdo('mousemove', '--window', window, 1228, 440)
                    xdo('click', 1)
                    capture('journal')
                    log = (output / 'capture-game.log').read_text()
                    if 'panicked' in log or 'ERROR' in log:
                        raise RuntimeError('Runtime errors detected; see capture-game.log')
                finally:
                    game.terminate()
                    try:
                        game.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        game.kill()
                        game.wait()
        finally:
            server.terminate()
            server.wait(timeout=10)


if __name__ == '__main__':
    main()
