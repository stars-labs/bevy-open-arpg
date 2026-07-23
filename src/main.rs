mod assets;
mod bestiary;
mod bounty;
mod challenge;
mod chapter;
mod combat;
mod companion;
mod data;
mod dungeon;
mod enemy;
mod feedback;
mod journey;
mod loot;
mod lore;
mod mastery;
mod milestone;
mod obelisk;
mod ordeal;
mod player;
mod rift;
mod save;
mod story;
mod ui;

use assets::GameAssetsPlugin;
use bestiary::BestiaryPlugin;
use bevy::anti_alias::{
    AntiAliasPlugin,
    smaa::{Smaa, SmaaPreset},
};
use bevy::app::AppExit;
use bevy::asset::{AssetApp, AssetMode, AssetPlugin};
use bevy::audio::AudioPlugin;
use bevy::camera::Hdr;
#[cfg(feature = "dev_tools")]
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::clipboard::Clipboard;
use bevy::core_pipeline::{CorePipelinePlugin, prepass::DepthPrepass, tonemapping::Tonemapping};
#[cfg(feature = "dev_tools")]
use bevy::dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin};
#[cfg(feature = "dev_tools")]
use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::gilrs::GilrsPlugin;
use bevy::gizmos_render::GizmoRenderPlugin;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
#[cfg(feature = "dev_tools")]
use bevy::input::mouse::MouseButton;
use bevy::light::{
    Atmosphere, AtmosphereEnvironmentMapLight, FogVolume, RectLight, ShadowFilteringMethod,
    VolumetricFog, VolumetricLight, atmosphere::ScatteringMedium,
};
use bevy::pbr::{
    AtmosphereMode, AtmosphereSettings, ContactShadows, DistanceFog, FogFalloff, PbrPlugin,
    ScreenSpaceAmbientOcclusion,
};
use bevy::picking::{
    PickingSettings,
    prelude::{MeshPickingCamera, MeshPickingSettings},
};
use bevy::post_process::{
    PostProcessPlugin,
    bloom::Bloom,
    dof::{DepthOfField, DepthOfFieldMode},
    effect_stack::{ChromaticAberration, Vignette},
};
use bevy::prelude::*;
use bevy::reflect::{
    TypeInfo, Typed,
    func::{ArgList, FunctionRegistry},
};
#[cfg(not(target_arch = "wasm32"))]
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
#[cfg(feature = "dev_tools")]
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::{
    RenderPlugin,
    settings::{
        Backends, InstanceFlags, RenderCreation, WgpuFeatures, WgpuSettings, WgpuSettingsPriority,
    },
};
use bevy::settings::{
    ReflectSettingsGroup, SaveSettingsDeferred, SaveSettingsSync, SettingsGroup, SettingsPlugin,
};
use bevy::sprite_render::SpriteRenderPlugin;
use bevy::ui_render::{GlobalUiDebugOptions, UiRenderPlugin};
use bevy::window::{ExitCondition, WindowCloseRequested};
use bevy::winit::WinitPlugin;
use bounty::BountyPlugin;
use challenge::ChallengePlugin;
use chapter::ChapterPlugin;
use combat::CombatPlugin;
use companion::CompanionPlugin;
use data::GameDataPlugin;
use dungeon::DungeonPlugin;
use enemy::{EnemyKilled, EnemyPlugin, SpawnAshenThreatWave};
use feedback::{CombatEvent, FeedbackPlugin};
use loot::LootPlugin;
use lore::LorePlugin;
use mastery::MasteryPlugin;
use milestone::MilestonePlugin;
use obelisk::ObeliskPlugin;
use ordeal::{ChapterModifier, modifier_for_run};
use player::{Player, PlayerPlugin};
use rift::RiftPlugin;
use save::{PendingLoadGame, SavePlugin};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use story::StoryPlugin;
use ui::{ChapterRecords, HudPlugin, next_unlocked_difficulty};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    GameOver,
    Victory,
}

impl GameState {
    fn label(self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::MainMenu => "main_menu",
            Self::InGame => "in_game",
            Self::GameOver => "game_over",
            Self::Victory => "victory",
        }
    }
}

pub(crate) const CARNAGE_MASTER_STREAK: u32 = 10;

#[derive(Resource, Default)]
pub(crate) struct RunStats {
    kills: u32,
    gold: u32,
    ember_shards: u32,
    affix_essence: u32,
    valor_stacks: u32,
    best_valor_stacks: u32,
    valor_timer_secs: f32,
    elapsed_secs: f32,
    completion_bonus_gold: u32,
    completion_bonus_shards: u32,
    completion_bonus_essence: u32,
    chapter_clear_bonus_gold: u32,
    chapter_clear_bonus_shards: u32,
    chapter_clear_bonus_essence: u32,
    chapter_clear_reward_claimed: bool,
    journey_score: u32,
    journey_bonus_gold: u32,
    journey_bonus_shards: u32,
    journey_bonus_essence: u32,
    journey_reward_claimed: bool,
    renown_rank: u32,
    renown_bonus_gold: u32,
    renown_bonus_shards: u32,
    renown_bonus_essence: u32,
    renown_bonus_claimed: bool,
    stash_bonus_gold: u32,
    stash_bonus_shards: u32,
    stash_bonus_essence: u32,
    stash_bonus_claimed: bool,
    altar_seals: u32,
    altar_bonus_gold: u32,
    altar_bonus_shards: u32,
    altar_bonus_essence: u32,
    altar_bonus_claimed: bool,
    primal_caches: u32,
    primal_cache_bonus_gold: u32,
    primal_cache_bonus_shards: u32,
    primal_cache_bonus_essence: u32,
    primal_cache_items_claimed: u32,
    echo_keystones: u32,
    primal_cache_echo_items: u32,
    malrec_soul_sigils: u32,
    malrec_soul_sigils_earned: u32,
    soul_sigil_caches: u32,
    completion_reward_claimed: bool,
    massacre_streak: u32,
    best_massacre_streak: u32,
    massacre_timer_secs: f32,
    massacre_bonus_gold: u32,
    ancient_augments: u32,
    primal_infusions: u32,
    potions_used: u32,
    last_stand_potions: u32,
    boss_enraged: bool,
    boss_staggers: u32,
    shrine_resonance_triggered: bool,
    elite_affix_kills: u32,
    affix_codex_mask: u16,
    seal_warden_slain: bool,
    cursed_ambush_kills: u32,
    champion_pack_kills: u32,
    champion_pack_reward_claimed: bool,
    nemesis_kills: u32,
    treasure_vaults_opened: u32,
    breakables_smashed: u32,
    health_globes_collected: u32,
    surge_kills: u32,
    reap_dash_hits: u32,
    hemorrhage_rupture_hits: u32,
    frost_nova_hits: u32,
    armory_loadouts_saved: u32,
    town_portal_returns: u32,
    pylon_kills: u32,
    salvage_progress: u32,
    salvage_caches: u32,
    alchemy_gold_transmutes: u32,
    alchemy_essence_transmutes: u32,
    alchemy_keystone_transmutes: u32,
    loot_filter_cycles: u32,
    codex_attuned_kills: u32,
    set_resonance_kills: u32,
    ruby_socketed: bool,
    emerald_socketed: bool,
    amethyst_socketed: bool,
    topaz_socketed: bool,
    iron_elixir_used: bool,
    wrath_elixir_used: bool,
    haste_elixir_used: bool,
    reliquary_momentum: u32,
    best_reliquary_momentum: u32,
    ashen_threat: u32,
    ashen_threat_surges: u32,
}

pub const ASHEN_THREAT_MAX: u32 = 100;
pub const ASHEN_THREAT_SURGE_TARGET: u32 = 3;
pub const BOSS_BREAK_TARGET: u32 = 2;
pub const CHAMPION_PACK_TARGET: u32 = 4;
pub const GLORY_SEEKER_GLOBES: u32 = 3;
pub const LAST_STAND_POTIONS: u32 = 3;
pub const LAST_STAND_HEALTH_RATIO: f32 = 0.30;
pub const CODEX_ADEPT_KILLS: u32 = 5;
pub const SET_ADEPT_KILLS: u32 = 8;
pub const GEM_ADEPT_KINDS: u32 = 4;
pub const AFFIX_CODEX_TARGET: u32 = 6;
pub(crate) const CHAMPION_PACK_REWARD_GOLD: u32 = 85;
pub(crate) const CHAMPION_PACK_REWARD_SHARDS: u32 = 3;
pub(crate) const CHAMPION_PACK_REWARD_ESSENCE: u32 = 1;
pub(crate) const TREASURE_VAULT_REWARD_GOLD: u32 = 180;
pub(crate) const TREASURE_VAULT_REWARD_SHARDS: u32 = 5;
pub(crate) const TREASURE_VAULT_REWARD_ESSENCE: u32 = 3;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub(crate) struct PrimalCacheReward {
    pub gold: u32,
    pub shards: u32,
    pub essence: u32,
}

impl PrimalCacheReward {
    pub fn is_empty(self) -> bool {
        self.gold == 0 && self.shards == 0 && self.essence == 0
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Normal,
    Nightmare,
    Hell,
    Torment,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Nightmare => "Nightmare",
            Self::Hell => "Hell",
            Self::Torment => "Torment",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::Nightmare,
            Self::Nightmare => Self::Hell,
            Self::Hell => Self::Torment,
            Self::Torment => Self::Normal,
        }
    }

    pub fn enemy_health_multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Nightmare => 1.45,
            Self::Hell => 1.95,
            Self::Torment => 2.55,
        }
    }

    pub fn enemy_damage_multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Nightmare => 1.3,
            Self::Hell => 1.7,
            Self::Torment => 2.15,
        }
    }

    pub fn reward_multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Nightmare => 1.35,
            Self::Hell => 1.75,
            Self::Torment => 2.25,
        }
    }
}

pub fn escalated_difficulty_after_clear(difficulty: Difficulty) -> Difficulty {
    match difficulty {
        Difficulty::Normal => Difficulty::Nightmare,
        Difficulty::Nightmare => Difficulty::Hell,
        Difficulty::Hell | Difficulty::Torment => Difficulty::Torment,
    }
}

#[derive(Resource, Default)]
pub struct DifficultySettings {
    pub current: Difficulty,
}

#[derive(Resource, Default)]
pub struct PauseState {
    pub paused: bool,
}

#[derive(Resource, Default)]
pub struct InventoryOpen {
    pub open: bool,
}

#[derive(Resource, Default)]
pub struct BuildOpen {
    pub open: bool,
}

#[derive(Resource, Default)]
pub struct JournalOpen {
    pub open: bool,
}

#[derive(Resource, Clone, Copy, Default)]
pub struct AudioSettings {
    pub enabled: bool,
}

impl AudioSettings {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn status_label(&self) -> &'static str {
        if self.enabled {
            "audio on"
        } else {
            "audio muted"
        }
    }
}

#[derive(Resource, Default)]
pub struct DebugVisuals {
    pub enabled: bool,
}

/// Player-facing runtime preferences persisted by Bevy Settings and exposed to Bevy Remote tooling.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "runtime", file = "preferences")]
struct OpenArpgUserSettings {
    /// Stores whether generated combat audio should start enabled on the next run.
    audio_enabled: bool,
    /// Stores whether combat-area and player-radius debug gizmos should start visible.
    debug_visuals: bool,
    /// Stores whether Bevy Remote HTTP tooling should be enabled on startup.
    remote_enabled: bool,
    /// Stores whether Bevy AssetPlugin should read unprocessed or imported processed assets.
    asset_mode: String,
    /// Stores the selected WGPU compatibility/rendering profile label.
    render_profile: String,
}

impl Default for OpenArpgUserSettings {
    fn default() -> Self {
        Self {
            audio_enabled: true,
            debug_visuals: false,
            remote_enabled: false,
            asset_mode: OpenArpgAssetMode::Unprocessed.label().to_string(),
            render_profile: "auto".to_string(),
        }
    }
}

