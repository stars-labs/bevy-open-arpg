# PixelFX 技能特效

五种动画由 PixelFX 的固定版本生成，调色使用暗金、灰烬、冰蓝和幽魂绿。播放使用透明图集、逐帧时间与原始锚点，放置在有深度遮挡的3D场景中。

| 素材 | 实际触发 |
| --- | --- |
| 刀光 | 灰誓卫士左键普攻释放 |
| 重击命中 | 灰誓卫士 R 重劈实际造成伤害 |
| 火球 | 烬语者左键火球弹体飞行 |
| 冰霜新星 | 烬语者 E 霜环释放 |
| 召唤出现 | 缚魂使契灵初次出现与死亡后重聚 |

召唤素材保留符文、碎石与灰尘，使用游戏自身的契灵模型。近战刀光按攻击方向摆放，火球按实际飞行方向旋转，命中与召唤效果锚定各自的世界位置。冰霜新星将上游侧视冰晶改编为地面径向冰晶和扩散环。

运行 `node tools/vfx/generate_pixelfx.mjs` 可重新生成五张PNG图集、JSON元数据和运行时RON。首次运行会把上游固定提交下载到 `target/pixelfx-upstream`。五张图集共约1.2MB，再次生成的文件哈希一致。

上游提交为 `044e12a45dc9cc312cec50ee606dfc98afe5adc2`，许可与署名随素材保存。来源 https://github.com/patch-gg/pixelfx 。

## 实机验证

2026-11-12，在 RTX 3070 Laptop / Vulkan 上通过七个桌面场景，使用同一最终二进制和相同五张图集、运行时元数据哈希。普攻22、重击29.52、火球17、霜环18.72的实际伤害与对应特效都已记录。召唤通过正常R献祭及重聚验证。无目标裂地斩按目标要求拒绝施放，真实起手后退距脱靶则实际扣费并出现刀光，伤害、命中冲击与震屏均为0。所有场景世界速度保持1，运行日志无panic、B0001或渲染ERROR。

实捕目录为 `target/validation/pixelfx/final-v3/`。`review-contactsheet.png` 汇总五种实际峰值帧，`review-frames.json` 记录完整截图和录像时间，`verification.json` 记录二进制及资产一致性。另在960×640复查霜环，完整圆环与英雄轮廓可见，证据位于 `target/validation/pixelfx/short-960/`。

本轮按实机画面修正了侧视霜环的地面投影、召唤符文的地形裁切和普通刀光的亮帧时长。录制脚本等待首帧编码后才发输入，并记录每场使用的图集哈希。

格式、严格全目标全功能Clippy、907项测试、原生默认及全功能检查、wasm32/WebGPU检查、第二章120帧启动均通过。完整Web发布构建通过，构建标识为 `2e85e2990b2d8e61`。Chromium 156 / AMD WebGPU 中通过正常菜单进入烬语者，实际释放火球，逐帧截图可见弹体动画，控制台无渲染ERROR或shader错误。证据为 `target/validation/pixelfx/web-fireball-sequence.png` 与 `web-amd-fireball-console.json`。

浏览器兼容性仍有一项限制。NVIDIA 浏览器后端在本地新版本与线上旧版本均出现 `VK_ERROR_OUT_OF_DEVICE_MEMORY` 和 DeviceLost，原因尚未定位。本轮浏览器验证通过独立测试浏览器的低功耗适配器启动参数选择AMD完成，生产配置未作变更。五种效果的完整技能验证在原生Vulkan完成。
