#!/usr/bin/env python3
"""Record PixelFX through normal class attacks and commands in the native game."""
import argparse
import hashlib
import json
from pathlib import Path
import re

from capture_combat_feel import run_case

CASES = ('slash', 'heavy', 'fireball', 'frostnova', 'summon', 'heavy-empty', 'heavy-whiff')
EXPECTED_EFFECT = {
    'slash': 'slash',
    'heavy': 'heavy_impact',
    'fireball': 'fireball',
    'frostnova': 'frost_nova',
    'summon': 'summon',
}
EXPECTED_CLASS = {
    'slash': 'Warden',
    'heavy': 'Warden',
    'heavy-whiff': 'Warden',
    'heavy-empty': 'Warden',
    'fireball': 'Embercaller',
    'frostnova': 'Embercaller',
    'summon': 'Soulbinder',
}


def validate(output, case):
    text = (output / 'game.log').read_text()
    result = json.loads((output / 'result.json').read_text())
    effect_names = re.findall(r'PIXELFX_SPAWN (\w+)', text)
    loadout = re.search(
        r'PIXELFX_REVIEW_LOADOUT \S+ class=(\w+) level=(\d+) '
        r'area_alt=(\w+) finisher_alt=(\w+)', text)
    assert loadout, f'{case}: fixture did not report its shipping loadout'
    assert loadout[1] == EXPECTED_CLASS[case], (case, loadout.groups())
    assert int(loadout[2]) >= 6, f'{case}: skills remain level-locked'
    assert result['global_speed_always_one'], f'{case}: effect slowed the world clock'
    if case == 'heavy-empty':
        telemetry = json.loads((output / 'feel-telemetry.json').read_text())
        assert all(frame['resource'] == 100 for frame in telemetry), 'target-required skill spent resource without a target'
        assert all(frame['swing'] == 1 for frame in telemetry), 'target-required skill entered an attack without a target'
        assert not effect_names, 'target-required skill emitted a visual without a target'
        assert result['damage'] == 0 and not result['feedback_events'], result
        result['rejected_by_target_requirement'] = True
    elif case == 'heavy-whiff':
        telemetry = json.loads((output / 'feel-telemetry.json').read_text())
        assert min(frame['resource'] for frame in telemetry) < 100, 'retreating heavy attack never paid its cost'
        assert min(frame['swing'] for frame in telemetry) < 1, 'retreating heavy attack never entered its animation'
        assert 'slash' in effect_names, 'retreating heavy attack never released its weapon trail'
        assert result['damage'] == 0, result
        assert 'heavy_impact' not in effect_names, 'retreating heavy attack fabricated an impact'
        assert not result['feedback_events'], 'retreating heavy attack fabricated contact feedback'
    else:
        expected = EXPECTED_EFFECT[case]
        assert expected in effect_names, f'{case}: no {expected} visual was spawned'
        if case == 'summon':
            assert 'SUMMON_ORDER Sacrifice' in text, 'normal R command was not accepted'
            assert 'SUMMON_REVIVE' in text, 'sacrificed spirit did not reform'
            assert effect_names.count('summon') >= 2, 'initial spirit appeared but rebirth did not'
        else:
            assert result['damage'] > 0, f'{case}: normal input never damaged its target'
    assert (output / 'contact.mp4').stat().st_size > 1000
    result['pixelfx_effects'] = effect_names
    result['review_class'] = loadout[1]
    result['review_level'] = int(loadout[2])
    result['verified'] = True
    (output / 'result.json').write_text(json.dumps(result, indent=2))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/debug/bevy-open-arpg')
    parser.add_argument('--output', default='target/validation/pixelfx')
    parser.add_argument('--cases', nargs='+', choices=CASES, default=CASES)
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    results = {}
    for path in output.glob('*/result.json'):
        previous = json.loads(path.read_text())
        if previous.get('verified'):
            results[previous['case']] = previous
    for case in args.cases:
        case_output = output / case
        case_output.mkdir(parents=True, exist_ok=True)
        asset_root = Path(__file__).resolve().parents[2] / 'assets/vfx/pixelfx'
        asset_hashes = {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                        for path in sorted(asset_root.iterdir())
                        if path.suffix == '.png' or path.name == 'manifest.ron'}
        (case_output / 'asset-sha256.json').write_text(json.dumps(asset_hashes, indent=2))
        run_case(binary, case_output, 'pixelfx-' + case)
        result = validate(case_output, case)
        results[result['case']] = result
        (output / 'results.json').write_text(json.dumps(list(results.values()), indent=2))
        print(json.dumps(result), flush=True)