impl OpenArpgUserSettings {
    fn from_config(config: OpenArpgRuntimeConfig) -> Self {
        Self {
            audio_enabled: config.audio_enabled,
            debug_visuals: config.debug_visuals,
            remote_enabled: config.remote_enabled,
            asset_mode: config.asset_mode.label().to_string(),
            render_profile: config
                .render_profile
                .map(OpenArpgRenderProfile::label)
                .unwrap_or("auto")
                .to_string(),
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, Default)]
struct OpenArpgPreferenceLocks {
    audio_enabled: bool,
    debug_visuals: bool,
}

#[cfg(feature = "dev_tools")]
#[derive(Resource, Debug, Clone, Copy, Default)]
struct DebugFreeCamera {
    enabled: bool,
}

#[derive(Debug, Clone, Copy, Resource)]
struct HeadlessSmokeExit {
    frames_remaining: u32,
}

const OPEN_ARPG_WINDOW_TITLE: &str = "Bevy Open ARPG";
const OPEN_ARPG_WINDOW_WIDTH: u32 = 1280;
const OPEN_ARPG_WINDOW_HEIGHT: u32 = 720;

#[derive(Debug, Clone, Copy)]
struct OpenArpgRuntimeConfig {
    audio_enabled: bool,
    audio_locked: bool,
    debug_visuals: bool,
    debug_visuals_locked: bool,
    #[cfg(feature = "dev_tools")]
    diagnostics_enabled: bool,
    #[cfg(feature = "dev_tools")]
    free_camera_enabled: bool,
    headless_smoke: bool,
    remote_enabled: bool,
    smoke_frames: u32,
    asset_mode: OpenArpgAssetMode,
    render_profile: Option<OpenArpgRenderProfile>,
    window_backend: Option<OpenArpgWindowBackend>,
    explicit_windowed_request: bool,
}

impl OpenArpgRuntimeConfig {
    fn from_env() -> Self {
        Self::from_env_and_args(std::env::args().skip(1))
    }

    fn from_env_and_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args = parse_open_arpg_cli_args(args);
        let audio_from_env = open_arpg_audio_enabled_from_env();
        let debug_visuals_from_env = open_arpg_env_flag_value("BEVY_OPEN_ARPG_DEBUG_GIZMOS");
        let headless_from_env = open_arpg_env_flag_value("BEVY_OPEN_ARPG_HEADLESS_SMOKE");
        let explicit_windowed_request = args.headless_smoke == Some(false);
        let headless_smoke = if explicit_windowed_request {
            false
        } else {
            args.headless_smoke.or(headless_from_env).unwrap_or(false)
        };
        Self {
            audio_enabled: args.audio_enabled.or(audio_from_env).unwrap_or(true),
            audio_locked: args.audio_enabled.is_some() || audio_from_env.is_some(),
            debug_visuals: args
                .debug_visuals
                .or(debug_visuals_from_env)
                .unwrap_or(false),
            debug_visuals_locked: args.debug_visuals.is_some() || debug_visuals_from_env.is_some(),
            #[cfg(feature = "dev_tools")]
            diagnostics_enabled: args
                .diagnostics_enabled
                .unwrap_or_else(|| open_arpg_env_flag("BEVY_OPEN_ARPG_DIAGNOSTICS")),
            #[cfg(feature = "dev_tools")]
            free_camera_enabled: args
                .free_camera_enabled
                .unwrap_or_else(|| open_arpg_env_flag("BEVY_OPEN_ARPG_FREE_CAMERA")),
            headless_smoke,
            remote_enabled: args
                .remote_enabled
                .unwrap_or_else(|| open_arpg_env_flag("BEVY_OPEN_ARPG_REMOTE")),
            smoke_frames: args.smoke_frames.unwrap_or_else(open_arpg_smoke_frames),
            asset_mode: args.asset_mode.unwrap_or_else(open_arpg_asset_mode),
            render_profile: args.render_profile.or_else(open_arpg_render_profile),
            window_backend: args.window_backend,
            explicit_windowed_request,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct OpenArpgCliArgs {
    audio_enabled: Option<bool>,
    debug_visuals: Option<bool>,
    #[cfg(feature = "dev_tools")]
    diagnostics_enabled: Option<bool>,
    #[cfg(feature = "dev_tools")]
    free_camera_enabled: Option<bool>,
    headless_smoke: Option<bool>,
    remote_enabled: Option<bool>,
    smoke_frames: Option<u32>,
    asset_mode: Option<OpenArpgAssetMode>,
    render_profile: Option<OpenArpgRenderProfile>,
    window_backend: Option<OpenArpgWindowBackend>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
enum OpenArpgAssetMode {
    #[default]
    Unprocessed,
    Processed,
}

impl OpenArpgAssetMode {
    fn bevy_mode(self) -> AssetMode {
        match self {
            Self::Unprocessed => AssetMode::Unprocessed,
            Self::Processed => AssetMode::Processed,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Unprocessed => "unprocessed",
            Self::Processed => "processed",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OpenArpgRenderProfile {
    Compatibility,
    Functionality,
    WebGl2,
    Gl,
}

impl OpenArpgRenderProfile {
    fn label(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::Functionality => "functionality",
            Self::WebGl2 => "webgl2",
            Self::Gl => "gl",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OpenArpgWindowBackend {
    X11,
    Wayland,
}

impl OpenArpgWindowBackend {
    fn winit_value(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Wayland => "wayland",
        }
    }

    fn label(self) -> &'static str {
        self.winit_value()
    }
}

pub fn not_paused(pause: Res<PauseState>) -> bool {
    !pause.paused
}

fn main() {
    let mut config = OpenArpgRuntimeConfig::from_env();
    let display_present = open_arpg_display_is_present();

    if config.explicit_windowed_request && !display_present {
        eprintln!(
            "No graphical display server was detected and --windowed was explicitly requested.",
        );
        eprintln!(
            "Set DISPLAY or WAYLAND_DISPLAY in this shell and relaunch, or run with `cargo run -- --headless-smoke` to continue in smoke mode.",
        );
        std::process::exit(1);
    }

    if should_fallback_to_headless_smoke(config, display_present) {
        eprintln!(
            "No graphical display server was detected. Falling back to headless smoke mode for this run.",
        );
        eprintln!("Set DISPLAY or WAYLAND_DISPLAY in this shell for popup mode.");
        config.headless_smoke = true;
    }

    apply_process_env_overrides(config);
    log_startup_window_diagnostic(config);

    let mut app = build_app(config);
    app.run();
}

fn should_fallback_to_headless_smoke(config: OpenArpgRuntimeConfig, display_present: bool) -> bool {
    !config.headless_smoke && !display_present && !config.explicit_windowed_request
}

fn build_app(config: OpenArpgRuntimeConfig) -> App {
    let mut app = App::new();
    let mut default_plugins = DefaultPlugins
        .build()
        .set(AssetPlugin {
            // On web, assets are fetched over HTTP relative to the page, so the
            // path must stay relative; native runs anchor to the crate root so
            // `cargo run` works from any directory.
            #[cfg(not(target_arch = "wasm32"))]
            file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
            #[cfg(target_arch = "wasm32")]
            file_path: "assets".to_string(),
            #[cfg(not(target_arch = "wasm32"))]
            processed_file_path: format!("{}/imported_assets/Default", env!("CARGO_MANIFEST_DIR")),
            watch_for_changes_override: open_arpg_watch_for_changes_override(),
            mode: config.asset_mode.bevy_mode(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: open_arpg_primary_window(config.headless_smoke),
            exit_condition: if config.headless_smoke {
                ExitCondition::DontExit
            } else {
                ExitCondition::OnAllClosed
            },
            ..default()
        });
    // On web, Bevy's default WebGPU render setup is the reliable path; the
    // profile-driven WgpuSettings overrides are for native backend debugging.
    #[cfg(not(target_arch = "wasm32"))]
    {
        default_plugins = default_plugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(open_arpg_wgpu_settings(
                config.render_profile,
            ))),
            ..default()
        });
    }
    if !config.audio_enabled {
        default_plugins = default_plugins.disable::<AudioPlugin>();
    }
    if config.headless_smoke {
        #[cfg(not(target_arch = "wasm32"))]
        {
            default_plugins = default_plugins.disable::<PipelinedRenderingPlugin>();
        }
        default_plugins = default_plugins
            .disable::<WinitPlugin>()
            .disable::<GilrsPlugin>()
            .disable::<RenderPlugin>()
            .disable::<CorePipelinePlugin>()
            .disable::<SpriteRenderPlugin>()
            .disable::<UiRenderPlugin>()
            .disable::<GizmoRenderPlugin>()
            .disable::<PbrPlugin>()
            .disable::<PostProcessPlugin>()
            .disable::<AntiAliasPlugin>();
        app.insert_resource(CompressedImageFormatSupport(CompressedImageFormats::NONE));
    }
    app.insert_resource(ClearColor(Color::srgb(0.015, 0.013, 0.018)))
        .insert_resource(RunStats::default())
        .insert_resource(DifficultySettings::default())
        .insert_resource(ChapterModifier::default())
        .insert_resource(PauseState::default())
        .insert_resource(InventoryOpen::default())
        .insert_resource(BuildOpen::default())
        .insert_resource(JournalOpen::default())
        .insert_resource(OpenArpgUserSettings::from_config(config))
        .insert_resource(OpenArpgPreferenceLocks {
            audio_enabled: config.audio_locked,
            debug_visuals: config.debug_visuals_locked,
        })
        .insert_resource(DebugVisuals {
            enabled: config.debug_visuals,
        })
        .insert_resource(AudioSettings {
            enabled: config.audio_enabled,
        })
        .insert_resource(PickingSettings {
            is_window_picking_enabled: false,
            multi_click_interval: Duration::from_millis(350),
            ..default()
        })
        .insert_resource(MeshPickingSettings {
            require_markers: false,
            ..default()
        })
        .add_plugins(default_plugins);
    app.add_plugins(SettingsPlugin::new("org.stars-labs.bevy-open-arpg"));
    #[cfg(not(target_arch = "wasm32"))]
    if config.remote_enabled {
        app.add_plugins((RemotePlugin::default(), RemoteHttpPlugin::default()));
    }
    #[cfg(feature = "dev_tools")]
    {
        app.insert_resource(DebugFreeCamera {
            enabled: config.free_camera_enabled,
        });
        if !config.headless_smoke {
            app.add_plugins(FreeCameraPlugin);
        }
    }
    #[cfg(feature = "dev_tools")]
    if config.diagnostics_enabled {
        app.add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            SystemInformationDiagnosticsPlugin,
        ));
        if !config.headless_smoke {
            app.add_plugins(RenderDiagnosticsPlugin);
        }
    }
    #[cfg(feature = "dev_tools")]
    {
        app.add_plugins(DebugPickingPlugin)
            .insert_resource(DebugPickingMode::Disabled)
            .add_systems(PreUpdate, cycle_debug_picking);
    }
    if config.headless_smoke {
        init_headless_smoke_assets(&mut app);
    }
    if config.headless_smoke {
        app.insert_resource(HeadlessSmokeExit {
            frames_remaining: config.smoke_frames,
        })
        .set_runner(headless_smoke_runner);
    }
    app.init_state::<GameState>().add_plugins((
        GameDataPlugin,
        GameAssetsPlugin,
        BestiaryPlugin,
        BountyPlugin,
        ChallengePlugin,
        ChapterPlugin,
        DungeonPlugin,
        PlayerPlugin,
        CompanionPlugin,
    ));
    app.add_plugins((
        EnemyPlugin,
        CombatPlugin,
        FeedbackPlugin,
        RiftPlugin,
        LorePlugin,
        StoryPlugin,
        MasteryPlugin,
        MilestonePlugin,
        ObeliskPlugin,
        LootPlugin,
        SavePlugin,
        HudPlugin,
    ));
    app.add_systems(
        Startup,
        (apply_loaded_user_settings, setup_camera_and_light).chain(),
    )
    .add_systems(Update, toggle_audio)
    .add_systems(Update, toggle_debug_visuals)
    .add_systems(Update, toggle_ui_debug_overlay)
    .add_systems(Update, copy_runtime_summary_to_clipboard)
    .add_systems(Update, save_user_settings_on_close)
    .add_systems(
        Update,
        main_menu_input.run_if(in_state(GameState::MainMenu)),
    )
    .add_systems(
        Update,
        game_over_input.run_if(in_state(GameState::GameOver)),
    )
    .add_systems(Update, victory_input.run_if(in_state(GameState::Victory)))
    .add_systems(Update, toggle_pause.run_if(in_state(GameState::InGame)))
    .add_systems(Update, toggle_inventory.run_if(in_state(GameState::InGame)))
    .add_systems(Update, toggle_build.run_if(in_state(GameState::InGame)))
    .add_systems(Update, toggle_journal.run_if(in_state(GameState::InGame)))
    .add_systems(
        Update,
        tick_run_stats.run_if(in_state(GameState::InGame).and_then(not_paused)),
    )
    .add_systems(
        Update,
        update_ashen_threat.run_if(in_state(GameState::InGame).and_then(not_paused)),
    )
    .add_systems(
        Update,
        draw_debug_gizmos.run_if(in_state(GameState::InGame)),
    );
    app
}

fn init_headless_smoke_assets(app: &mut App) {
    app.init_asset::<Shader>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<ScatteringMedium>();
}

fn open_arpg_watch_for_changes_override() -> Option<bool> {
    #[cfg(feature = "dev_tools")]
    {
        Some(true)
    }
    #[cfg(not(feature = "dev_tools"))]
    {
        None
    }
}

fn open_arpg_primary_window(headless_smoke: bool) -> Option<Window> {
    if headless_smoke {
        None
    } else {
        Some(Window {
            title: OPEN_ARPG_WINDOW_TITLE.to_string(),
            resolution: (OPEN_ARPG_WINDOW_WIDTH, OPEN_ARPG_WINDOW_HEIGHT).into(),
            present_mode: bevy::window::PresentMode::AutoVsync,
            visible: true,
            decorations: true,
            resizable: true,
            // Render into the loader page's canvas and track the browser
            // viewport instead of opening a fixed-size surface.
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#bevy-canvas".to_string()),
            #[cfg(target_arch = "wasm32")]
            fit_canvas_to_parent: true,
            ..default()
        })
    }
}

fn headless_smoke_runner(mut app: App) -> AppExit {
    app.finish();
    app.cleanup();

    let mut frames_remaining = app
        .world_mut()
        .remove_resource::<HeadlessSmokeExit>()
        .map(|resource| resource.frames_remaining)
        .unwrap_or_default();

    loop {
        if frames_remaining == 0 {
            return AppExit::Success;
        }

        app.update();

        if let Some(exit) = app.should_exit() {
            return exit;
        }

        frames_remaining = frames_remaining.saturating_sub(1);
    }
}

fn apply_loaded_user_settings(
    settings: Res<OpenArpgUserSettings>,
    locks: Res<OpenArpgPreferenceLocks>,
    mut audio: ResMut<AudioSettings>,
    mut debug: ResMut<DebugVisuals>,
) {
    if !locks.audio_enabled {
        audio.enabled = settings.audio_enabled;
    }
    if !locks.debug_visuals {
        debug.enabled = settings.debug_visuals;
    }
}

fn save_user_settings_on_close(
    mut close_events: MessageReader<WindowCloseRequested>,
    mut commands: Commands,
) {
    if close_events.read().next().is_some() {
        commands.queue(SaveSettingsSync::IfChanged);
    }
}

fn open_arpg_audio_enabled_from_env() -> Option<bool> {
    std::env::var("BEVY_OPEN_ARPG_AUDIO").ok().map(|value| {
        !matches!(
            value.trim(),
            "0" | "false" | "FALSE" | "off" | "OFF" | "no" | "NO"
        )
    })
}

fn open_arpg_display_is_present() -> bool {
    // The browser canvas is always available on web; native display checks are
    // required only for desktop runs.
    if cfg!(target_arch = "wasm32") {
        return true;
    }

    let display_present = env_value_present(std::env::var("DISPLAY").ok().as_deref());
    let wayland_display_present =
        env_value_present(std::env::var("WAYLAND_DISPLAY").ok().as_deref());
    let wayland_socket_present = env_value_present(std::env::var("WAYLAND_SOCKET").ok().as_deref());

    display_present
        || wayland_display_present
        || wayland_socket_present
        || open_arpg_has_x11_socket_server()
        || open_arpg_has_wayland_socket_server()
}

fn open_arpg_has_x11_socket_server() -> bool {
    has_socket_with_prefix(std::path::Path::new("/tmp/.X11-unix"), "X")
}

fn open_arpg_has_wayland_socket_server() -> bool {
    if let Some(display) = std::env::var("WAYLAND_DISPLAY")
        .ok()
        .map(|value| value.trim().to_string())
    {
        if !display.is_empty() {
            let maybe_path = std::path::Path::new(&display);
            if maybe_path.is_absolute() && maybe_path.exists() {
                return true;
            }

            if let Some(xdg_runtime) = env_value_present_path("XDG_RUNTIME_DIR") {
                if xdg_runtime.join(&display).exists() {
                    return true;
                }
            }

            if let Some(uid_runtime) = uid_runtime_path()
                && uid_runtime.join(&display).exists()
            {
                return true;
            }
        }
    }

    if let Some(xdg_runtime) = env_value_present_path("XDG_RUNTIME_DIR") {
        if has_socket_with_prefix(&xdg_runtime, "wayland-") {
            return true;
        }
    }

    if let Some(uid_runtime) = uid_runtime_path() {
        if has_socket_with_prefix(&uid_runtime, "wayland-") {
            return true;
        }
    }

    has_socket_with_prefix(std::path::Path::new("/tmp"), "wayland-")
}

fn uid_runtime_path() -> Option<std::path::PathBuf> {
    std::env::var("UID")
        .ok()
        .filter(|uid| !uid.trim().is_empty())
        .map(|uid| {
            let base = format!("/run/user/{uid}");
            std::path::PathBuf::from(base)
        })
}

fn env_value_present_path(name: &str) -> Option<std::path::PathBuf> {
    let Some(value) = std::env::var(name).ok() else {
        return None;
    };
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.into())
}

fn has_socket_with_prefix(dir: &std::path::Path, prefix: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    entries
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
}

fn parse_open_arpg_cli_args<I, S>(args: I) -> OpenArpgCliArgs
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut parsed = OpenArpgCliArgs::default();
    for arg in args {
        parse_open_arpg_cli_arg(arg.as_ref(), &mut parsed);
    }
    parsed
}

fn parse_open_arpg_cli_arg(arg: &str, parsed: &mut OpenArpgCliArgs) {
    match arg.trim() {
        "--audio" => parsed.audio_enabled = Some(true),
        "--no-audio" | "--mute" => parsed.audio_enabled = Some(false),
        "--debug-gizmos" => parsed.debug_visuals = Some(true),
        "--no-debug-gizmos" => parsed.debug_visuals = Some(false),
        #[cfg(feature = "dev_tools")]
        "--diagnostics" => parsed.diagnostics_enabled = Some(true),
        #[cfg(feature = "dev_tools")]
        "--no-diagnostics" => parsed.diagnostics_enabled = Some(false),
        #[cfg(feature = "dev_tools")]
        "--free-camera" => parsed.free_camera_enabled = Some(true),
        #[cfg(feature = "dev_tools")]
        "--no-free-camera" => parsed.free_camera_enabled = Some(false),
        "--headless-smoke" => parsed.headless_smoke = Some(true),
        "--windowed" => parsed.headless_smoke = Some(false),
        "--remote" => parsed.remote_enabled = Some(true),
        "--no-remote" => parsed.remote_enabled = Some(false),
        "--x11" => parsed.window_backend = Some(OpenArpgWindowBackend::X11),
        "--wayland" => parsed.window_backend = Some(OpenArpgWindowBackend::Wayland),
        value => {
            if let Some(value) = value.strip_prefix("--smoke-frames=") {
                parsed.smoke_frames = Some(parse_open_arpg_smoke_frames(Some(value)));
            } else if let Some(value) = value.strip_prefix("--asset-mode=") {
                parsed.asset_mode = Some(parse_open_arpg_asset_mode(Some(value)));
            } else if let Some(value) = value.strip_prefix("--render-profile=") {
                parsed.render_profile = parse_open_arpg_render_profile(Some(value));
            }
        }
    }
}

fn open_arpg_env_flag(name: &str) -> bool {
    open_arpg_env_flag_value(name).unwrap_or(false)
}

fn open_arpg_env_flag_value(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|value| {
        matches!(
            value.trim(),
            "1" | "true" | "TRUE" | "on" | "ON" | "yes" | "YES"
        )
    })
}

