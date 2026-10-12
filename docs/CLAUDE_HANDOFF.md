# Claude Code 交接：Bevy Open ARPG

更新时间：2026-10-05。仓库：`/home/freeman.xiong/Documents/github/stars-labs/bevy-open-arpg`。

## 当前实现与边界

游戏已有四个可选职业：灰誓卫士、骨弦游猎、烬语者、缚魂使。缚魂使已从单灵体实验推进为正常职业，接入六项指令技能、两条三阶天赋、专属魂能、召唤物承伤/死亡/重聚、存档和章节携带。旧召唤实验模块与运行开关已删除，不要恢复它们。

**前三职业的新专精树仍是设计提案。** `docs/changes/2026-10-05-class-talents.md` 提出了三职业各两条三节点路线，共18节点；这些节点尚未开发，也没有平衡结论。当前运行的前三职业技能书及既有成长机制不能当作提案已经实现。

用户已经要求提交更改。累计职业和表现改动提交为 `985381f`，三职业天赋提案为 `cd0fd5c`，召唤原型验证为 `cc058bc`。当前完整缚魂使这一轮的提交状态以 `git status`、`git log` 为准；保护现有改动，不要 reset、覆盖或清理。提交不代表授权推送、合并或部署。

## 先读与工作惯例

1. 阅读 `AGENTS.md`、本文件、`docs/SOULBINDER_DESIGN.md`、`docs/UI_STYLE.md`、`docs/ASSET_AUDIT.md`。涉及战斗时再读 `docs/COMBAT_GROWTH.md` 与 `docs/SKILL_FEEDBACK.md`。
2. 默认中文是用户明确要求，覆盖通用的默认英文惯例。语言入口位于设置，不使用 F8。界面、剧情、技能和天赋不能中英混杂。
3. 操作是右键移动、左键攻击；Q/E/R 为职业技能槽。命中卡刀、拖刀与恢复，必须由真实接触驱动；升级和加点要改变真实出手与战斗循环。
4. 按策划 → 战斗设计 → 美术 → 技术美术 → 实机验收推进，自主完成已授权工作。“已修复/完成”必须附验证证据；未验证明确说明。
5. 数值放 RON，模型/音效修改生成器后再生成，不手改二进制。保留 native/wasm 门控和已审计 Bevy features。不要同时运行原生编译与 Web release 构建。

## 缚魂使的完整循环

设计入口：`docs/SOULBINDER_DESIGN.md`。正常新局从职业选择进入，普攻真实命中建立标记，契灵追击；付费技能在实际释放时发出类型化指令。

| 槽位 | 技能 | 实际行为 |
| --- | --- | --- |
| Q | 归契 | 召回并治疗活契灵，取消追击和待命齐击；不能即时复活死亡契灵 |
| Q | 易魂 | 与任意活契灵换位，约束在地图可通行边界内，并清除旧移动/攻击指令 |
| E | 灵契齐击 | 标记后命令重击；无有效标记或活契灵时不扣魂能 |
| E | 契灵护阵 | 召回守势，给予主人护盾、治疗并减少契灵承伤 |
| R | 解契狂攻 | 限时提高攻击速度和伤害；持续金色铭纹显示强化状态 |
| R | 焚契祭爆 | 献祭一名契灵造成真实范围伤害，随后等待重聚；第二名骨卫可继续作战 |

天赋第一/第二/第三阶在等级2/4/6开放，最多投入五点，两条路线的第三阶互斥。脱战可重置；变化保留召唤物生命比例和重聚等待时间，不靠换构筑刷满生命。

- **骨卫统御**：双卫改变数量；护主拦截接近主人的敌人；骨墙提供有冷却的护佑与守势减伤。
- **灵契共战**：共鸣由三次唯一的真实普攻接触积累重劈；共击让齐击命中触发主人回响；灵契让狂攻重劈横扫周围目标。

召唤物使用独立生命组件，近战、投射物、范围及首领攻击都可承伤，不进入敌人死亡奖励链。死亡先显示倒下姿势，再隐藏并等待重聚。标记过期、目标死亡或切换时取消未接触攻击；契灵伤害不生成新的主人标记或无限资源回路。等级与装备强化真实召唤伤害和生存。

角色、契灵、骨卫有专用锚定模型与生成源，职业有独立肖像和七格技能图标。护阵/狂攻使用持续的小型铭纹，不使用实心光球。宠物状态卡显示生命条、重聚、追击/待命和已解锁的共鸣状态，支持中文/英文。职业选择页随窗口调整卡片宽度，短窗口可滚动，确认按钮固定在卡片外。

## 本轮修复与验证状态

- 旧职业测试已迁移到当前 `SkillLoadout` 和槽位冷却接口；不恢复废弃兼容 API。
- HUD 查询增加互斥过滤，避免 `BackgroundColor` / `ImageNode` 的 B0001。全量测试中的 HUD 查询初始化回归已执行通过。
- 23个剧情场景的标题、摘要、角色名、44句对白及行动提示使用完整中文译文；新增覆盖测试检查没有残留英文字母。角色名统一为执事艾达、军需官维伊、瓦尔、马尔雷克、维斯珀、妮克萨拉。
- 17个原生真实输入场景已全部通过，覆盖空挥不标记、标记追击、齐击、三次普攻共鸣、六项指令、护主、承伤/死亡/重聚、首领伤害等。录像、逐帧数据和日志在 `target/validation/soulbinder/`；界面截图在 `target/validation/soulbinder/ui/`。
- 最终完整测试全部通过：844项单元测试、27项集成测试，共871项；格式、全功能check、全目标全功能Clippy均通过。旧接口错误已消除，未恢复兼容API。
- 最终二进制另外重录标记、双骨卫、普攻共鸣3例全部通过；960/1280界面复查通过，素材RGB/透明通道导出问题已修复。
- 默认原生构建及第二章120帧启动已通过；Web完整构建正在执行，具体结果见 `docs/SOULBINDER_VERIFICATION.md`，完成后补录。该文还列出没有验证的完整通关、购点GUI、试听及浏览器范围。

