#!/usr/bin/env python3
"""Cross-check native summon recordings against measured player and spirit damage."""
import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', default='target/validation/summon')
args = parser.parse_args()
root = Path(args.output)
cases = ['idle', 'empty', 'skill-only', 'mark', 'coop', 'kill', 'leash', 'pause']
results = []
for name in cases:
    folder = root / f'summon-{name}'
    result = json.loads((folder / 'result.json').read_text())
    rows = json.loads((folder / 'telemetry.json').read_text())
    log = (folder / 'game.log').read_text()
    assert 'panicked' not in log and 'ERROR' not in log
    assert (folder / 'contact.mp4').stat().st_size > 1000
    commands, hits = result['commands'], result['spirit_hits']
    spirit_damage = sum(h['damage'] for h in hits)
    player_damage = 0 if name in ('idle', 'empty') else 28 if name == 'skill-only' else 50 if name == 'coop' else 22
    assert abs(result['damage'] - player_damage - spirit_damage) < .05, (name, result)
    if name in ('idle', 'empty', 'skill-only'):
        assert not commands and not hits
    else:
        assert sum(c[0] == 'mark' for c in commands) == 1, commands
    if name == 'empty':
        assert min(r['swing'] for r in rows) < 1 and result['damage'] == 0
    if name == 'skill-only':
        assert 'SKILL_VFX_RELEASE' in log and min(r['fury'] for r in rows) <= 55
    if name == 'coop':
        assert sum(h['empowered'] for h in hits) == 1
        assert sum(c[0] == 'empower' for c in commands) == 1
        assert next(h for h in hits if h['empowered'])['damage'] == 25
    for h in hits:
        assert h['pose'] == (.58 if h['empowered'] else .44)
        assert h['damage'] == (25 if h['empowered'] else 9)
    # In this fixture only the hero's initial hit may slow the global clock.
    pet_damage_frames = []
    for before, after in zip(rows, rows[1:]):
        if round(before['health'] - after['health'], 3) in (9, 25):
            assert after['speed'] == 1.0, (name, after)
            pet_damage_frames.append(after)
    if name == 'pause':
        assert result['pause_frozen_seconds'] > 1
    if name in ('kill', 'leash'):
        assert 'SUMMON_CLEAR invalid=true' in log
        after_clear = log.split('SUMMON_CLEAR invalid=true', 1)[1]
        assert 'SUMMON_HIT' not in after_clear
    result['player_damage'] = player_damage
    result['spirit_damage'] = spirit_damage
    result['pet_damage_frames_at_full_speed'] = len(pet_damage_frames)
    results.append(result)
(root / 'results.json').write_text(json.dumps(results, indent=2))
lines = ['# 单灵体协同实机验证', '',
         '固定装备、关闭暴击；真实鼠标/键盘输入。以下是原型验证，不代表完整召唤职业或平衡验收。', '',
         '| 场景 | 玩家伤害 | 灵体伤害 | 灵体攻击次数 | 强化次数 |', '|---|---:|---:|---:|---:|']
for r in results:
    lines.append(f"| {r['case']} | {r['player_damage']} | {r['spirit_damage']:.0f} | {len(r['spirit_hits'])} | {sum(h['empowered'] for h in r['spirit_hits'])} |")
lines += ['', '普通挥斩接触姿势44%，强化重劈58%；强化命中一次消费，后续恢复9伤害。',
          '未标记、空挥、只放技能时灵体零伤害；死亡与超距清除后不再攻击；暂停期间位置、动作及标记时限冻结。',
          '每次可单独辨识的灵体伤害帧，世界时间速度为1；不由灵体发起普通命中全局停顿。',
          '击杀场景包含过量伤害，伤害列记录生命变化量，不等同有效生命扣除。',
          '换目标及同施法多目标去重由7项规则测试覆盖，本轮未做对应多敌人实机录像。',
          '召唤物生存、复杂绕障、主动敌群和首领平衡未验证。']
(root / 'measured-summon.md').write_text('\n'.join(lines) + '\n')
print('PASS: 8 native recordings; separate damage attribution, command guards, contact poses, pause and leash.')