fn open_arpg_smoke_frames() -> u32 {
    parse_open_arpg_smoke_frames(std::env::var("BEVY_OPEN_ARPG_SMOKE_FRAMES").ok().as_deref())
}

fn open_arpg_asset_mode() -> OpenArpgAssetMode {
    parse_open_arpg_asset_mode(std::env::var("BEVY_OPEN_ARPG_ASSET_MODE").ok().as_deref())
}

fn open_arpg_render_profile() -> Option<OpenArpgRenderProfile> {
    parse_open_arpg_render_profile(
        std::env::var("BEVY_OPEN_ARPG_RENDER_PROFILE")
            .ok()
            .as_deref(),
    )
}

fn parse_open_arpg_asset_mode(value: Option<&str>) -> OpenArpgAssetMode {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("processed" | "processor" | "imported" | "imported_assets") => {
            OpenArpgAssetMode::Processed
        }
        _ => OpenArpgAssetMode::Unprocessed,
    }
}

fn parse_open_arpg_render_profile(value: Option<&str>) -> Option<OpenArpgRenderProfile> {
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("compat" | "compatibility" | "stable" | "safe" | "conservative") => {
            Some(OpenArpgRenderProfile::Compatibility)
        }
        Some("functionality" | "full" | "high") => Some(OpenArpgRenderProfile::Functionality),
        Some("webgl2" | "webgl") => Some(OpenArpgRenderProfile::WebGl2),
        Some("gl" | "gles" | "opengl") => Some(OpenArpgRenderProfile::Gl),
        _ => None,
    }
}

fn parse_open_arpg_smoke_frames(value: Option<&str>) -> u32 {
    value
        .and_then(|value| value.trim().parse::<u32>().ok())
        .map(|frames| frames.clamp(1, 600))
        .unwrap_or(12)
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_process_env_overrides(config: OpenArpgRuntimeConfig) {
    let Some(window_backend) = config.window_backend else {
        return;
    };
    // This happens before Bevy starts Winit or worker threads, so the process env override is scoped to startup.
    unsafe {
        std::env::set_var("WINIT_UNIX_BACKEND", window_backend.winit_value());
    }
}

#[cfg(target_arch = "wasm32")]
fn apply_process_env_overrides(_config: OpenArpgRuntimeConfig) {}

fn log_startup_window_diagnostic(config: OpenArpgRuntimeConfig) {
    let wayland_socket = std::env::var("WAYLAND_SOCKET").ok();
    let display_env_present = display_vars_present(
        std::env::var("DISPLAY").ok().as_deref(),
        std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
        wayland_socket.as_deref(),
    );
    eprintln!(
        "{}",
        startup_window_diagnostic(
            config,
            std::env::var("DISPLAY").ok().as_deref(),
            std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
            std::env::var("WINIT_UNIX_BACKEND").ok().as_deref(),
            display_env_present,
            wayland_socket.as_deref(),
        )
    );
}

fn display_vars_present(
    display: Option<&str>,
    wayland_display: Option<&str>,
    wayland_socket: Option<&str>,
) -> bool {
    env_value_present(display)
        || env_value_present(wayland_display)
        || env_value_present(wayland_socket)
}

fn env_value_present(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn startup_window_diagnostic(
    config: OpenArpgRuntimeConfig,
    display: Option<&str>,
    wayland_display: Option<&str>,
    winit_backend: Option<&str>,
    display_present: bool,
    wayland_socket: Option<&str>,
) -> String {
    let display = display_server_label(display, wayland_display, wayland_socket);
    let backend = startup_window_backend_label(config.window_backend, winit_backend);
    let render = config
        .render_profile
        .map(OpenArpgRenderProfile::label)
        .unwrap_or("auto");
    let audio = if config.audio_enabled { "on" } else { "off" };
    let mut message = if config.headless_smoke {
        format!(
            "Bevy Open ARPG startup: window=disabled(headless-smoke) display={display} backend={backend} render={render} audio={audio} asset_mode={}. If you expected a popup, run `cargo run -- --windowed` or unset BEVY_OPEN_ARPG_HEADLESS_SMOKE.",
            config.asset_mode.label()
        )
    } else {
        format!(
            "Bevy Open ARPG startup: window=enabled title=\"{}\" size={}x{} display={display} backend={backend} render={render} audio={audio} asset_mode={}.",
            OPEN_ARPG_WINDOW_TITLE,
            OPEN_ARPG_WINDOW_WIDTH,
            OPEN_ARPG_WINDOW_HEIGHT,
            config.asset_mode.label()
        )
    };

    if !config.headless_smoke {
        if display_present {
            message.push_str(
                " If no window appears, retry `cargo run -- --x11 --render-profile=compat` or `cargo run -- --wayland --render-profile=compat`.",
            );
        } else {
            message.push_str(
                " A graphical display server was not detected, but windowed mode was requested. Verify `DISPLAY`/`WAYLAND_DISPLAY` in your shell launch context, then retry with a matching backend switch (`--x11` or `--wayland`) or unset `--headless-smoke` overrides.",
            );
        }
    }
    message.push_str(&format!(
        " Feature audit: {}.",
        bevy_runtime_feature_audit_summary()
    ));

    message
}

fn display_server_label(
    display: Option<&str>,
    wayland_display: Option<&str>,
    wayland_socket: Option<&str>,
) -> &'static str {
    match (
        env_value_present(display),
        env_value_present(wayland_display),
        env_value_present(wayland_socket),
    ) {
        (true, false, false) => "x11",
        (true, true, false) => "x11+wayland",
        (true, true, true) => "x11+wayland+socket",
        (true, false, true) => "x11+socket",
        (false, true, false) => "wayland",
        (false, true, true) => "wayland+socket",
        (false, false, true) => "socket",
        (false, false, false) => "none",
    }
}

fn startup_window_backend_label(
    explicit_backend: Option<OpenArpgWindowBackend>,
    winit_backend: Option<&str>,
) -> String {
    if let Some(backend) = explicit_backend {
        return format!("{}(forced)", backend.label());
    }
    match winit_backend
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(backend) => format!("{backend}(env)"),
        None => "auto".to_string(),
    }
}

