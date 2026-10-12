#!/usr/bin/env python3
"""Capture shipping Soulbinder commands and survival using real native input."""
import argparse
import json
from pathlib import Path
import re
from capture_combat_growth import run_case

CASES = ['idle', 'empty', 'mark', 'volley', 'bone', 'spirit', 'recall',
         'exchange', 'bulwark', 'overdrive', 'sacrifice', 'pause',
         'melee', 'projectile', 'boss', 'pack', 'resonance']


def validate(output, case):
    text = (output / 'game.log').read_text()
    frames = []
    for m in re.finditer(r'SOULBINDER_MINION \S+ ([\d.]+) (\S+) ([-\d.]+) ([-\d.]+) ([\d.]+) ([\d.]+) ([\d.]+)', text):
        frames.append(dict(time=float(m[1]), entity=m[2], x=float(m[3]), z=float(m[4]),
                           health=float(m[5]), maximum=float(m[6]), dead_seconds=float(m[7])))
    hits = [dict(target=m[1], heavy=m[2] == 'true', amount=float(m[3]), slot=int(m[4]))
            for m in re.finditer(r'SUMMON_HIT target=(\S+) heavy=(true|false) amount=([\d.]+) slot=(\d+)', text)]
    orders = re.findall(r'SUMMON_ORDER (\w+) bone=(\d+) spirit=(\d+)', text)
    assert frames, 'shipping class spawned no minion'
    assert (output / 'contact.mp4').stat().st_size > 1000
    if case in ('idle', 'empty'):
        assert not hits and not orders, (case, hits, orders)
    elif case not in ('recall', 'bulwark'):
        assert hits, f'{case}: marked prey never received spirit attack'
    expected = dict(volley='Volley', bone='Volley', spirit='Volley', recall='Recall',
                    exchange='Exchange', bulwark='Bulwark', overdrive='Overdrive', sacrifice='Sacrifice', pack='Volley')
    if case in expected:
        assert any(order[0] == expected[case] for order in orders), (case, orders)
    by_time = {}
    for frame in frames:
        by_time.setdefault(frame['time'], []).append(frame)
    count = max(map(len, by_time.values()))
    if case == 'bone':
        assert count == 2, count
        assert {hit['slot'] for hit in hits} == {0, 1}, hits
    else:
        assert count == 1, count
    if case in ('volley', 'bone', 'spirit', 'overdrive', 'pack', 'resonance'):
        assert any(hit['heavy'] for hit in hits), hits
    if case == 'sacrifice':
        assert any(frame['health'] == 0 and frame['dead_seconds'] > 0 for frame in frames)
        assert 'SUMMON_REVIVE' in text, 'sacrificed spirit did not reform'
    if case in ('melee', 'projectile', 'boss', 'pack'):
        assert any(frame['health'] < frame['maximum'] for frame in frames), 'active enemy never damaged summon'
    if case == 'resonance':
        assert 'SUMMON_RESONANCE basics=3' in text, 'third unique basic contact was not observed'
        assert not orders, 'resonance must activate through basic contacts, without paid commands'
        assert any(hit['heavy'] for hit in hits), 'three genuine basic contacts did not trigger resonance'
    if case == 'pack':
        enemy_frames = [(float(m[1]), m[2], float(m[3])) for m in re.finditer(
            r'SOULBINDER_ENEMY \S+ ([\d.]+) (\S+) \S+ ([\d.]+)', text)]
        # Keep the three actual initial actors separate from later natural world spawns.
        initial_ids = {entity for time, entity, _ in enemy_frames if time == enemy_frames[0][0]}
        assert len(initial_ids) == 3, initial_ids
        enemy_rows = {entity: [] for entity in initial_ids}
        for _, entity, health in enemy_frames:
            if entity in enemy_rows:
                enemy_rows[entity].append(health)
        assert sum(min(values) < max(values) for values in enemy_rows.values()) >= 2, 'volley never hit surrounding pack'
    heroes = [dict(time=float(m[1]), x=float(m[2]), z=float(m[3]), health=float(m[4]))
              for m in re.finditer(r'SOULBINDER_HERO \S+ ([\d.]+) ([-\d.]+) ([-\d.]+) ([\d.]+)', text)]
    if case == 'recall':
        assert heroes and max(f['z'] for f in frames) - min(f['z'] for f in frames) > 1.0, 'recall never moved spirit'
        last = frames[-1]
        owner = heroes[-1]
        assert ((last['x'] - owner['x']) ** 2 + (last['z'] - owner['z']) ** 2) ** .5 < 2.0, 'recalled spirit failed to return to owner'
    if case == 'exchange':
        assert heroes and max(abs(h['z'] - heroes[0]['z']) for h in heroes) > 1.0, 'exchange never moved owner'
    if case == 'bulwark':
        telemetry = json.loads((output / 'telemetry.json').read_text())
        assert max(row['barrier'] for row in telemetry) >= 28, 'guard command never raised shield'
    if case == 'pause':
        # Measure repeated positive-health minion position while game is paused.
        longest = 0
        previous = None
        start = 0
        for frame in frames:
            key = (frame['x'], frame['z'], frame['health'])
            if key != previous:
                start = frame['time']
                previous = key
            longest = max(longest, frame['time'] - start)
        assert longest >= 1.0, 'pause did not hold minion position'
    result = dict(case=case, minion_count=count, hits=hits, orders=orders,
                  minimum_health=min(f['health'] for f in frames), verified=True)
    (output / 'minion-telemetry.json').write_text(json.dumps(frames, indent=2))
    (output / 'verified.json').write_text(json.dumps(result, indent=2))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/debug/bevy-open-arpg')
    parser.add_argument('--output', default='target/validation/soulbinder')
    parser.add_argument('--cases', nargs='+', choices=CASES, default=CASES)
    args = parser.parse_args()
    results = []
    for case in args.cases:
        output = Path(args.output).resolve() / case
        run_case(Path(args.binary).resolve(), output, 'soulbinder-' + case)
        result = validate(output, case)
        results.append(result)
        print(json.dumps(result), flush=True)
    all_results = [json.loads(p.read_text()) for p in sorted(Path(args.output).glob('*/verified.json'))]
    (Path(args.output) / 'results.json').write_text(json.dumps(all_results, indent=2))
