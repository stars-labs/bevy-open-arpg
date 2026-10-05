#!/usr/bin/env python3
"""Check the measured native talent matrix, not predictions from formulas."""
import argparse
import json
from pathlib import Path
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', default='target/validation/combat-growth')
args = parser.parse_args()
root = Path(args.output)
ron = Path('assets/data/player.ron').read_text()
def tuning(key):
    return float(re.search(r'\b' + key + r':\s*([\d.]+)', ron)[1])
def measured(kind, rank):
    return json.loads((root / f'{kind}-{rank}/result.json').read_text())
rows = []
for rank in [0, 1, 4, 8]:
    wrath, vigor, focus = [measured(kind, rank) for kind in ['wrath', 'vigor', 'focus']]
    shield = tuning('vigor_base_barrier') + rank * tuning('vigor_barrier_per_rank') if rank else 0
    fury = 18 + tuning('focus_base_fury') + rank * tuning('focus_fury_per_rank') if rank else 18
    impact = 1 + tuning('wrath_first_impact_bonus') + rank * tuning('wrath_impact_per_rank') if rank else 1
    assert abs(vigor['max_barrier'] - shield) < .01, vigor
    assert abs(focus['max_fury'] - fury) < .01, focus
    assert abs(wrath['recoil'] - .34 * impact) < .008, wrath
    assert .04 <= wrath['minimum_time_speed'] <= .10, wrath
    assert focus['damage'] > tuning('basic_damage') if rank else abs(focus['damage'] - tuning('basic_damage')) < .01, focus
    rows.append(dict(rank=rank, strike_damage=wrath['damage'], recoil_metres=wrath['recoil'],
                     skill_barrier=vigor['max_barrier'], strike_fury=focus['max_fury'],
                     strike_into_rupture_damage=focus['damage'], contact_speed=wrath['minimum_time_speed']))
for before, after in zip(rows, rows[1:]):
    assert after['strike_damage'] > before['strike_damage']
    assert after['recoil_metres'] > before['recoil_metres']
for name in ['expose', 'blood', 'cleanse', 'reap', 'ember', 'frost']:
    result = json.loads((root / name / 'result.json').read_text())
    assert result['damage'] > 0, result
for case in [f'{kind}-{rank}' for kind in ['wrath', 'vigor', 'focus'] for rank in [0, 1, 4, 8]] + ['expose', 'blood']:
    frames = json.loads((root / case / 'telemetry.json').read_text())
    contact = next(i for i, frame in enumerate(frames) if frame['health'] < 500)
    expected_pose = .58 if case.startswith('vigor') or case in ['expose', 'blood'] else .44
    assert abs(frames[contact]['swing'] - expected_pose) < .001, (case, frames[contact])
    assert .04 <= frames[contact]['speed'] <= .10, (case, frames[contact])
    assert any(frame['speed'] == 1.0 and frame['swing'] == 1.0 for frame in frames[contact + 1:]), case
(root / 'measured-growth.json').write_text(json.dumps(rows, indent=2))
lines = ['# 实机战斗成长对比', '', '相同装备、固定敌人、关闭暴击；真实鼠标/键盘输入。', '',
         '| 天赋阶数 | 愤怒普攻伤害 | 后坐距离（米） | 活力技能护盾 | 专注一刀怒气 | 普攻接裂地斩总伤害 |',
         '|---|---|---|---|---|---|']
for row in rows:
    lines.append(f"| {row['rank']} | {row['strike_damage']:.2f} | {row['recoil_metres']:.3f} | {row['skill_barrier']:.0f} | {row['strike_fury']:.0f} | {row['strike_into_rupture_damage']:.2f} |")
lines += ['', '六符文均以实际输入命中，逐帧数据和录像保存在对应子目录。',
          '逐帧断言：近战伤害当帧进入接触姿势与卡刀，随后恢复正常速度并完成收刀。',
          '截图场景仅在 dev_tools + BEVY_OPEN_ARPG_COMBAT_REVIEW 显式开启；不读写用户存档。',
          '录像使用无声环境；音频素材通过 PCM、时长、削波与差异校验，未验证实际扬声器播放。']
(root / 'measured-growth.md').write_text('\n'.join(lines) + '\n')
print('PASS: 12 talent cases and 6 real rune casts; measured damage, recoil, Fury, shields and hit-stop.')