### 仍有效的历史证据入口

这些证明各阶段的实现和回归基线，不代替当前版本最终检查。

| 证据 | 内容与限制 |
| --- | --- |
| `target/validation/skill-vfx/verification.md`、`results.json` | 三职业18个主动技能和3个符文变体的真实窗口录像、伤害与释放事件；浏览器未录制 |
| `target/validation/hud-startup-fix/window/gameplay.png` | HUD 查询冲突修复后的窗口启动与进入游戏 |
| `target/validation/combat-growth/verification.md` | 2026-09-30天赋/符文矩阵、接触姿势、卡刀拖刀和帧率后坐基线 |
| `target/validation/combat-growth/ui/` | 当阶段技能页、狭窄窗口与升级反馈截图 |

原先 `focus-8` 最后补录失败覆盖了 `game.log`，但旧 `contact.mp4`、`telemetry.json`、`result.json` 仍在。`capture-final.log` / `capture-final-xvfb-error.log` 记录了当时 X11 socket 问题，不能把该旧录像当作后续受击到期竞争修复的补录。本轮 Xvfb 已实际使用，旧的显示权限阻塞已不再是当前状态。

## 关键代码与工具

| 文件 | 责任 |
| --- | --- |
| `src/class.rs`、`assets/data/classes.ron` | 四职业、资源、技能、符文与解锁数据 |
| `src/combat/summons.rs`、`assets/data/summons.ron` | 生产召唤控制器、指令、生命、天赋与平衡 |
| `src/combat/summon_logic.rs` | 标记去重/过期、接触时机、追击约束 |
| `src/combat.rs`、`src/combat/ranged.rs` | 实际付费释放、目标判定、近战/投射/落点伤害 |
| `src/combat/skill_vfx.rs`、`skill_vfx_art.rs` | 刀弧、冲击波、冰纹、位移和投射轨迹 |
| `src/player.rs` | 接触保持、动作、等级/天赋点、职业和章节携带 |
| `src/enemy.rs`、`src/feedback.rs` | 契灵承伤、后坐、卡刀拖刀、声音与限流 |
| `src/save.rs` | 职业、技能配置、召唤天赋与进度保存恢复 |
| `src/ui/class_select.rs`、`src/ui/summon_talents.rs` | 响应式职业选择、召唤天赋投入和重置 |
| `src/ui/skills.rs`、`src/ui/progression.rs` | 技能说明、配置、解锁与升级反馈 |
| `src/ui/localization.rs`、`assets/data/*zh_cn.tsv` | 双语显示、完整剧情翻译及回归检查 |
| `src/assets.rs`、`tests/asset_manifest.rs` | 运行素材加载与清单检查 |
| `tools/blender/`、`tools/audio/generate_sfx.py` | 模型和音效生成源 |
| `tools/ui/capture_soulbinder.py` | 召唤职业真实键鼠输入、录像与遥测断言 |
| `tools/ui/capture_combat_growth.py`、`verify_combat_growth.py` | 技能/成长场景与逐帧断言 |

验证准备场景仅用于 native + `dev_tools`，正常运行不设置 `BEVY_OPEN_ARPG_COMBAT_REVIEW`。不要把固定验证装备、敌人生命或关闭暴击的设置带入正常玩法。

## 继续工作的优先顺序

1. 阅读 `docs/SOULBINDER_VERIFICATION.md` 的最终结果与边界，复用已有检查及捕获工具，不重复修复已通过项。
2. 围绕实际玩家操作复查命中手感、连续技能与普攻、首领战、契灵寻路/换位及存档后继续；17段场景录像不能推出全流程平衡已经完成。
3. 试听命中和群攻音量。目前录制无声，文件/PCM检查不能代替主观试听。
4. 继续审计全场景的死亡、拾取、hover、精英/首领特效、地图导航、剧情浮条和狭窄窗口。完整剧情翻译不代表全游戏所有动态文案都已人工验收。
5. 若继续前三职业专精树，先按现有提案建立一个职业的完整闭环：节点实际行为、资源/连招变化、专属表现、真实输入对比、平衡与验收。不要把剩余两职业也先铺成只有数值和文案的树。

## 运行与验证

先进入 Nix 开发环境。缓存的 `.direnv/flake-profile-a5d5b61aa8a61b7d9d765e1daf971a9a578f1cfa.rc` 是 Bash 脚本，路径以实际文件为准；找不到 ALSA/pkg-config/Xvfb 时修复环境，不随意修改 Bevy features。

```bash
cargo run
cargo fmt --all -- --check
cargo check --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo run --locked -- --headless-smoke --no-audio --smoke-chapter=2 --smoke-frames=120
# 不与原生编译同时运行
bash scripts/build_web.sh

cargo build --locked --features dev_tools
python3 tools/ui/capture_soulbinder.py --help
python3 tools/ui/capture_combat_growth.py --cases focus:8 wrath:8
python3 tools/ui/verify_combat_growth.py
```

Claude 接手后先检查最新提交、工作区和证据，继续实际开发并用中文说明结果与限制。不要重复实现已落地机制，不要只写方案，不要把截图或规则测试包装成完整品质/平衡结论。