#[cfg(feature = "dev_tools")]
fn cycle_debug_picking(keyboard: Res<ButtonInput<KeyCode>>, mut mode: ResMut<DebugPickingMode>) {
    if keyboard.just_pressed(KeyCode::F4) {
        *mode = match *mode {
            DebugPickingMode::Disabled => DebugPickingMode::Normal,
            DebugPickingMode::Normal => DebugPickingMode::Noisy,
            DebugPickingMode::Noisy => DebugPickingMode::Disabled,
        };
    }
}

fn open_arpg_wgpu_settings(profile: Option<OpenArpgRenderProfile>) -> WgpuSettings {
    let mut settings = WgpuSettings {
        features: WgpuFeatures::empty(),
        instance_flags: InstanceFlags::empty(),
        ..default()
    };

    if let Some(profile) = render_settings_from_profile(profile) {
        settings.priority = profile.priority;
        if let Some(backends) = profile.backends {
            settings.backends = Some(backends);
        }
    } else if std::env::var_os("WGPU_SETTINGS_PRIO").is_none() {
        settings.priority = WgpuSettingsPriority::WebGPU;
    }

    settings
}

struct OpenArpgRenderSettings {
    priority: WgpuSettingsPriority,
    backends: Option<Backends>,
}

fn render_settings_from_profile(
    profile: Option<OpenArpgRenderProfile>,
) -> Option<OpenArpgRenderSettings> {
    match profile? {
        OpenArpgRenderProfile::Compatibility => Some(OpenArpgRenderSettings {
            priority: WgpuSettingsPriority::WebGPU,
            backends: None,
        }),
        OpenArpgRenderProfile::Functionality => Some(OpenArpgRenderSettings {
            priority: WgpuSettingsPriority::Functionality,
            backends: None,
        }),
        OpenArpgRenderProfile::WebGl2 => Some(OpenArpgRenderSettings {
            priority: WgpuSettingsPriority::WebGL2,
            backends: None,
        }),
        OpenArpgRenderProfile::Gl => Some(OpenArpgRenderSettings {
            priority: WgpuSettingsPriority::WebGL2,
            backends: Some(Backends::GL),
        }),
    }
}

fn setup_camera_and_light(
    mut commands: Commands,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    #[cfg(feature = "dev_tools")] free_camera: Res<DebugFreeCamera>,
) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.16, 0.12, 0.20),
        brightness: 90.0,
        affects_lightmapped_meshes: true,
    });

    let atmosphere_medium = scattering_mediums.add(
        ScatteringMedium::earth(128, 128)
            .with_density_multiplier(1.65)
            .with_label("ashen_reliquary_atmosphere"),
    );
    commands.spawn((
        Atmosphere::earth(atmosphere_medium),
        Name::new("Ashen Reliquary Atmosphere"),
    ));

    let mut camera = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-9.0, 12.0, 13.0).looking_at(Vec3::ZERO, Vec3::Y),
        // UI composites on this camera directly; a second Camera2d overlay
        // left a stale menu frame in its pooled view target on WebGPU.
        IsDefaultUiCamera,
        Name::new("Isometric Camera"),
    ));
    camera.insert((
        Msaa::Off,
        Hdr,
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Smaa {
            preset: SmaaPreset::High,
        },
        ShadowFilteringMethod::Gaussian,
        MeshPickingCamera,
        DepthPrepass,
    ));
    camera.insert((
        ContactShadows::default(),
        ScreenSpaceAmbientOcclusion::default(),
        open_arpg_atmosphere_settings(),
        open_arpg_atmosphere_environment_light(),
    ));
    camera.insert((
        DistanceFog {
            color: Color::srgba(0.05, 0.035, 0.075, 0.62),
            directional_light_color: Color::srgba(0.72, 0.34, 0.18, 0.24),
            directional_light_exponent: 28.0,
            falloff: FogFalloff::Linear {
                start: 13.0,
                end: 38.0,
            },
        },
        open_arpg_depth_of_field(),
        open_arpg_vignette(),
        open_arpg_chromatic_aberration(),
    ));
    // Volumetric fog renders a solid black frame on WebGPU (bisected via
    // headless Chrome); the web build keeps distance fog only.
    #[cfg(not(target_arch = "wasm32"))]
    camera.insert(VolumetricFog {
        ambient_color: Color::srgb(0.34, 0.18, 0.12),
        ambient_intensity: 0.025,
        jitter: 0.35,
        step_count: 48,
    });
    #[cfg(feature = "dev_tools")]
    if free_camera.enabled {
        camera.insert(open_arpg_free_camera());
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            // PCSS is native-only (its WGSL breaks on WebGPU); the field only
            // exists when bevy/experimental_pbr_pcss is enabled via `native`.
            #[cfg(feature = "native")]
            soft_shadow_size: Some(1.85),
            ..default()
        },
        VolumetricLight,
        Transform::from_xyz(-5.0, 8.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("Moon Key Light"),
    ));

    commands.spawn((
        PointLight {
            intensity: 900.0,
            range: 18.0,
            radius: 1.1,
            color: Color::srgb(1.0, 0.42, 0.16),
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            #[cfg(feature = "native")]
            soft_shadows_enabled: true,
            ..default()
        },
        VolumetricLight,
        Transform::from_xyz(0.0, 4.0, -3.0),
        Name::new("Infernal Fill Light"),
    ));

    commands.spawn((
        RectLight {
            color: Color::srgb(1.0, 0.26, 0.08),
            intensity: 95_000.0,
            width: 4.2,
            height: 1.4,
            range: 22.0,
        },
        Transform::from_xyz(-5.5, 3.3, -5.8).looking_at(Vec3::new(-1.0, 0.7, -1.2), Vec3::Y),
        Name::new("Reliquary Brazier Area Light"),
    ));

    commands.spawn((
        RectLight {
            color: Color::srgb(0.36, 0.55, 1.0),
            intensity: 52_000.0,
            width: 2.6,
            height: 5.2,
            range: 20.0,
        },
        Transform::from_xyz(6.4, 3.6, 4.8).looking_at(Vec3::new(1.5, 0.8, 1.2), Vec3::Y),
        Name::new("Moonlit Rift Area Light"),
    ));

    commands.spawn((
        FogVolume {
            fog_color: Color::srgb(0.36, 0.18, 0.12),
            density_factor: 0.014,
            ..default()
        },
        Transform::from_xyz(0.0, 2.8, 0.0).with_scale(Vec3::new(34.0, 7.0, 24.0)),
        Name::new("Low Reliquary Volumetric Haze"),
    ));
}

fn open_arpg_depth_of_field() -> DepthOfField {
    DepthOfField {
        mode: DepthOfFieldMode::Gaussian,
        focal_distance: 16.0,
        aperture_f_stops: 7.2,
        max_circle_of_confusion_diameter: 10.0,
        max_depth: 42.0,
        ..default()
    }
}

fn open_arpg_vignette() -> Vignette {
    Vignette {
        intensity: 0.32,
        radius: 0.92,
        smoothness: 3.4,
        roundness: 0.88,
        color: Color::srgb(0.025, 0.012, 0.035),
        edge_compensation: 0.82,
        center: Vec2::new(0.5, 0.53),
    }
}

fn open_arpg_chromatic_aberration() -> ChromaticAberration {
    ChromaticAberration {
        color_lut: None,
        intensity: 0.004,
        max_samples: 4,
    }
}

fn open_arpg_atmosphere_settings() -> AtmosphereSettings {
    AtmosphereSettings {
        transmittance_lut_size: UVec2::new(128, 64),
        multiscattering_lut_size: UVec2::new(32, 32),
        sky_view_lut_size: UVec2::new(256, 128),
        aerial_view_lut_size: UVec3::new(16, 16, 16),
        aerial_view_lut_max_distance: 30_000.0,
        sky_max_samples: 12,
        rendering_method: AtmosphereMode::LookupTexture,
        ..default()
    }
}

fn open_arpg_atmosphere_environment_light() -> AtmosphereEnvironmentMapLight {
    AtmosphereEnvironmentMapLight {
        intensity: 0.16,
        size: UVec2::new(256, 256),
        ..default()
    }
}

#[cfg(feature = "dev_tools")]
fn open_arpg_free_camera() -> FreeCamera {
    FreeCamera {
        key_forward: KeyCode::ArrowUp,
        key_back: KeyCode::ArrowDown,
        key_left: KeyCode::ArrowLeft,
        key_right: KeyCode::ArrowRight,
        key_up: KeyCode::PageUp,
        key_down: KeyCode::PageDown,
        key_run: KeyCode::ShiftRight,
        keyboard_key_toggle_cursor_grab: KeyCode::F8,
        mouse_key_cursor_grab: MouseButton::Right,
        walk_speed: 8.0,
        run_speed: 28.0,
        ..default()
    }
}

fn gamepad_just_pressed(gamepads: &Query<&Gamepad>, buttons: &[GamepadButton]) -> bool {
    gamepads
        .iter()
        .any(|gamepad| buttons.iter().any(|button| gamepad.just_pressed(*button)))
}

#[allow(clippy::too_many_arguments)]
fn main_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut next_state: ResMut<NextState<GameState>>,
    mut stats: ResMut<RunStats>,
    mut difficulty: ResMut<DifficultySettings>,
    mut modifier: ResMut<ChapterModifier>,
    mut pending_load: ResMut<PendingLoadGame>,
    records: Res<ChapterRecords>,
) {
    if keyboard.just_pressed(KeyCode::Tab)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::DPadRight])
    {
        difficulty.current = next_unlocked_difficulty(difficulty.current, &records);
    }
    if keyboard.just_pressed(KeyCode::F9)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::Select])
    {
        pending_load.request();
        next_state.set(GameState::InGame);
        return;
    }
    if keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::Enter)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::South, GamepadButton::Start])
    {
        reset_run_stats_for_new_chapter(&mut stats);
        *modifier = modifier_for_run(difficulty.current, records.clears(difficulty.current));
        next_state.set(GameState::InGame);
    }
}

fn game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut next_state: ResMut<NextState<GameState>>,
    mut stats: ResMut<RunStats>,
    difficulty: Res<DifficultySettings>,
    mut modifier: ResMut<ChapterModifier>,
    records: Res<ChapterRecords>,
) {
    if keyboard.just_pressed(KeyCode::KeyR)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::South, GamepadButton::Start])
    {
        reset_run_stats_for_new_chapter(&mut stats);
        *modifier = modifier_for_run(difficulty.current, records.clears(difficulty.current));
        next_state.set(GameState::InGame);
    }
}

fn victory_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut next_state: ResMut<NextState<GameState>>,
    mut stats: ResMut<RunStats>,
    mut difficulty: ResMut<DifficultySettings>,
    mut modifier: ResMut<ChapterModifier>,
    records: Res<ChapterRecords>,
) {
    if keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::Enter)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::South, GamepadButton::Start])
    {
        difficulty.current = victory_next_difficulty_after_clear(difficulty.current, &records);
        reset_run_stats_for_new_chapter(&mut stats);
        *modifier = modifier_for_run(difficulty.current, records.clears(difficulty.current));
        next_state.set(GameState::InGame);
        return;
    }
    if keyboard.just_pressed(KeyCode::KeyR)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::West])
    {
        reset_run_stats_for_new_chapter(&mut stats);
        *modifier = modifier_for_run(difficulty.current, records.clears(difficulty.current));
        next_state.set(GameState::InGame);
    }
}

fn victory_next_difficulty_after_clear(
    current: Difficulty,
    records: &ChapterRecords,
) -> Difficulty {
    if current == Difficulty::Torment {
        Difficulty::Torment
    } else {
        next_unlocked_difficulty(current, records)
    }
}

fn reset_run_stats_for_new_chapter(stats: &mut RunStats) {
    let carried_soul_sigils = stats.malrec_soul_sigils;
    *stats = RunStats {
        malrec_soul_sigils: carried_soul_sigils,
        ..Default::default()
    };
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut pause: ResMut<PauseState>,
) {
    if keyboard.just_pressed(KeyCode::Escape)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::Start])
    {
        pause.paused = !pause.paused;
    }
}

fn toggle_audio(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut audio: ResMut<AudioSettings>,
    mut settings: ResMut<OpenArpgUserSettings>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::KeyM)
        || gamepad_just_pressed(&gamepads, &[GamepadButton::North])
    {
        audio.toggle();
        settings.audio_enabled = audio.enabled;
        commands.queue(SaveSettingsDeferred(Duration::from_millis(250)));
    }
}

fn toggle_debug_visuals(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug: ResMut<DebugVisuals>,
    mut settings: ResMut<OpenArpgUserSettings>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        debug.enabled = !debug.enabled;
        settings.debug_visuals = debug.enabled;
        commands.queue(SaveSettingsDeferred(Duration::from_millis(250)));
    }
}

