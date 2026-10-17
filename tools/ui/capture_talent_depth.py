#!/usr/bin/env python3
"""Capture six deep Soulbinder talents with real native inputs.

Rebirth deliberately uses controller-death fixtures, not enemy damage. Enemy
attack integration is covered by the existing Soulbinder contact suite.
"""
import argparse
import json
from pathlib import Path
import re

from capture_combat_growth import run_case

CASES = ('aftershock', 'siphon', 'relay', 'retreat', 'flank', 'rebirth')


def verify(output, case):
    text = (output / 'game.log').read_text()
    short = case.removeprefix('soulbinder-')
    def require(condition, message):
        if not condition:
            raise AssertionError(f'{case}: {message}')
    minions = []
    for row in text.splitlines():
        match = re.search(r'SOULBINDER_MINION \S+ ([\d.]+) (\S+) ([-\d.]+) ([-\d.]+) ([\d.]+) ([\d.]+) ([\d.]+)', row)
        if match:
            t, entity, x, z, hp, maximum, dead = match.groups()
            minions.append((float(t), entity, float(x), float(z), float(hp), float(maximum), float(dead)))
    require(len(minions) > 30, 'missing sustained summon telemetry')
    if short in ('aftershock', 'siphon', 'relay'):
        require(f'SUMMON_{short.upper()}' in text, 'deep talent produced no event')
        require('SUMMON_HIT' in text, 'no real summon contact')
    if short == 'relay':
        ids = set(re.findall(r'SOULBINDER_ENEMY \S+ [\d.]+ (\S+)', text))
        require(len(ids) == 2, 'relay requires two real enemies')
    if short == 'retreat':
        require('DEPTH_FIXTURE retreat' in text, 'low-health fixture never applied')
        low = [r for r in minions if 0 < r[4] / r[5] < .3]
        require(bool(low), 'no low-health observation')
        first = low[0]
        later = [r for r in minions if r[1] == first[1] and r[0] > first[0] + 1]
        require(any(r[4] > first[4] + 5 for r in later), 'guard did not heal')
        require(any(r[3] > first[3] + 1 for r in later), 'guard did not return toward owner')
    if short == 'flank':
        frames = {}
        for row in minions:
            frames.setdefault(row[0], []).append(row)
        heroes = re.findall(r'SOULBINDER_HERO \S+ [\d.]+ ([-\d.]+) ([-\d.]+)', text)
        require(bool(heroes), 'missing owner position')
        owner_x = float(heroes[0][0])
        require(any(len(rows) == 2 and min(r[2] for r in rows) < owner_x - .25 and max(r[2] for r in rows) > owner_x + .25
                    for rows in frames.values()), 'guards never occupied opposite target sides')
        require('SUMMON_HIT' in text, 'flanking guards never landed contact')
    if short == 'rebirth':
        require('controller_death step=2' in text, 'second controller death not injected')
        require('last_stand=true' in text and 'last_stand=false' in text, 'rebirth cooldown did not select ordinary reform')
        require(text.count('SUMMON_REVIVE') >= 2, 'both reform cycles did not finish')
        require(any(.45 <= r[4] / r[5] <= .55 and r[6] == 0 for r in minions), 'no half-health last-stand revival')
        require(any(r[6] > 5 for r in minions), 'ordinary six-second reform was not observed')
        starts = {}
        cycles = []
        for row in minions:
            if row[4] <= 0 and row[1] not in starts:
                starts[row[1]] = row[0]
            elif row[4] > 0 and row[1] in starts:
                cycles.append(row[0] - starts.pop(row[1]))
        require(any(.9 <= duration <= 1.6 for duration in cycles), 'last stand did not reform near 1.2 seconds')
        require(any(5.7 <= duration <= 6.5 for duration in cycles), 'cooldown death did not reform near six seconds')
    result = {'case': case, 'passed': True, 'death_source': 'controller fixture' if short == 'rebirth' else None}
    (output / 'depth-result.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/debug/bevy-open-arpg')
    parser.add_argument('--output', default='target/validation/talent-depth')
    parser.add_argument('--cases', nargs='+', choices=CASES, default=CASES)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    for name in args.cases:
        case = 'soulbinder-' + name
        output = Path(args.output).resolve() / case
        if not args.verify_only:
            run_case(Path(args.binary).resolve(), output, case)
        verify(output, case)
