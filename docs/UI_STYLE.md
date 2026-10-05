# Reliquary interface style

The interface uses dark stone, aged brass and ember light to match the world. Decorative treatment belongs on titles, controls and borders; descriptions and numerical values stay readable.

- **Language:** Simplified Chinese by default, with an explicit language selection in Settings, available from the title screen and pause menu; the choice persists. Chinese uses bundled Noto Sans SC (SIL OFL); English uses the display and reading faces below. Translations live in `assets/data/zh_cn.tsv`, applied by `src/ui/localization.rs` while retaining stable gameplay identifiers.
- **Display type:** Cinzel, semibold, warm ivory or aged gold. Reserve it for titles, panel names and actions.
- **Reading type:** Crimson Pro, medium, warm gray. Use it for descriptions, tooltips and secondary information. Both fonts ship locally under the SIL Open Font License.
- **Depth:** restrained dark text shadows, fine brass edges and a soft ember halo on the primary action. Hover brightens the edge and surface; it does not move the control.
- **Icons:** use the painted skill, gear and talent atlases for gameplay. Speaker, journal, coins and skull use matching bronze medallions. Never use Unicode pictograms as icons.
- **Information:** show the action, relevant value and shortcut first. Put detailed difficulty and journey information in hover/press descriptions.
- **Motion:** slow rising menu embers. Keep text stationary and contrast stable.

The menu supports mouse, keyboard and gamepad. Begin, Continue, Difficulty and Audio retain Enter/Space, F9, Tab and M respectively. The narrow layout expands the menu to fit the viewport.

The shared typography is in `src/ui/theme.rs`, the title screen in `src/ui/menu.rs`, and image icon selection in `src/ui/icons.rs`.

The bottom HUD uses one centered coordinate group for its backdrop, resource orbs and skills. Resource levels crop a fixed circular fill vertically, preserving the aperture shape at every value. Compact status cards use image icons and short values; clip their child text rather than relying on a text node to clip itself.

Rebuild the utility medallion atlas with:

```sh
python3 tools/ui/generate_utility_icons.py
```

The editable vector source lives alongside the generator. Do not edit the generated PNG by hand.

Capture desktop, narrow menu, hover, gameplay and open panels on an isolated virtual display with `python3 tools/ui/capture_ui.py` after building the native binary. The harness requires Xvfb, xdotool and ImageMagick; screenshots and runtime logs go to `target/validation/`.

For animation regressions, run `python3 tools/ui/capture_ui.py --motion-review --output target/validation/motion` to capture walking and attacks. Run `python3 tools/ui/capture_ui.py --hit-review --output target/validation/hits` separately to approach enemies and capture real incoming damage. The two review modes use separate fresh runs.

Use `python3 tools/ui/capture_ui.py --combat-review --output target/validation/combat` to verify language switching, enemy focus, empty R casts and Rupture resource consumption.

Use `--settings-review` to capture both language choices, assert saved preferences and exercise settings while paused. Use `--defeat-review` for consecutive basic attack frames, desktop/narrow defeat panels and mouse-driven retry/menu actions. Basic attacks have one short weapon arc near contact, no trailing copies or ground glyphs, and one small spark per successful contact.

The defeat screen (`src/ui/death.rs`) reuses the cathedral backdrop and sliced stone frame. A bronze skull, restrained ivory title, one concise ward failure, three run metrics and one combat hint form the recap. Full-width Retry and Return to menu buttons provide direct mouse actions; R retries. Keep progression diagnostics out of this screen. Reward banking still runs when entering defeat.

Mouse controls separate travel from combat: right-click moves and cancels attack intent; left-click attacks or swings in place on empty ground. Rupture uses R. Run `--controls-review` for the native pointer-control capture.

## Combat presentation

The combat HUD shows resource orbs, skill icons, a thin experience bar and four utility icons. Utility details and the minimap legend appear on hover. Opening a panel preserves the clean HUD. The journal uses separate headings and paragraphs, supports scrolling, and closes with its button or J. Item pickup prompts appear only near collection range; gear comparisons belong in the inventory.

Inventory and talent panels use independent headings and unobstructed content areas. Long item names are translated before layout and can wrap. The north-up minimap marks cardinal directions, shows the player's actual facing with an arrow, and the objective compass points toward the tracked target.