fn toggle_ui_debug_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    options: Option<ResMut<GlobalUiDebugOptions>>,
) {
    if !keyboard.just_pressed(KeyCode::F7) {
        return;
    }

    let Some(mut options) = options else {
        warn!("Bevy UI debug overlay is unavailable because UI rendering is disabled");
        return;
    };

    let enabled = !options.enabled;
    apply_open_arpg_ui_debug_overlay(&mut options, enabled);
    info!(
        "Bevy UI debug overlay {}",
        ui_debug_overlay_status(Some(enabled))
    );
}

fn apply_open_arpg_ui_debug_overlay(options: &mut GlobalUiDebugOptions, enabled: bool) {
    options.enabled = enabled;
    options.outline_border_box = true;
    options.outline_padding_box = enabled;
    options.outline_content_box = enabled;
    options.outline_scrollbars = enabled;
    options.show_clipped = enabled;
    options.show_hidden = false;
    options.ignore_border_radius = enabled;
    options.line_width = if enabled { 2.0 } else { 1.0 };
}

fn ui_debug_overlay_status(enabled: Option<bool>) -> &'static str {
    match enabled {
        Some(true) => "on",
        Some(false) => "off",
        None => "unavailable",
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_runtime_summary_to_clipboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    difficulty: Res<DifficultySettings>,
    stats: Res<RunStats>,
    audio: Res<AudioSettings>,
    debug: Res<DebugVisuals>,
    settings: Res<OpenArpgUserSettings>,
    ui_debug: Option<Res<GlobalUiDebugOptions>>,
    mut clipboard: ResMut<Clipboard>,
) {
    if !keyboard.just_pressed(KeyCode::F6) {
        return;
    }

    let summary = runtime_clipboard_summary(
        *state.get(),
        difficulty.current,
        &stats,
        &audio,
        &debug,
        &settings,
        ui_debug.as_deref().map(|options| options.enabled),
    );
    match clipboard.set_text(summary) {
        Ok(()) => info!("Copied Bevy Open ARPG runtime summary to clipboard"),
        Err(err) => warn!("Failed to copy Bevy Open ARPG runtime summary to clipboard: {err}"),
    }
}

fn runtime_clipboard_summary(
    state: GameState,
    difficulty: Difficulty,
    stats: &RunStats,
    audio: &AudioSettings,
    debug: &DebugVisuals,
    settings: &OpenArpgUserSettings,
    ui_debug_overlay_enabled: Option<bool>,
) -> String {
    format!(
        "Bevy Open ARPG\n\
         state={}\n\
         difficulty={}\n\
         elapsed={:.1}s\n\
         kills={}\n\
         gold={}\n\
         ember_shards={}\n\
         affix_essence={}\n\
         journey_score={}\n\
         renown_rank={}\n\
         ashen_threat={}/{}\n\
         audio={}\n\
         debug_gizmos={}\n\
         ui_debug_overlay={}\n\
         remote={}\n\
         asset_hot_reload={}\n\
         asset_mode={}\n\
         render_profile={}\n\
         bevy_feature_audit={}",
        state.label(),
        difficulty.label(),
        stats.elapsed_secs,
        stats.kills,
        stats.gold,
        stats.ember_shards,
        stats.affix_essence,
        stats.journey_score,
        stats.renown_rank,
        stats.ashen_threat,
        ASHEN_THREAT_MAX,
        audio.status_label(),
        if debug.enabled { "on" } else { "off" },
        ui_debug_overlay_status(ui_debug_overlay_enabled),
        if settings.remote_enabled { "on" } else { "off" },
        open_arpg_asset_hot_reload_status(),
        settings.asset_mode,
        settings.render_profile,
        bevy_runtime_feature_audit_summary(),
    )
}

fn bevy_runtime_feature_audit_summary() -> String {
    let (documented_fields, total_fields) = reflected_user_settings_doc_counts();
    format!(
        "profiles=3d+ui+scene+picking+audio-all-formats; animation=gltf_animation+morph_animation; ui=bevy_ui+widgets+input-focus; assets=processor+gltf+ktx2+basis+webp+hdr+audio-codecs; render=hdr+bloom+smaa+dof+fog+area-lights+pcss+contact-shadows; post_process=hdr+bloom+smaa+dof+fog; picking=mesh+ui; platform=x11+wayland+accesskit+gamepad+clipboard+system-fonts; tools=settings+remote-opt-in+ui-debug+reflect_docs={documented_fields}/{total_fields} settings fields+reflect_functions={} helpers+settings=on; omitted=http/https+solari+dlss+meshlet+hotpatching+zstd_c; deferred=feathers+clipboard_image+pan_camera; dev_tools={}",
        reflected_tool_helper_count(),
        open_arpg_asset_hot_reload_status()
    )
}

fn reflected_user_settings_doc_counts() -> (usize, usize) {
    match <OpenArpgUserSettings as Typed>::type_info() {
        TypeInfo::Struct(info) => {
            let documented = info
                .iter()
                .filter(|field| field.docs().is_some_and(|docs| !docs.trim().is_empty()))
                .count();
            (documented, info.field_len())
        }
        _ => (0, 0),
    }
}

fn reflected_tool_helper_count() -> usize {
    let mut registry = FunctionRegistry::default();
    if registry
        .register_with_name(
            "open_arpg_render_profile_count",
            open_arpg_render_profile_count,
        )
        .is_err()
    {
        return 0;
    }

    let Some(function) = registry.get("open_arpg_render_profile_count") else {
        return 0;
    };
    let Ok(result) = function.call(ArgList::new()) else {
        return 0;
    };
    result
        .unwrap_owned()
        .try_downcast_ref::<usize>()
        .copied()
        .unwrap_or(0)
}

fn open_arpg_render_profile_count() -> usize {
    [
        OpenArpgRenderProfile::Compatibility,
        OpenArpgRenderProfile::Functionality,
        OpenArpgRenderProfile::WebGl2,
        OpenArpgRenderProfile::Gl,
    ]
    .len()
}

fn open_arpg_asset_hot_reload_status() -> &'static str {
    #[cfg(feature = "dev_tools")]
    {
        "on"
    }
    #[cfg(not(feature = "dev_tools"))]
    {
        "off"
    }
}

