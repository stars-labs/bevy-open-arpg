use crate::GameState;
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameAssets {
    pub hero: Handle<WorldAsset>,
    pub skeleton: Handle<WorldAsset>,
    pub cultist: Handle<WorldAsset>,
    pub butcher: Handle<WorldAsset>,
    pub sword: Handle<WorldAsset>,
    pub chest: Handle<WorldAsset>,
    pub altar: Handle<WorldAsset>,
    pub quartermaster: Handle<WorldAsset>,
    pub fortune_shrine: Handle<WorldAsset>,
    pub storm_shrine: Handle<WorldAsset>,
    pub healing_well: Handle<WorldAsset>,
    pub cursed_shrine: Handle<WorldAsset>,
    pub blood_obelisk: Handle<WorldAsset>,
    pub reliquary_vault: Handle<WorldAsset>,
    pub ember_rift_prop: Handle<WorldAsset>,
    pub ashen_pylon: Handle<WorldAsset>,
    pub lore_page: Handle<WorldAsset>,
    pub breakable_urn: Handle<WorldAsset>,
    pub breakable_coffer: Handle<WorldAsset>,
    pub slash_arc: Handle<WorldAsset>,
    pub hit_spark: Handle<WorldAsset>,
    pub bone_shatter: Handle<WorldAsset>,
    pub bone_impact: Handle<WorldAsset>,
    pub blood_spray: Handle<WorldAsset>,
    pub execution_burst: Handle<WorldAsset>,
    pub arcane_impact: Handle<WorldAsset>,
    pub holy_impact: Handle<WorldAsset>,
    pub ember_impact: Handle<WorldAsset>,
    pub frost_impact: Handle<WorldAsset>,
    pub void_impact: Handle<WorldAsset>,
    pub frenzy_impact: Handle<WorldAsset>,
    pub vampiric_siphon: Handle<WorldAsset>,
    pub desecrator_burst: Handle<WorldAsset>,
    pub guard_clash: Handle<WorldAsset>,
    pub armor_break: Handle<WorldAsset>,
    pub soul_ward_hit: Handle<WorldAsset>,
    pub hit_bone_rune: Handle<WorldAsset>,
    pub hit_bone_lock: Handle<WorldAsset>,
    pub marrow_flash: Handle<WorldAsset>,
    pub bone_fracture_echo: Handle<WorldAsset>,
    pub elite_affix_break: Handle<WorldAsset>,
    pub shadow_burst: Handle<WorldAsset>,
    pub headshot_burst: Handle<WorldAsset>,
    pub crit_bone_crown: Handle<WorldAsset>,
    pub crit_burst: Handle<WorldAsset>,
    pub stagger_burst: Handle<WorldAsset>,
    pub shadow_trail: Handle<WorldAsset>,
    pub loot_prism: Handle<WorldAsset>,
    pub objective_sigil: Handle<WorldAsset>,
    pub ember_vent: Handle<WorldAsset>,
    pub boss_summon_portal: Handle<WorldAsset>,
    pub affix_ember_aura: Handle<WorldAsset>,
    pub affix_arcane_aura: Handle<WorldAsset>,
    pub affix_frost_aura: Handle<WorldAsset>,
    pub affix_blood_aura: Handle<WorldAsset>,
    pub affix_ward_aura: Handle<WorldAsset>,
}

pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), load_assets)
            .add_systems(Update, finish_loading.run_if(in_state(GameState::Loading)));
    }
}

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(GameAssets {
        hero: asset_server.load("models/hero.glb#Scene0"),
        skeleton: asset_server.load("models/skeleton.glb#Scene0"),
        cultist: asset_server.load("models/cultist.glb#Scene0"),
        butcher: asset_server.load("models/butcher.glb#Scene0"),
        sword: asset_server.load("models/sword.glb#Scene0"),
        chest: asset_server.load("models/chest.glb#Scene0"),
        altar: asset_server.load("models/altar.glb#Scene0"),
        quartermaster: asset_server.load("models/quartermaster.glb#Scene0"),
        fortune_shrine: asset_server.load("models/fortune_shrine.glb#Scene0"),
        storm_shrine: asset_server.load("models/storm_shrine.glb#Scene0"),
        healing_well: asset_server.load("models/healing_well.glb#Scene0"),
        cursed_shrine: asset_server.load("models/cursed_shrine.glb#Scene0"),
        blood_obelisk: asset_server.load("models/blood_obelisk.glb#Scene0"),
        reliquary_vault: asset_server.load("models/reliquary_vault.glb#Scene0"),
        ember_rift_prop: asset_server.load("models/ember_rift_prop.glb#Scene0"),
        ashen_pylon: asset_server.load("models/ashen_pylon.glb#Scene0"),
        lore_page: asset_server.load("models/lore_page.glb#Scene0"),
        breakable_urn: asset_server.load("models/breakable_urn.glb#Scene0"),
        breakable_coffer: asset_server.load("models/breakable_coffer.glb#Scene0"),
        slash_arc: asset_server.load("models/slash_arc.glb#Scene0"),
        hit_spark: asset_server.load("models/hit_spark.glb#Scene0"),
        bone_shatter: asset_server.load("models/bone_shatter.glb#Scene0"),
        bone_impact: asset_server.load("models/bone_impact.glb#Scene0"),
        blood_spray: asset_server.load("models/blood_spray.glb#Scene0"),
        execution_burst: asset_server.load("models/execution_burst.glb#Scene0"),
        arcane_impact: asset_server.load("models/arcane_impact.glb#Scene0"),
        holy_impact: asset_server.load("models/holy_impact.glb#Scene0"),
        ember_impact: asset_server.load("models/ember_impact.glb#Scene0"),
        frost_impact: asset_server.load("models/frost_impact.glb#Scene0"),
        void_impact: asset_server.load("models/void_impact.glb#Scene0"),
        frenzy_impact: asset_server.load("models/frenzy_impact.glb#Scene0"),
        vampiric_siphon: asset_server.load("models/vampiric_siphon.glb#Scene0"),
        desecrator_burst: asset_server.load("models/desecrator_burst.glb#Scene0"),
        guard_clash: asset_server.load("models/guard_clash.glb#Scene0"),
        armor_break: asset_server.load("models/armor_break.glb#Scene0"),
        soul_ward_hit: asset_server.load("models/soul_ward_hit.glb#Scene0"),
        hit_bone_rune: asset_server.load("models/hit_bone_rune.glb#Scene0"),
        hit_bone_lock: asset_server.load("models/hit_bone_lock.glb#Scene0"),
        marrow_flash: asset_server.load("models/marrow_flash.glb#Scene0"),
        bone_fracture_echo: asset_server.load("models/bone_fracture_echo.glb#Scene0"),
        elite_affix_break: asset_server.load("models/elite_affix_break.glb#Scene0"),
        shadow_burst: asset_server.load("models/shadow_burst.glb#Scene0"),
        headshot_burst: asset_server.load("models/headshot_burst.glb#Scene0"),
        crit_bone_crown: asset_server.load("models/crit_bone_crown.glb#Scene0"),
        crit_burst: asset_server.load("models/crit_burst.glb#Scene0"),
        stagger_burst: asset_server.load("models/stagger_burst.glb#Scene0"),
        shadow_trail: asset_server.load("models/shadow_trail.glb#Scene0"),
        loot_prism: asset_server.load("models/loot_prism.glb#Scene0"),
        objective_sigil: asset_server.load("models/objective_sigil.glb#Scene0"),
        ember_vent: asset_server.load("models/ember_vent.glb#Scene0"),
        boss_summon_portal: asset_server.load("models/boss_summon_portal.glb#Scene0"),
        affix_ember_aura: asset_server.load("models/affix_ember_aura.glb#Scene0"),
        affix_arcane_aura: asset_server.load("models/affix_arcane_aura.glb#Scene0"),
        affix_frost_aura: asset_server.load("models/affix_frost_aura.glb#Scene0"),
        affix_blood_aura: asset_server.load("models/affix_blood_aura.glb#Scene0"),
        affix_ward_aura: asset_server.load("models/affix_ward_aura.glb#Scene0"),
    });
}

fn finish_loading(assets: Option<Res<GameAssets>>, mut next_state: ResMut<NextState<GameState>>) {
    if assets.is_none() {
        return;
    }
    next_state.set(GameState::MainMenu);
}