fn draw_debug_gizmos(
    debug: Res<DebugVisuals>,
    mut gizmos: Gizmos,
    players: Query<&Transform, With<Player>>,
) {
    if !debug.enabled {
        return;
    }

    let floor_y = 0.08;
    let min_x = -13.0;
    let max_x = 13.0;
    let min_z = -9.0;
    let max_z = 9.0;
    let a = Vec3::new(min_x, floor_y, min_z);
    let b = Vec3::new(max_x, floor_y, min_z);
    let c = Vec3::new(max_x, floor_y, max_z);
    let d = Vec3::new(min_x, floor_y, max_z);
    let arena_color = Color::srgba(0.34, 0.72, 1.0, 0.75);
    gizmos.line(a, b, arena_color);
    gizmos.line(b, c, arena_color);
    gizmos.line(c, d, arena_color);
    gizmos.line(d, a, arena_color);

    for transform in &players {
        let center = transform.translation + Vec3::Y * 0.35;
        let player_color = Color::srgb(1.0, 0.82, 0.24);
        gizmos.line(
            center + Vec3::new(-0.85, 0.0, 0.0),
            center + Vec3::new(0.85, 0.0, 0.0),
            player_color,
        );
        gizmos.line(
            center + Vec3::new(0.0, 0.0, -0.85),
            center + Vec3::new(0.0, 0.0, 0.85),
            player_color,
        );
        gizmos.circle(
            Isometry3d::new(center, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            1.15,
            player_color,
        );
    }
}

fn toggle_inventory(keyboard: Res<ButtonInput<KeyCode>>, mut inventory: ResMut<InventoryOpen>) {
    if keyboard.just_pressed(KeyCode::KeyI) {
        inventory.open = !inventory.open;
    }
}

fn toggle_build(keyboard: Res<ButtonInput<KeyCode>>, mut build: ResMut<BuildOpen>) {
    if keyboard.just_pressed(KeyCode::KeyK) {
        build.open = !build.open;
    }
}

fn toggle_journal(keyboard: Res<ButtonInput<KeyCode>>, mut journal: ResMut<JournalOpen>) {
    if keyboard.just_pressed(KeyCode::KeyJ) {
        journal.open = !journal.open;
    }
}

fn tick_run_stats(time: Res<Time>, mut stats: ResMut<RunStats>) {
    stats.elapsed_secs += time.delta_secs();
    if stats.massacre_timer_secs > 0.0 {
        stats.massacre_timer_secs = (stats.massacre_timer_secs - time.delta_secs()).max(0.0);
        if stats.massacre_timer_secs == 0.0 {
            stats.massacre_streak = 0;
        }
    }
    if stats.valor_timer_secs > 0.0 {
        stats.valor_timer_secs = (stats.valor_timer_secs - time.delta_secs()).max(0.0);
        if stats.valor_timer_secs == 0.0 {
            stats.valor_stacks = 0;
        }
    }
}

fn update_ashen_threat(
    mut kills: MessageReader<EnemyKilled>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut threat_waves: MessageWriter<SpawnAshenThreatWave>,
) {
    for kill in kills.read() {
        if kill.enemy_id == "treasure_imp" && grant_treasure_vault_reward(&mut stats) {
            combat_events.write(CombatEvent {
                text: format!(
                    "Treasure vault opened: +{} gold, +{} shards, +{} essence",
                    TREASURE_VAULT_REWARD_GOLD,
                    TREASURE_VAULT_REWARD_SHARDS,
                    TREASURE_VAULT_REWARD_ESSENCE
                ),
            });
        }
        if kill.enemy_id == "nemesis" {
            stats.nemesis_kills = stats.nemesis_kills.saturating_add(1);
        }
        let gained = ashen_threat_gain(kill);
        let surges = register_ashen_threat(&mut stats, gained);
        let first_surge = stats.ashen_threat_surges.saturating_sub(surges) + 1;
        if gained > 0 {
            combat_events.write(CombatEvent {
                text: format!(
                    "Ashen threat +{gained} ({}/{})",
                    stats.ashen_threat, ASHEN_THREAT_MAX
                ),
            });
        }
        for index in 0..surges {
            let surge = first_surge + index;
            stats.gold += 25;
            stats.ember_shards += 1;
            threat_waves.write(SpawnAshenThreatWave {
                origin: kill.position,
                surge,
            });
            combat_events.write(CombatEvent {
                text: format!("Ashen threat surge {}: +25 gold, +1 shard", surge),
            });
        }
    }
}

pub(crate) fn register_champion_pack_kill(stats: &mut RunStats) -> bool {
    if stats.champion_pack_reward_claimed {
        return false;
    }
    stats.champion_pack_kills = stats
        .champion_pack_kills
        .saturating_add(1)
        .min(CHAMPION_PACK_TARGET);
    stats.champion_pack_kills >= CHAMPION_PACK_TARGET
}

pub(crate) fn grant_champion_pack_reward(stats: &mut RunStats) -> bool {
    if stats.champion_pack_reward_claimed || stats.champion_pack_kills < CHAMPION_PACK_TARGET {
        return false;
    }
    stats.gold += CHAMPION_PACK_REWARD_GOLD;
    stats.ember_shards += CHAMPION_PACK_REWARD_SHARDS;
    stats.affix_essence += CHAMPION_PACK_REWARD_ESSENCE;
    stats.champion_pack_reward_claimed = true;
    true
}

pub(crate) fn grant_treasure_vault_reward(stats: &mut RunStats) -> bool {
    if stats.treasure_vaults_opened > 0 {
        return false;
    }
    stats.treasure_vaults_opened = 1;
    stats.gold += TREASURE_VAULT_REWARD_GOLD;
    stats.ember_shards += TREASURE_VAULT_REWARD_SHARDS;
    stats.affix_essence += TREASURE_VAULT_REWARD_ESSENCE;
    true
}

pub fn ashen_threat_gain(kill: &EnemyKilled) -> u32 {
    if kill.enemy_id == "keeper" {
        60
    } else if kill.enemy_id == "nemesis" || kill.enemy_id == "treasure_imp" {
        34
    } else if kill.affix_count > 0 {
        28 + kill.affix_count as u32 * 4
    } else {
        16
    }
}

pub(crate) fn register_ashen_threat(stats: &mut RunStats, gained: u32) -> u32 {
    stats.ashen_threat = stats.ashen_threat.saturating_add(gained);
    let surges = stats.ashen_threat / ASHEN_THREAT_MAX;
    if surges > 0 {
        stats.ashen_threat %= ASHEN_THREAT_MAX;
        stats.ashen_threat_surges = stats.ashen_threat_surges.saturating_add(surges);
    }
    surges
}

pub fn format_run_time(seconds: f32) -> String {
    let total_seconds = seconds.max(0.0).round() as u32;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

pub fn chapter_rating(difficulty: Difficulty, elapsed_secs: f32, kills: u32) -> &'static str {
    let difficulty_bonus = match difficulty {
        Difficulty::Normal => 0,
        Difficulty::Nightmare => 1,
        Difficulty::Hell => 2,
        Difficulty::Torment => 3,
    };
    let time_score = if elapsed_secs <= 240.0 {
        3
    } else if elapsed_secs <= 360.0 {
        2
    } else if elapsed_secs <= 540.0 {
        1
    } else {
        0
    };
    let kill_score = if kills >= 7 { 1 } else { 0 };
    match time_score + kill_score + difficulty_bonus {
        6.. => "S",
        4..=5 => "A",
        2..=3 => "B",
        _ => "C",
    }
}

pub fn chapter_completion_reward(
    difficulty: Difficulty,
    elapsed_secs: f32,
    kills: u32,
    lore_entries: usize,
) -> u32 {
    let rating_base = match chapter_rating(difficulty, elapsed_secs, kills) {
        "S" => 180.0,
        "A" => 130.0,
        "B" => 90.0,
        _ => 50.0,
    };
    let difficulty_scaled = (rating_base * difficulty.reward_multiplier()).round() as u32;
    difficulty_scaled + lore_entries as u32 * 20
}

pub fn chapter_completion_shard_reward(
    difficulty: Difficulty,
    elapsed_secs: f32,
    kills: u32,
    lore_entries: usize,
) -> u32 {
    let rating_base = match chapter_rating(difficulty, elapsed_secs, kills) {
        "S" => 8,
        "A" => 6,
        "B" => 4,
        _ => 2,
    };
    let difficulty_bonus = match difficulty {
        Difficulty::Normal => 0,
        Difficulty::Nightmare => 2,
        Difficulty::Hell => 4,
        Difficulty::Torment => 7,
    };
    rating_base + difficulty_bonus + lore_entries as u32
}

pub fn chapter_completion_essence_reward(
    difficulty: Difficulty,
    elapsed_secs: f32,
    kills: u32,
    lore_entries: usize,
) -> u32 {
    let rating_base = match chapter_rating(difficulty, elapsed_secs, kills) {
        "S" => 5,
        "A" => 3,
        "B" => 2,
        _ => 1,
    };
    let difficulty_bonus = match difficulty {
        Difficulty::Normal => 0,
        Difficulty::Nightmare => 1,
        Difficulty::Hell => 3,
        Difficulty::Torment => 5,
    };
    rating_base + difficulty_bonus + (lore_entries as u32 / 2)
}

pub(crate) fn primal_ember_cache_reward(
    difficulty: Difficulty,
    elapsed_secs: f32,
    kills: u32,
    boss_staggers: u32,
    boss_enraged: bool,
    echo_keystones: u32,
) -> PrimalCacheReward {
    if difficulty != Difficulty::Torment {
        return PrimalCacheReward::default();
    }

    let mut reward = PrimalCacheReward {
        gold: 180,
        shards: 10,
        essence: 8,
    };
    match chapter_rating(difficulty, elapsed_secs, kills) {
        "S" => {
            reward.gold += 80;
            reward.shards += 4;
            reward.essence += 4;
        }
        "A" => {
            reward.gold += 50;
            reward.shards += 3;
            reward.essence += 2;
        }
        _ => {}
    }
    if boss_staggers >= BOSS_BREAK_TARGET {
        reward.gold += 50;
        reward.shards += 3;
        reward.essence += 3;
    }
    if !boss_enraged {
        reward.gold += 40;
        reward.shards += 2;
        reward.essence += 2;
    }
    if echo_keystones > 0 {
        reward.gold += 100;
        reward.shards += 5;
        reward.essence += 4;
    }
    reward
}

pub fn massacre_tier(streak: u32) -> u32 {
    match streak {
        0..=2 => 0,
        3..=4 => 1,
        5..=6 => 2,
        _ => 3,
    }
}

pub fn massacre_xp_bonus(base_xp: u32, streak: u32) -> u32 {
    let tier = massacre_tier(streak);
    ((base_xp as f32) * tier as f32 * 0.25).round() as u32
}

pub fn massacre_gold_bonus(streak: u32) -> u32 {
    massacre_tier(streak) * 5
}

pub(crate) fn register_massacre_kill(stats: &mut RunStats, base_xp: u32) -> (u32, u32) {
    const MASSACRE_WINDOW_SECS: f32 = 4.0;

    if stats.massacre_timer_secs <= 0.0 {
        stats.massacre_streak = 0;
    }
    stats.massacre_streak += 1;
    stats.best_massacre_streak = stats.best_massacre_streak.max(stats.massacre_streak);
    stats.massacre_timer_secs = MASSACRE_WINDOW_SECS;

    let xp_bonus = massacre_xp_bonus(base_xp, stats.massacre_streak);
    let gold_bonus = massacre_gold_bonus(stats.massacre_streak);
    stats.gold += gold_bonus;
    stats.massacre_bonus_gold += gold_bonus;
    (xp_bonus, gold_bonus)
}

pub(crate) fn register_valor_kill(stats: &mut RunStats) -> u32 {
    const VALOR_MAX_STACKS: u32 = 5;
    const VALOR_DURATION_SECS: f32 = 90.0;

    stats.valor_stacks = (stats.valor_stacks + 1).min(VALOR_MAX_STACKS);
    stats.best_valor_stacks = stats.best_valor_stacks.max(stats.valor_stacks);
    stats.valor_timer_secs = VALOR_DURATION_SECS;
    stats.valor_stacks
}

pub(crate) fn valor_reward_multiplier(stats: &RunStats) -> f32 {
    if stats.valor_timer_secs <= 0.0 || stats.valor_stacks == 0 {
        1.0
    } else {
        1.0 + stats.valor_stacks as f32 * 0.08
    }
}

pub(crate) fn valor_xp_reward(base_xp: u32, stats: &RunStats) -> u32 {
    ((base_xp as f32) * valor_reward_multiplier(stats)).round() as u32
}

pub(crate) fn valor_gold_reward(base_gold: u32, stats: &RunStats) -> u32 {
    ((base_gold as f32) * valor_reward_multiplier(stats)).round() as u32
}

pub(crate) fn massacre_summary(stats: &RunStats) -> String {
    if stats.massacre_streak == 0 {
        format!("Massacre: best {}", stats.best_massacre_streak)
    } else {
        format!(
            "Massacre: {}x {:.0}s best {}",
            stats.massacre_streak, stats.massacre_timer_secs, stats.best_massacre_streak
        )
    }
}

pub(crate) fn valor_summary(stats: &RunStats) -> String {
    if stats.valor_stacks == 0 || stats.valor_timer_secs <= 0.0 {
        "Valor: none".to_string()
    } else {
        format!(
            "Valor: {}x {:.0}s +{:.0}% rewards",
            stats.valor_stacks,
            stats.valor_timer_secs,
            (valor_reward_multiplier(stats) - 1.0) * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_env_values<F>(display: Option<&str>, wayland_display: Option<&str>, action: F)
    where
        F: FnOnce(),
    {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock poisoned");

        let old_display = std::env::var_os("DISPLAY");
        let old_wayland = std::env::var_os("WAYLAND_DISPLAY");
        let old_headless = std::env::var_os("BEVY_OPEN_ARPG_HEADLESS_SMOKE");

        unsafe {
            match display {
                Some(value) => std::env::set_var("DISPLAY", value),
                None => std::env::remove_var("DISPLAY"),
            }
        }
        unsafe {
            match wayland_display {
                Some(value) => std::env::set_var("WAYLAND_DISPLAY", value),
                None => std::env::remove_var("WAYLAND_DISPLAY"),
            }
        }
        unsafe {
            std::env::remove_var("BEVY_OPEN_ARPG_HEADLESS_SMOKE");
        }

        action();

        unsafe {
            match old_headless {
                Some(value) => std::env::set_var("BEVY_OPEN_ARPG_HEADLESS_SMOKE", value),
                None => std::env::remove_var("BEVY_OPEN_ARPG_HEADLESS_SMOKE"),
            }
        }

        unsafe {
            match old_display {
                Some(value) => std::env::set_var("DISPLAY", value),
                None => std::env::remove_var("DISPLAY"),
            }
        }
        unsafe {
            match old_wayland {
                Some(value) => std::env::set_var("WAYLAND_DISPLAY", value),
                None => std::env::remove_var("WAYLAND_DISPLAY"),
            }
        }
    }

    fn runtime_config_for_test(headless_smoke: bool) -> OpenArpgRuntimeConfig {
        OpenArpgRuntimeConfig {
            audio_enabled: true,
            audio_locked: false,
            debug_visuals: false,
            debug_visuals_locked: false,
            #[cfg(feature = "dev_tools")]
            diagnostics_enabled: false,
            #[cfg(feature = "dev_tools")]
            free_camera_enabled: false,
            headless_smoke,
            remote_enabled: false,
            smoke_frames: 30,
            asset_mode: OpenArpgAssetMode::Unprocessed,
            render_profile: None,
            window_backend: None,
            explicit_windowed_request: false,
        }
    }

    #[test]
    fn runtime_defaults_to_windowed_when_no_display_server_is_not_explicitly_overridden() {
        with_env_values(None, None, || {
            let config = OpenArpgRuntimeConfig::from_env_and_args(["--window-title=unused"]);
            assert!(!config.headless_smoke);
            assert!(config.window_backend.is_none());
        });

        with_env_values(Some(":0"), None, || {
            let config = OpenArpgRuntimeConfig::from_env_and_args(["--window-title=unused"]);
            assert!(!config.headless_smoke);
        });

        with_env_values(None, None, || {
            let config = OpenArpgRuntimeConfig::from_env_and_args(["--windowed"]);
            assert!(!config.headless_smoke);
            assert!(config.window_backend.is_none());
        });
    }

    #[test]
    fn implicit_windowing_request_falls_back_to_headless_when_no_display() {
        with_env_values(None, None, || {
            let config = runtime_config_for_test(false);
            assert!(should_fallback_to_headless_smoke(config, false));
        });
    }

    #[test]
    fn explicit_windowed_request_blocks_headless_fallback() {
        with_env_values(None, None, || {
            let mut config = runtime_config_for_test(false);
            config.explicit_windowed_request = true;
            assert!(!should_fallback_to_headless_smoke(config, false));
        });
    }

    #[test]
    fn explicit_windowed_request_is_not_headless_when_no_display_is_detected() {
        with_env_values(None, None, || {
            let config = OpenArpgRuntimeConfig::from_env_and_args(["--windowed"]);

            assert!(!config.headless_smoke);
            assert!(config.explicit_windowed_request);
            assert!(
                startup_window_diagnostic(config, None, None, None, false, None).contains(
                    "A graphical display server was not detected, but windowed mode was requested",
                )
            );
        });
    }

    #[test]
    fn headless_smoke_env_is_obeyed_even_without_display() {
        with_env_values(None, None, || {
            unsafe {
                std::env::set_var("BEVY_OPEN_ARPG_HEADLESS_SMOKE", "off");
            }
            let config = OpenArpgRuntimeConfig::from_env_and_args(["--window-title=unused"]);
            assert!(!config.headless_smoke);
        });

        with_env_values(None, None, || {
            unsafe {
                std::env::set_var("BEVY_OPEN_ARPG_HEADLESS_SMOKE", "1");
            }
            let config = OpenArpgRuntimeConfig::from_env_and_args(["--window-title=unused"]);
            assert!(config.headless_smoke);
        });
    }

    #[test]
    fn render_profile_aliases_pick_expected_wgpu_priority() {
        assert_eq!(
            parse_open_arpg_render_profile(Some("compat")),
            Some(OpenArpgRenderProfile::Compatibility)
        );
        assert_eq!(
            parse_open_arpg_render_profile(Some("FUNCTIONALITY")),
            Some(OpenArpgRenderProfile::Functionality)
        );
        assert_eq!(
            parse_open_arpg_render_profile(Some("webgl2")),
            Some(OpenArpgRenderProfile::WebGl2)
        );
        assert_eq!(parse_open_arpg_render_profile(Some("unknown")), None);
        assert!(matches!(
            render_settings_from_profile(Some(OpenArpgRenderProfile::Compatibility))
                .map(|settings| settings.priority),
            Some(WgpuSettingsPriority::WebGPU)
        ));
        assert!(matches!(
            render_settings_from_profile(Some(OpenArpgRenderProfile::Functionality))
                .map(|settings| settings.priority),
            Some(WgpuSettingsPriority::Functionality)
        ));
        assert!(matches!(
            render_settings_from_profile(Some(OpenArpgRenderProfile::WebGl2))
                .map(|settings| settings.priority),
            Some(WgpuSettingsPriority::WebGL2)
        ));

        let gl_settings = render_settings_from_profile(Some(OpenArpgRenderProfile::Gl))
            .expect("GL profile exists");
        assert!(matches!(gl_settings.priority, WgpuSettingsPriority::WebGL2));
        assert_eq!(gl_settings.backends, Some(Backends::GL));

        assert!(render_settings_from_profile(None).is_none());
    }

    #[test]
    fn cli_args_override_audio_smoke_render_and_window_backend() {
        let args = parse_open_arpg_cli_args([
            "--no-audio",
            "--headless-smoke",
            "--smoke-frames=24",
            "--render-profile=gl",
            "--asset-mode=processed",
            "--x11",
            "--remote",
        ]);

        assert_eq!(args.audio_enabled, Some(false));
        assert_eq!(args.headless_smoke, Some(true));
        assert_eq!(args.smoke_frames, Some(24));
        assert_eq!(args.render_profile, Some(OpenArpgRenderProfile::Gl));
        assert_eq!(args.asset_mode, Some(OpenArpgAssetMode::Processed));
        assert_eq!(args.window_backend, Some(OpenArpgWindowBackend::X11));
        assert_eq!(args.remote_enabled, Some(true));

        let config = OpenArpgRuntimeConfig::from_env_and_args([
            "--no-audio",
            "--headless-smoke",
            "--smoke-frames=999",
            "--render-profile=compat",
            "--wayland",
        ]);
        assert!(!config.audio_enabled);
        assert!(config.headless_smoke);
        assert_eq!(config.smoke_frames, 600);
        assert_eq!(
            config.render_profile,
            Some(OpenArpgRenderProfile::Compatibility)
        );
        assert_eq!(config.window_backend, Some(OpenArpgWindowBackend::Wayland));
    }

    #[test]
    fn user_settings_export_runtime_feature_preferences() {
        let config = OpenArpgRuntimeConfig::from_env_and_args([
            "--no-audio",
            "--debug-gizmos",
            "--remote",
            "--asset-mode=processed",
            "--render-profile=compat",
        ]);
        let settings = OpenArpgUserSettings::from_config(config);

        assert!(!settings.audio_enabled);
        assert!(settings.debug_visuals);
        assert!(settings.remote_enabled);
        assert_eq!(settings.asset_mode, "processed");
        assert_eq!(settings.render_profile, "compatibility");
    }

    #[test]
    fn reflected_settings_docs_and_helpers_are_available_for_tooling() {
        assert_eq!(reflected_user_settings_doc_counts(), (5, 5));
        assert_eq!(reflected_tool_helper_count(), 4);

        let audit = bevy_runtime_feature_audit_summary();
        assert!(audit.contains("reflect_docs=5/5 settings fields"));
        assert!(audit.contains("reflect_functions=4 helpers"));
        assert!(audit.contains("settings=on"));
        assert!(audit.contains("animation=gltf_animation+morph_animation"));
        assert!(audit.contains("ui=bevy_ui+widgets+input-focus"));
        assert!(audit.contains("picking=mesh+ui"));
        assert!(audit.contains("post_process=hdr+bloom+smaa+dof+fog"));
        assert!(audit.contains("omitted=http/https+solari+dlss+meshlet+hotpatching+zstd_c"));
        assert!(audit.contains("deferred=feathers+clipboard_image+pan_camera"));
    }

    #[test]
    fn camera_post_processing_uses_subtle_arpg_tuning() {
        let depth = open_arpg_depth_of_field();
        assert_eq!(depth.mode, DepthOfFieldMode::Gaussian);
        assert_eq!(depth.focal_distance, 16.0);
        assert!(depth.aperture_f_stops > 6.0);
        assert!(depth.max_circle_of_confusion_diameter <= 10.0);

        let vignette = open_arpg_vignette();
        assert!((0.25..=0.4).contains(&vignette.intensity));
        assert!(vignette.radius >= 0.9);
        assert_eq!(vignette.center, Vec2::new(0.5, 0.53));

        let chromatic = open_arpg_chromatic_aberration();
        assert!(chromatic.intensity > 0.0);
        assert!(chromatic.intensity < 0.01);
        assert_eq!(chromatic.max_samples, 4);

        let atmosphere = open_arpg_atmosphere_settings();
        assert_eq!(atmosphere.transmittance_lut_size, UVec2::new(128, 64));
        assert_eq!(atmosphere.sky_view_lut_size, UVec2::new(256, 128));
        assert_eq!(atmosphere.aerial_view_lut_size, UVec3::new(16, 16, 16));
        assert!(matches!(
            atmosphere.rendering_method,
            AtmosphereMode::LookupTexture
        ));

        let environment = open_arpg_atmosphere_environment_light();
        assert!(environment.intensity > 0.0);
        assert!(environment.intensity < 0.25);
        assert_eq!(environment.size, UVec2::new(256, 256));
    }

    #[test]
    fn asset_mode_parser_defaults_safe_and_accepts_processed_aliases() {
        assert_eq!(
            parse_open_arpg_asset_mode(None),
            OpenArpgAssetMode::Unprocessed
        );
        assert_eq!(
            parse_open_arpg_asset_mode(Some("unprocessed")),
            OpenArpgAssetMode::Unprocessed
        );
        assert_eq!(
            parse_open_arpg_asset_mode(Some("processed")),
            OpenArpgAssetMode::Processed
        );
        assert_eq!(
            parse_open_arpg_asset_mode(Some(" IMPORTED_ASSETS ")),
            OpenArpgAssetMode::Processed
        );
        assert_eq!(
            parse_open_arpg_asset_mode(Some("bad")),
            OpenArpgAssetMode::Unprocessed
        );
    }

    #[test]
    fn audio_settings_toggle_and_label_track_runtime_mute() {
        let mut settings = AudioSettings { enabled: true };
        assert_eq!(settings.status_label(), "audio on");

        settings.toggle();
        assert!(!settings.enabled);
        assert_eq!(settings.status_label(), "audio muted");

        settings.toggle();
        assert!(settings.enabled);
        assert_eq!(settings.status_label(), "audio on");
    }

    #[test]
    fn clipboard_summary_exports_runtime_feature_state() {
        let stats = RunStats {
            elapsed_secs: 93.5,
            kills: 27,
            gold: 420,
            ember_shards: 8,
            affix_essence: 3,
            journey_score: 11,
            renown_rank: 2,
            ashen_threat: 64,
            ..Default::default()
        };
        let audio = AudioSettings { enabled: false };
        let debug = DebugVisuals { enabled: true };
        let settings = OpenArpgUserSettings {
            remote_enabled: true,
            asset_mode: "processed".to_string(),
            render_profile: "compatibility".to_string(),
            ..Default::default()
        };

        let summary = runtime_clipboard_summary(
            GameState::InGame,
            Difficulty::Hell,
            &stats,
            &audio,
            &debug,
            &settings,
            Some(true),
        );

        assert!(summary.contains("state=in_game"));
        assert!(summary.contains("difficulty=Hell"));
        assert!(summary.contains("kills=27"));
        assert!(summary.contains("audio=audio muted"));
        assert!(summary.contains("debug_gizmos=on"));
        assert!(summary.contains("ui_debug_overlay=on"));
        assert!(summary.contains("remote=on"));
        assert!(summary.contains(&format!(
            "asset_hot_reload={}",
            open_arpg_asset_hot_reload_status()
        )));
        assert!(summary.contains("asset_mode=processed"));
        assert!(
            summary.contains("bevy_feature_audit=profiles=3d+ui+scene+picking+audio-all-formats")
        );
        assert!(summary.contains("animation=gltf_animation+morph_animation"));
        assert!(summary.contains("ui=bevy_ui+widgets+input-focus"));
        assert!(summary.contains("assets=processor+gltf+ktx2+basis+webp+hdr+audio-codecs"));
        assert!(summary.contains("render=hdr+bloom+smaa+dof+fog+area-lights+pcss+contact-shadows"));
        assert!(summary.contains("platform=x11+wayland+accesskit+gamepad+clipboard+system-fonts"));
        assert!(summary.contains("tools=settings+remote-opt-in+ui-debug+reflect_docs=5/5 settings fields+reflect_functions=4 helpers"));
        assert!(summary.contains("omitted=http/https+solari+dlss+meshlet+hotpatching+zstd_c"));
        assert!(summary.contains("deferred=feathers+clipboard_image+pan_camera"));
        assert!(summary.contains(&format!(
            "dev_tools={}",
            open_arpg_asset_hot_reload_status()
        )));
    }

    #[test]
    fn asset_hot_reload_matches_dev_tools_feature_gate() {
        #[cfg(feature = "dev_tools")]
        {
            assert_eq!(open_arpg_watch_for_changes_override(), Some(true));
            assert_eq!(open_arpg_asset_hot_reload_status(), "on");
        }
        #[cfg(not(feature = "dev_tools"))]
        {
            assert_eq!(open_arpg_watch_for_changes_override(), None);
            assert_eq!(open_arpg_asset_hot_reload_status(), "off");
        }
    }

    #[test]
    fn ui_debug_overlay_toggle_configures_bevy_debug_boxes() {
        let mut options = GlobalUiDebugOptions::default();
        assert_eq!(ui_debug_overlay_status(Some(options.enabled)), "off");

        apply_open_arpg_ui_debug_overlay(&mut options, true);

        assert!(options.enabled);
        assert!(options.outline_border_box);
        assert!(options.outline_padding_box);
        assert!(options.outline_content_box);
        assert!(options.outline_scrollbars);
        assert!(options.show_clipped);
        assert!(options.ignore_border_radius);
        assert_eq!(options.line_width, 2.0);
        assert_eq!(ui_debug_overlay_status(Some(options.enabled)), "on");
        assert_eq!(ui_debug_overlay_status(None), "unavailable");

        apply_open_arpg_ui_debug_overlay(&mut options, false);

        assert!(!options.enabled);
        assert!(options.outline_border_box);
        assert!(!options.outline_padding_box);
        assert!(!options.outline_content_box);
        assert!(!options.outline_scrollbars);
        assert!(!options.show_clipped);
        assert!(!options.ignore_border_radius);
        assert_eq!(options.line_width, 1.0);
    }

    #[test]
    fn new_chapter_reset_preserves_unspent_malrec_soul_sigils_only() {
        let mut stats = RunStats {
            kills: 12,
            gold: 400,
            ember_shards: 9,
            affix_essence: 4,
            malrec_soul_sigils: 3,
            malrec_soul_sigils_earned: 2,
            soul_sigil_caches: 1,
            boss_staggers: 2,
            completion_reward_claimed: true,
            echo_keystones: 1,
            ..Default::default()
        };

        reset_run_stats_for_new_chapter(&mut stats);

        assert_eq!(stats.malrec_soul_sigils, 3);
        assert_eq!(stats.malrec_soul_sigils_earned, 0);
        assert_eq!(stats.kills, 0);
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(stats.affix_essence, 0);
        assert_eq!(stats.soul_sigil_caches, 0);
        assert_eq!(stats.boss_staggers, 0);
        assert_eq!(stats.echo_keystones, 0);
        assert!(!stats.completion_reward_claimed);
    }

    #[test]
    fn victory_continue_uses_unlocked_record_route_and_keeps_torment_endgame() {
        let mut records = ChapterRecords::default();

        assert_eq!(
            victory_next_difficulty_after_clear(Difficulty::Normal, &records),
            Difficulty::Normal
        );

        records.add_clear(Difficulty::Normal);
        assert_eq!(
            victory_next_difficulty_after_clear(Difficulty::Normal, &records),
            Difficulty::Nightmare
        );

        records.add_clear(Difficulty::Nightmare);
        assert_eq!(
            victory_next_difficulty_after_clear(Difficulty::Nightmare, &records),
            Difficulty::Hell
        );

        records.add_clear(Difficulty::Hell);
        assert_eq!(
            victory_next_difficulty_after_clear(Difficulty::Hell, &records),
            Difficulty::Torment
        );

        records.add_clear(Difficulty::Torment);
        assert_eq!(
            victory_next_difficulty_after_clear(Difficulty::Torment, &records),
            Difficulty::Torment
        );
    }

    #[test]
    fn difficulty_cycles_and_scales() {
        assert_eq!(Difficulty::Normal.next(), Difficulty::Nightmare);
        assert_eq!(Difficulty::Hell.next(), Difficulty::Torment);
        assert_eq!(Difficulty::Torment.next(), Difficulty::Normal);
        assert_eq!(
            escalated_difficulty_after_clear(Difficulty::Normal),
            Difficulty::Nightmare
        );
        assert_eq!(
            escalated_difficulty_after_clear(Difficulty::Nightmare),
            Difficulty::Hell
        );
        assert_eq!(
            escalated_difficulty_after_clear(Difficulty::Hell),
            Difficulty::Torment
        );
        assert_eq!(
            escalated_difficulty_after_clear(Difficulty::Torment),
            Difficulty::Torment
        );
        assert!(
            Difficulty::Torment.enemy_health_multiplier()
                > Difficulty::Normal.enemy_health_multiplier()
        );
        assert!(Difficulty::Nightmare.reward_multiplier() > Difficulty::Normal.reward_multiplier());
        assert!(Difficulty::Torment.reward_multiplier() > Difficulty::Hell.reward_multiplier());
    }

    #[test]
    fn run_time_and_rating_are_stable() {
        assert_eq!(format_run_time(125.2), "2:05");
        assert_eq!(chapter_rating(Difficulty::Hell, 220.0, 7), "S");
        assert_eq!(chapter_rating(Difficulty::Torment, 360.0, 7), "S");
        assert_eq!(chapter_rating(Difficulty::Normal, 700.0, 3), "C");
    }

    #[test]
    fn completion_reward_scales_with_rating_difficulty_and_lore() {
        assert!(
            chapter_completion_reward(Difficulty::Hell, 220.0, 7, 3)
                > chapter_completion_reward(Difficulty::Normal, 700.0, 3, 0)
        );
        assert!(
            chapter_completion_shard_reward(Difficulty::Hell, 220.0, 7, 3)
                > chapter_completion_shard_reward(Difficulty::Normal, 700.0, 3, 0)
        );
        assert!(
            chapter_completion_essence_reward(Difficulty::Hell, 220.0, 7, 3)
                > chapter_completion_essence_reward(Difficulty::Normal, 700.0, 3, 0)
        );
        assert_eq!(
            chapter_completion_reward(Difficulty::Normal, 700.0, 3, 2),
            90
        );
        assert_eq!(
            chapter_completion_shard_reward(Difficulty::Normal, 700.0, 3, 2),
            4
        );
        assert_eq!(
            chapter_completion_essence_reward(Difficulty::Normal, 700.0, 3, 2),
            2
        );
    }

    #[test]
    fn primal_ember_cache_rewards_torment_endgame_performance() {
        assert!(primal_ember_cache_reward(Difficulty::Hell, 220.0, 7, 2, false, 1).is_empty());

        let slow_enraged = primal_ember_cache_reward(Difficulty::Torment, 700.0, 3, 0, true, 0);
        let clean_break = primal_ember_cache_reward(Difficulty::Torment, 360.0, 7, 2, false, 0);
        let echo_empowered = primal_ember_cache_reward(Difficulty::Torment, 360.0, 7, 2, false, 1);

        assert_eq!(
            slow_enraged,
            PrimalCacheReward {
                gold: 180,
                shards: 10,
                essence: 8,
            }
        );
        assert!(clean_break.gold > slow_enraged.gold);
        assert!(clean_break.shards > slow_enraged.shards);
        assert!(clean_break.essence > slow_enraged.essence);
        assert!(echo_empowered.gold > clean_break.gold);
        assert!(echo_empowered.shards > clean_break.shards);
        assert!(echo_empowered.essence > clean_break.essence);
    }

    #[test]
    fn massacre_rewards_scale_by_streak_tier() {
        let mut stats = RunStats::default();

        assert_eq!(register_massacre_kill(&mut stats, 20), (0, 0));
        assert_eq!(register_massacre_kill(&mut stats, 20), (0, 0));
        assert_eq!(register_massacre_kill(&mut stats, 20), (5, 5));
        assert_eq!(register_massacre_kill(&mut stats, 20), (5, 5));
        assert_eq!(register_massacre_kill(&mut stats, 20), (10, 10));
        assert_eq!(stats.massacre_streak, 5);
        assert_eq!(stats.best_massacre_streak, 5);
        assert_eq!(stats.massacre_bonus_gold, 20);
        assert_eq!(stats.gold, 20);
    }

    #[test]
    fn massacre_summary_handles_inactive_and_active_streaks() {
        let inactive = RunStats {
            best_massacre_streak: 6,
            ..RunStats::default()
        };
        assert_eq!(massacre_summary(&inactive), "Massacre: best 6");

        let active = RunStats {
            massacre_streak: 4,
            best_massacre_streak: 5,
            massacre_timer_secs: 2.4,
            ..RunStats::default()
        };
        assert_eq!(massacre_summary(&active), "Massacre: 4x 2s best 5");
    }

    #[test]
    fn valor_stacks_cap_and_scale_rewards() {
        let mut stats = RunStats::default();

        for _ in 0..7 {
            register_valor_kill(&mut stats);
        }

        assert_eq!(stats.valor_stacks, 5);
        assert_eq!(stats.best_valor_stacks, 5);
        assert_eq!(stats.valor_timer_secs, 90.0);
        assert_eq!(valor_xp_reward(100, &stats), 140);
        assert_eq!(valor_gold_reward(50, &stats), 70);
        assert!(valor_summary(&stats).contains("5x"));
        assert!(valor_summary(&stats).contains("+40%"));
    }

    #[test]
    fn inactive_valor_does_not_scale_rewards() {
        let stats = RunStats {
            valor_stacks: 3,
            valor_timer_secs: 0.0,
            ..RunStats::default()
        };

        assert_eq!(valor_reward_multiplier(&stats), 1.0);
        assert_eq!(valor_xp_reward(100, &stats), 100);
        assert_eq!(valor_gold_reward(50, &stats), 50);
        assert_eq!(valor_summary(&stats), "Valor: none");
    }

    #[test]
    fn display_detection_accepts_x11_or_wayland_socket() {
        assert!(display_vars_present(Some(":0"), None, None));
        assert!(display_vars_present(None, Some("wayland-0"), None));
        assert!(display_vars_present(Some("  :1  "), Some(""), None));
        assert!(display_vars_present(None, None, Some("wayland-0")));
        assert!(!display_vars_present(None, None, None));
        assert!(!display_vars_present(Some(" "), Some(""), Some("")));

        assert_eq!(display_server_label(Some(":0"), None, None), "x11");
        assert_eq!(
            display_server_label(None, Some("wayland-0"), None),
            "wayland"
        );
        assert_eq!(
            display_server_label(Some(":0"), Some("wayland-0"), None),
            "x11+wayland"
        );
        assert_eq!(
            display_server_label(Some(":0"), None, Some("wayland-socket")),
            "x11+socket"
        );
        assert_eq!(
            display_server_label(None, Some("wayland-0"), Some("socket")),
            "wayland+socket"
        );
        assert_eq!(display_server_label(None, None, Some("socket")), "socket");
        assert_eq!(display_server_label(None, None, None), "none");
    }

    #[test]
    fn headless_smoke_uses_no_primary_window_and_clamped_frame_budget() {
        assert!(open_arpg_primary_window(true).is_none());
        let window = open_arpg_primary_window(false).expect("normal runtime should open a window");
        assert_eq!(window.title, OPEN_ARPG_WINDOW_TITLE);

        assert_eq!(parse_open_arpg_smoke_frames(None), 12);
        assert_eq!(parse_open_arpg_smoke_frames(Some("0")), 1);
        assert_eq!(parse_open_arpg_smoke_frames(Some("3")), 3);
        assert_eq!(parse_open_arpg_smoke_frames(Some("9999")), 600);
        assert_eq!(parse_open_arpg_smoke_frames(Some("bad")), 12);
    }

    #[test]
    fn startup_window_diagnostic_reports_window_backend_and_render_profile() {
        let mut config = runtime_config_for_test(false);
        config.render_profile = Some(OpenArpgRenderProfile::Compatibility);
        config.window_backend = Some(OpenArpgWindowBackend::X11);

        let diagnostic =
            startup_window_diagnostic(config, Some(":0"), None, Some("wayland"), true, None);

        assert!(diagnostic.contains("window=enabled"));
        assert!(diagnostic.contains("title=\"Bevy Open ARPG\""));
        assert!(diagnostic.contains("size=1280x720"));
        assert!(diagnostic.contains("display=x11"));
        assert!(diagnostic.contains("backend=x11(forced)"));
        assert!(diagnostic.contains("render=compatibility"));
        assert!(diagnostic.contains("retry `cargo run -- --x11 --render-profile=compat`"));
        assert!(
            diagnostic.contains("Feature audit: profiles=3d+ui+scene+picking+audio-all-formats")
        );
        assert!(diagnostic.contains("reflect_docs=5/5 settings fields"));
    }

    #[test]
    fn startup_window_diagnostic_explains_headless_no_popup_mode() {
        let mut config = runtime_config_for_test(true);
        config.audio_enabled = false;
        config.asset_mode = OpenArpgAssetMode::Processed;

        let diagnostic = startup_window_diagnostic(config, Some(":0"), None, None, true, None);

        assert!(diagnostic.contains("window=disabled(headless-smoke)"));
        assert!(diagnostic.contains("audio=off"));
        assert!(diagnostic.contains("asset_mode=processed"));
        assert!(diagnostic.contains("run `cargo run -- --windowed`"));
        assert!(
            diagnostic.contains("Feature audit: profiles=3d+ui+scene+picking+audio-all-formats")
        );
        assert!(diagnostic.contains("reflect_docs=5/5 settings fields"));
        assert!(!diagnostic.contains("No DISPLAY or WAYLAND_DISPLAY"));
    }

    #[test]
    fn startup_window_diagnostic_explains_missing_display_server() {
        let config = runtime_config_for_test(false);

        let diagnostic = startup_window_diagnostic(config, None, Some(" "), None, false, None);

        assert!(diagnostic.contains("display=none"));
        assert!(diagnostic.contains("A graphical display server was not detected"));
        assert!(diagnostic.contains("Verify `DISPLAY`/`WAYLAND_DISPLAY`"));
        assert!(
            diagnostic.contains("Feature audit: profiles=3d+ui+scene+picking+audio-all-formats")
        );
        assert!(diagnostic.contains("reflect_docs=5/5 settings fields"));
    }

    #[test]
    fn ashen_threat_gain_rewards_elites_and_bosses_more_than_normal_kills() {
        let normal = EnemyKilled {
            enemy_id: "skeleton".to_string(),
            display_name: "Skeleton".to_string(),
            position: Vec3::ZERO,
            xp_reward: 10,
            affix_count: 0,
            affix_mask: 0,
            cursed_ambusher: false,
            champion_pack_member: false,
        };
        let elite = EnemyKilled {
            enemy_id: "cultist".to_string(),
            display_name: "Elite Cultist".to_string(),
            affix_count: 2,
            ..normal.clone()
        };
        let boss = EnemyKilled {
            enemy_id: "keeper".to_string(),
            display_name: "Malrec".to_string(),
            affix_count: 8,
            ..normal.clone()
        };

        assert_eq!(ashen_threat_gain(&normal), 16);
        assert!(ashen_threat_gain(&elite) > ashen_threat_gain(&normal));
        assert!(ashen_threat_gain(&boss) > ashen_threat_gain(&elite));
    }

    #[test]
    fn ashen_threat_rolls_over_and_counts_surges() {
        let mut stats = RunStats {
            ashen_threat: 92,
            ..default()
        };

        assert_eq!(register_ashen_threat(&mut stats, 32), 1);
        assert_eq!(stats.ashen_threat, 24);
        assert_eq!(stats.ashen_threat_surges, 1);

        assert_eq!(register_ashen_threat(&mut stats, 210), 2);
        assert_eq!(stats.ashen_threat, 34);
        assert_eq!(stats.ashen_threat_surges, 3);
    }

    #[test]
    fn champion_pack_reward_claims_once_after_full_pack() {
        let mut stats = RunStats::default();

        for _ in 0..CHAMPION_PACK_TARGET - 1 {
            assert!(!register_champion_pack_kill(&mut stats));
        }
        assert!(!grant_champion_pack_reward(&mut stats));
        assert!(register_champion_pack_kill(&mut stats));
        assert!(grant_champion_pack_reward(&mut stats));
        assert_eq!(stats.gold, CHAMPION_PACK_REWARD_GOLD);
        assert_eq!(stats.ember_shards, CHAMPION_PACK_REWARD_SHARDS);
        assert_eq!(stats.affix_essence, CHAMPION_PACK_REWARD_ESSENCE);
        assert!(!grant_champion_pack_reward(&mut stats));
        assert_eq!(stats.gold, CHAMPION_PACK_REWARD_GOLD);
    }

    #[test]
    fn treasure_vault_reward_claims_once_after_imp_kill() {
        let mut stats = RunStats::default();

        assert!(grant_treasure_vault_reward(&mut stats));
        assert_eq!(stats.treasure_vaults_opened, 1);
        assert_eq!(stats.gold, TREASURE_VAULT_REWARD_GOLD);
        assert_eq!(stats.ember_shards, TREASURE_VAULT_REWARD_SHARDS);
        assert_eq!(stats.affix_essence, TREASURE_VAULT_REWARD_ESSENCE);

        assert!(!grant_treasure_vault_reward(&mut stats));
        assert_eq!(stats.treasure_vaults_opened, 1);
        assert_eq!(stats.gold, TREASURE_VAULT_REWARD_GOLD);
    }
}
