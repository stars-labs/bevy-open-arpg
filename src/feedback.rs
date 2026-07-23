use crate::{
    AudioSettings, GameState,
    bestiary::chapter_enemy_counterplay,
    combat::BasicAttackIntent,
    data::EnemyAttackKind,
    enemy::{
        BossPhase, Chilled, Enemy, EnemyAffix, EnemyEntity, EnemyTargetFocus, SealWardenWard,
        Staggered, enemy_cursor_pick_radius,
    },
    not_paused,
    player::Health,
};
use bevy::prelude::*;
use bevy::time::{Real, Virtual};
use bevy::window::PrimaryWindow;
#[cfg(not(target_arch = "wasm32"))]
use rodio::Source;
use std::{collections::VecDeque, sync::mpsc::Sender};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

const LOG_CAPACITY: usize = 6;

#[derive(Message, Debug, Clone)]
pub struct CombatEvent {
    pub text: String,
}

#[derive(Message, Debug, Clone)]
pub struct FloatingCombatTextEvent {
    pub text: String,
    pub position: Vec3,
    pub critical: bool,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct ScreenShakeEvent {
    pub intensity: f32,
    pub duration_secs: f32,
}

impl ScreenShakeEvent {
    pub fn new(intensity: f32, duration_secs: f32) -> Self {
        Self {
            intensity,
            duration_secs,
        }
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct HitStopEvent {
    pub relative_speed: f32,
    pub duration_secs: f32,
}

impl HitStopEvent {
    pub fn new(relative_speed: f32, duration_secs: f32) -> Self {
        Self {
            relative_speed: relative_speed.clamp(0.16, 0.82),
            duration_secs: duration_secs.clamp(0.012, 0.12),
        }
    }
}

#[derive(Resource, Default)]
pub struct CombatLog {
    entries: VecDeque<String>,
}

#[derive(Resource, Default)]
struct CameraShakeState {
    timer: Option<Timer>,
    duration_secs: f32,
    intensity: f32,
    base_translation: Option<Vec3>,
}

#[derive(Resource, Debug, Default)]
struct HitStopState {
    remaining_secs: f32,
    duration_secs: f32,
    relative_speed: f32,
}

#[derive(Resource, Default)]
struct GameAudio {
    sender: Option<Sender<SoundCue>>,
    // The web build has no mixer thread; cues queue here and a per-frame
    // system plays them through bevy_audio (Web Audio API).
    #[cfg(target_arch = "wasm32")]
    web_queue: std::sync::Mutex<Vec<SoundCue>>,
}

#[derive(Resource)]
struct AudioBackendRetry {
    timer: Timer,
}

impl Default for AudioBackendRetry {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Repeating),
        }
    }
}

#[derive(Resource, Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum AudioBackendStatus {
    Muted,
    #[default]
    Starting,
    Ready,
    NoOutputDevice,
    // Only the native backend spawns the mixer thread that can fail.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    ThreadFailed,
}

impl AudioBackendStatus {
    pub fn status_label(self, settings: &AudioSettings) -> &'static str {
        if !settings.enabled {
            return "audio muted";
        }
        match self {
            Self::Muted => "audio muted",
            Self::Starting => "audio starting",
            Self::Ready => "audio on",
            Self::NoOutputDevice => "audio no device",
            Self::ThreadFailed => "audio thread failed",
        }
    }
}

#[derive(Resource, Default)]
struct AudioCueLimiter {
    last_played: Vec<(SoundCue, f32)>,
}

#[derive(Resource, Default, Clone, Copy, Debug, Eq, PartialEq)]
struct EnemyHoverTarget(Option<Entity>);

#[derive(Resource, Default, Debug, Clone)]
pub struct TargetInfo {
    pub visible: bool,
    pub name: String,
    pub subtitle: String,
    pub health_line: String,
    pub details: String,
    pub health_percent: f32,
    pub threat_color: Color,
}

impl AudioCueLimiter {
    fn should_play(&mut self, cue: SoundCue, now_secs: f32) -> bool {
        let cooldown = sound_cue_cooldown_secs(cue);
        if cooldown <= 0.0 {
            self.remember(cue, now_secs);
            return true;
        }
        if let Some((_, last_secs)) = self.last_played.iter_mut().find(|(last, _)| *last == cue) {
            if now_secs - *last_secs < cooldown {
                return false;
            }
            *last_secs = now_secs;
            return true;
        }
        self.last_played.push((cue, now_secs));
        true
    }

    fn remember(&mut self, cue: SoundCue, now_secs: f32) {
        if let Some((_, last_secs)) = self.last_played.iter_mut().find(|(last, _)| *last == cue) {
            *last_secs = now_secs;
        } else {
            self.last_played.push((cue, now_secs));
        }
    }
}

impl CombatLog {
    pub fn push(&mut self, text: impl Into<String>) {
        self.entries.push_front(text.into());
        while self.entries.len() > LOG_CAPACITY {
            self.entries.pop_back();
        }
    }

    pub fn push_event(&mut self, text: &str) {
        let Some(entry) = combat_log_event_entry(text) else {
            return;
        };
        self.push(entry);
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(String::as_str)
    }
}

fn combat_log_event_entry(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || combat_log_is_transient_feedback(text) {
        None
    } else {
        Some(text.to_string())
    }
}

fn combat_log_is_transient_feedback(text: &str) -> bool {
    text.starts_with("Picked up ")
        || text.starts_with("Generated ")
        || text.starts_with("Need ")
        || text.contains(" not ready ")
        || text.starts_with("Potion restored ")
        || text.starts_with("Potion recharging ")
        || text.starts_with("Health globe restored ")
        || text == "Health globe dropped"
        || text.starts_with("Renewal well restored ")
        || text.starts_with("Ember altar extinguished")
        || text.starts_with("Combo Ready: ")
        || text.starts_with("Combo Break: ")
        || text == "Evade"
        || text.starts_with("Evade recharging ")
        || text == "Nephalem Surge ready"
        || text == "Nephalem Surge unleashed"
        || text.starts_with("Nephalem Surge charging ")
        || text.starts_with("Nephalem Surge extended ")
        || text.starts_with("Reap restored ")
        || text.starts_with("Rupture exposed ")
        || text.starts_with("Rupture bleeding ")
        || text.starts_with("Soulreaver restored ")
        || text.starts_with("Aegisbrand granted ")
        || text.starts_with("Emberbrand ")
        || text.starts_with("Frostbrand ")
        || text.starts_with("Stormbrand ")
        || text.starts_with("Elixir selected: ")
        || text.contains(" hit for ")
        || combat_log_is_skill_cast(text)
}

fn combat_log_is_skill_cast(text: &str) -> bool {
    let Some((rune, skill)) = text.split_once(' ') else {
        return false;
    };
    matches!(
        (rune, skill),
        ("Cleanse" | "Reap", "Dash")
            | ("Expose" | "Hemorrhage", "Rupture")
            | ("Ember" | "Frost", "Nova")
    )
}

#[derive(Component)]
struct EnemyHealthBar {
    background: Entity,
    fill: Entity,
    nameplate: Entity,
    focus_ring: Entity,
    base_width: f32,
}

#[derive(Component)]
struct EnemyHealthBarBackground;

#[derive(Component)]
struct EnemyHealthBarFill;

#[derive(Component)]
struct EnemyNameplate {
    enemy: Entity,
}

#[derive(Component)]
struct EnemyFocusRing;

type EnemyBarQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Health,
        &'static Enemy,
        &'static EnemyHealthBar,
        Option<&'static Chilled>,
        Option<&'static Staggered>,
        Option<&'static BossPhase>,
    ),
>;

type EnemyHoverQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Enemy, &'static Transform, &'static Health),
    (With<Enemy>, Without<EnemyFocusRing>),
>;

type EnemyBarFillQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static MeshMaterial3d<StandardMaterial>,
        &'static mut Visibility,
    ),
    With<EnemyHealthBarFill>,
>;

type EnemyBarBackgroundQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Visibility,
    (
        With<EnemyHealthBarBackground>,
        Without<EnemyHealthBarFill>,
        Without<EnemyNameplate>,
    ),
>;

type EnemyNameplateEnemyQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Enemy,
        &'static Health,
        &'static EnemyHealthBar,
        Option<&'static Chilled>,
        Option<&'static Staggered>,
        Option<&'static BossPhase>,
    ),
>;

type EnemyNameplateVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static EnemyNameplate,
        &'static mut Transform,
        &'static mut Text2d,
        &'static mut TextColor,
        &'static mut Visibility,
    ),
>;

type EnemyFocusQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Enemy,
        &'static Transform,
        &'static Health,
        &'static EnemyHealthBar,
        Option<&'static mut EnemyTargetFocus>,
    ),
    Without<EnemyFocusRing>,
>;

type EnemyFocusRingQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (With<EnemyFocusRing>, Without<Enemy>),
>;

type TargetInfoEnemyQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Enemy,
        &'static Health,
        Option<&'static Chilled>,
        Option<&'static Staggered>,
        Option<&'static BossPhase>,
        Option<&'static SealWardenWard>,
        Option<&'static EnemyTargetFocus>,
    ),
>;

#[derive(Component)]
struct FloatingCombatText {
    timer: Timer,
    lifetime_secs: f32,
    velocity: Vec3,
    base_scale: Vec3,
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FloatingTextStyle {
    color: Color,
    font_size: f32,
    scale: Vec3,
    velocity: Vec3,
    lifetime_secs: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SoundCue {
    Hit,
    Critical,
    Loot,
    Danger,
    Death,
    Skill,
    Combo,
    Boss,
    Quest,
    Potion,
    Utility,
    Victory,
    Defeat,
}

#[cfg(test)]
const ALL_SOUND_CUES: [SoundCue; 13] = [
    SoundCue::Hit,
    SoundCue::Critical,
    SoundCue::Loot,
    SoundCue::Danger,
    SoundCue::Death,
    SoundCue::Skill,
    SoundCue::Combo,
    SoundCue::Boss,
    SoundCue::Quest,
    SoundCue::Potion,
    SoundCue::Utility,
    SoundCue::Victory,
    SoundCue::Defeat,
];

pub struct FeedbackPlugin;

impl Plugin for FeedbackPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, play_web_sound_cues);
        app.init_resource::<CombatLog>()
            .init_resource::<CameraShakeState>()
            .init_resource::<HitStopState>()
            .init_resource::<GameAudio>()
            .init_resource::<AudioBackendRetry>()
            .init_resource::<AudioBackendStatus>()
            .init_resource::<AudioCueLimiter>()
            .init_resource::<EnemyHoverTarget>()
            .init_resource::<TargetInfo>()
            .init_resource::<BasicAttackIntent>()
            .add_message::<CombatEvent>()
            .add_message::<FloatingCombatTextEvent>()
            .add_message::<ScreenShakeEvent>()
            .add_message::<HitStopEvent>()
            .add_systems(Startup, (setup_audio_backend, start_ambient_music))
            .add_systems(Update, (maintain_audio_backend, sync_ambient_music_mute))
            .add_systems(
                Update,
                (
                    collect_combat_events,
                    spawn_floating_combat_text,
                    update_floating_combat_text,
                    attach_enemy_health_bars,
                    (
                        update_enemy_hover_target,
                        update_target_info,
                        update_enemy_health_bars,
                        update_enemy_nameplates,
                        update_enemy_focus_targets,
                    )
                        .chain(),
                    update_camera_shake,
                    update_hit_stop,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), reset_hit_stop)
            .add_systems(OnEnter(GameState::Victory), play_victory_sound)
            .add_systems(OnEnter(GameState::GameOver), play_defeat_sound);
    }
}

fn floating_text_color(critical: bool) -> Color {
    if critical {
        Color::srgb(1.0, 0.78, 0.22)
    } else {
        Color::srgb(0.94, 0.90, 0.78)
    }
}

fn floating_text_font_size(critical: bool) -> f32 {
    if critical { 32.0 } else { 24.0 }
}

fn floating_text_style(text: &str, critical: bool) -> FloatingTextStyle {
    if text.contains("BOSS DOWN") {
        return FloatingTextStyle {
            color: Color::srgb(1.0, 0.22, 0.08),
            font_size: 42.0,
            scale: Vec3::splat(0.024),
            velocity: Vec3::new(0.0, 1.62, 0.0),
            lifetime_secs: 1.20,
        };
    }
    if text.contains("ELITE SLAY") || text.contains("EXECUTE") {
        return FloatingTextStyle {
            color: Color::srgb(1.0, 0.55, 0.12),
            font_size: 36.0,
            scale: Vec3::splat(0.021),
            velocity: Vec3::new(0.0, 1.52, 0.0),
            lifetime_secs: 1.02,
        };
    }
    if text.contains("SLAY") {
        return FloatingTextStyle {
            color: Color::srgb(0.98, 0.82, 0.42),
            font_size: 30.0,
            scale: Vec3::splat(0.017),
            velocity: Vec3::new(0.0, 1.42, 0.0),
            lifetime_secs: 0.92,
        };
    }
    FloatingTextStyle {
        color: floating_text_color(critical),
        font_size: floating_text_font_size(critical),
        scale: if critical {
            Vec3::splat(0.018)
        } else {
            Vec3::splat(0.014)
        },
        velocity: Vec3::new(0.0, if critical { 1.48 } else { 1.28 }, 0.0),
        lifetime_secs: if critical { 0.92 } else { 0.76 },
    }
}

fn camera_shake_offset(elapsed_secs: f32, duration_secs: f32, intensity: f32) -> Vec3 {
    let fade = (1.0 - elapsed_secs / duration_secs.max(0.001)).clamp(0.0, 1.0);
    Vec3::new(
        (elapsed_secs * 58.0).sin() * intensity * fade,
        (elapsed_secs * 37.0).cos() * intensity * 0.35 * fade,
        (elapsed_secs * 43.0).sin() * intensity * 0.5 * fade,
    )
}

fn collect_combat_events(
    audio: Res<GameAudio>,
    audio_settings: Res<AudioSettings>,
    time: Res<Time>,
    mut limiter: ResMut<AudioCueLimiter>,
    mut events: MessageReader<CombatEvent>,
    mut log: ResMut<CombatLog>,
) {
    for event in events.read() {
        if let Some(cue) = sound_cue_for_combat_event(&event.text)
            && limiter.should_play(cue, time.elapsed_secs())
        {
            play_sound_cue(&audio, &audio_settings, cue);
        }
        log.push_event(&event.text);
    }
}

fn play_victory_sound(audio: Res<GameAudio>, audio_settings: Res<AudioSettings>) {
    play_sound_cue(&audio, &audio_settings, SoundCue::Victory);
}

fn play_defeat_sound(audio: Res<GameAudio>, audio_settings: Res<AudioSettings>) {
    play_sound_cue(&audio, &audio_settings, SoundCue::Defeat);
}

fn play_sound_cue(audio: &GameAudio, audio_settings: &AudioSettings, cue: SoundCue) {
    if !audio_settings.enabled {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    if let Ok(mut queue) = audio.web_queue.lock() {
        queue.push(cue);
    }
    if let Some(sender) = &audio.sender {
        let _ = sender.send(cue);
    }
}

/// Looping dark-ambient bed, played through bevy_audio on every platform.
#[derive(Component)]
struct AmbientMusic;

const AMBIENT_MUSIC_VOLUME: f32 = 0.35;

fn start_ambient_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio_settings: Res<AudioSettings>,
) {
    if !audio_settings.enabled {
        return;
    }
    commands.spawn((
        bevy::audio::AudioPlayer::new(asset_server.load("audio/ambient_theme.wav")),
        bevy::audio::PlaybackSettings::LOOP
            .with_volume(bevy::audio::Volume::Linear(AMBIENT_MUSIC_VOLUME)),
        AmbientMusic,
        Name::new("Ambient Theme"),
    ));
}

/// Keep the music bed in step with the M mute toggle.
fn sync_ambient_music_mute(
    audio_settings: Res<AudioSettings>,
    mut music: Query<&mut bevy::audio::AudioSink, With<AmbientMusic>>,
) {
    for mut sink in &mut music {
        let muted = sink.is_muted();
        if audio_settings.enabled == muted {
            if muted {
                sink.unmute();
            } else {
                sink.mute();
            }
        }
    }
}

/// Web mixer: drain queued cues and play them through bevy_audio, which maps
/// to the browser's Web Audio API. Entities despawn when playback ends.
#[cfg(target_arch = "wasm32")]
fn play_web_sound_cues(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Res<GameAudio>,
    status: Res<AudioBackendStatus>,
) {
    let cues: Vec<SoundCue> = match audio.web_queue.lock() {
        Ok(mut queue) => queue.drain(..).collect(),
        Err(_) => return,
    };
    if *status != AudioBackendStatus::Ready {
        return;
    }
    for cue in cues {
        let source: Handle<bevy::audio::AudioSource> =
            asset_server.load(format!("audio/{}", sound_cue_file(cue)));
        commands.spawn((
            bevy::audio::AudioPlayer::new(source),
            bevy::audio::PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(
                sound_cue_gain(cue).clamp(0.0, 1.4),
            )),
        ));
    }
}

fn setup_audio_backend(
    audio_settings: Res<AudioSettings>,
    mut audio: ResMut<GameAudio>,
    mut status: ResMut<AudioBackendStatus>,
) {
    try_start_audio_backend(&audio_settings, &mut audio, &mut status, true);
}

fn maintain_audio_backend(
    audio_settings: Res<AudioSettings>,
    time: Res<Time>,
    mut audio: ResMut<GameAudio>,
    mut status: ResMut<AudioBackendStatus>,
    mut retry: ResMut<AudioBackendRetry>,
) {
    let retry_due = retry.timer.tick(time.delta()).just_finished();
    if !audio_backend_should_attempt(
        &audio_settings,
        *status,
        audio.sender.is_some(),
        audio_settings.is_changed(),
        retry_due,
    ) {
        if !audio_settings.enabled {
            *status = AudioBackendStatus::Muted;
        } else if audio.sender.is_some() {
            *status = AudioBackendStatus::Ready;
        }
        return;
    }

    try_start_audio_backend(&audio_settings, &mut audio, &mut status, false);
}

fn audio_backend_should_attempt(
    audio_settings: &AudioSettings,
    status: AudioBackendStatus,
    has_sender: bool,
    settings_changed: bool,
    retry_due: bool,
) -> bool {
    if !audio_settings.enabled || has_sender {
        return false;
    }
    matches!(
        status,
        AudioBackendStatus::Starting | AudioBackendStatus::Muted
    ) || settings_changed
        || (retry_due
            && matches!(
                status,
                AudioBackendStatus::NoOutputDevice | AudioBackendStatus::ThreadFailed
            ))
}

// The cue mixer runs rodio on a dedicated OS thread; the web build has neither,
// so it reports NoOutputDevice and combat cues stay silent there.
#[cfg(target_arch = "wasm32")]
fn try_start_audio_backend(
    audio_settings: &AudioSettings,
    audio: &mut GameAudio,
    status: &mut AudioBackendStatus,
    _warn_failures: bool,
) {
    *status = if !audio_settings.enabled {
        AudioBackendStatus::Muted
    } else {
        // bevy_audio drives the Web Audio API; the queue drains every frame.
        AudioBackendStatus::Ready
    };
    audio.sender = None;
}

#[cfg(not(target_arch = "wasm32"))]
fn try_start_audio_backend(
    audio_settings: &AudioSettings,
    audio: &mut GameAudio,
    status: &mut AudioBackendStatus,
    warn_failures: bool,
) {
    if !audio_settings.enabled {
        *status = AudioBackendStatus::Muted;
        return;
    }
    if audio.sender.is_some() {
        *status = AudioBackendStatus::Ready;
        return;
    }
    if rodio::OutputStream::try_default().is_err() {
        if warn_failures {
            bevy::log::warn!("Bevy Open ARPG audio: no output device available");
        }
        *status = AudioBackendStatus::NoOutputDevice;
        return;
    }

    let (sender, receiver) = mpsc::channel();
    audio.sender = Some(sender);
    let audio_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/audio");
    if let Err(error) = std::thread::Builder::new()
        .name("bevy-open-arpg-audio".to_string())
        .spawn(move || audio_thread(receiver, audio_root))
    {
        bevy::log::warn!("Failed to start audio thread: {error}");
        audio.sender = None;
        *status = AudioBackendStatus::ThreadFailed;
    } else {
        *status = AudioBackendStatus::Ready;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn audio_thread(receiver: Receiver<SoundCue>, audio_root: PathBuf) {
    let Ok((_stream, handle)) = rodio::OutputStream::try_default() else {
        bevy::log::warn!("Bevy Open ARPG audio: no output device available");
        return;
    };

    for cue in receiver {
        let path = audio_root.join(sound_cue_file(cue));
        if let Err(error) = play_audio_file(&handle, &path, sound_cue_gain(cue)) {
            bevy::log::warn!(
                "Bevy Open ARPG audio: failed to play {}: {error}",
                path.display()
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn play_audio_file(
    handle: &rodio::OutputStreamHandle,
    path: &Path,
    gain: f32,
) -> Result<(), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let source = rodio::Decoder::new(BufReader::new(file)).map_err(|error| error.to_string())?;
    let sink = rodio::Sink::try_new(handle).map_err(|error| error.to_string())?;
    sink.append(source.amplify(gain.clamp(0.0, 1.4)));
    sink.detach();
    Ok(())
}

fn sound_cue_file(cue: SoundCue) -> &'static str {
    match cue {
        SoundCue::Hit => "hit.wav",
        SoundCue::Critical => "critical.wav",
        SoundCue::Loot => "loot.wav",
        SoundCue::Danger => "danger.wav",
        SoundCue::Death => "death.wav",
        SoundCue::Skill => "skill.wav",
        SoundCue::Combo => "combo.wav",
        SoundCue::Boss => "boss.wav",
        SoundCue::Quest => "quest.wav",
        SoundCue::Potion => "potion.wav",
        SoundCue::Utility => "utility.wav",
        SoundCue::Victory => "victory.wav",
        SoundCue::Defeat => "defeat.wav",
    }
}

fn sound_cue_cooldown_secs(cue: SoundCue) -> f32 {
    match cue {
        SoundCue::Hit => 0.045,
        SoundCue::Critical => 0.08,
        SoundCue::Loot => 0.14,
        SoundCue::Danger => 0.34,
        SoundCue::Death => 0.16,
        SoundCue::Skill => 0.10,
        SoundCue::Combo => 0.18,
        SoundCue::Boss => 0.45,
        SoundCue::Quest => 0.24,
        SoundCue::Potion => 0.16,
        SoundCue::Utility => 0.20,
        SoundCue::Victory | SoundCue::Defeat => 0.0,
    }
}

fn sound_cue_gain(cue: SoundCue) -> f32 {
    match cue {
        SoundCue::Hit => 0.48,
        SoundCue::Utility => 0.52,
        SoundCue::Skill => 0.62,
        SoundCue::Loot => 0.68,
        SoundCue::Potion => 0.70,
        SoundCue::Combo => 0.78,
        SoundCue::Death => 0.82,
        SoundCue::Quest => 0.86,
        SoundCue::Critical => 0.92,
        SoundCue::Danger => 1.02,
        SoundCue::Boss => 1.12,
        SoundCue::Defeat => 1.16,
        SoundCue::Victory => 1.18,
    }
}

fn sound_cue_for_combat_event(text: &str) -> Option<SoundCue> {
    if text.contains("collapsed before")
        || text.contains("faded before")
        || text.contains("failed")
        || text.contains("Low life")
        || text.contains("Need ")
        || text.contains("INVENTORY FULL")
        || text.contains("Inventory full")
        || text.contains("MAKE ROOM")
    {
        return Some(SoundCue::Danger);
    }
    if text.contains("Boss Phase")
        || text.contains("BOSS ")
        || text.contains("Malrec Awakened")
        || text.contains("Keeper awakens")
        || text.contains("Ashen Enrage")
        || text.contains("enrages")
        || text.contains("BREAK MALREC")
    {
        return Some(SoundCue::Boss);
    }
    if text.contains("Combo Break")
        || text.contains("Combo Ready")
        || text.contains("Nephalem Surge ready")
        || text.contains("Shrine resonance")
        || text.contains("Ashen pylon reaping")
    {
        return Some(SoundCue::Combo);
    }
    if text.contains("Challenge complete")
        || text.contains("Milestone")
        || text.contains("Reliquary seal")
        || text.contains("Final seal")
        || text.contains("Final Seal")
        || text.contains("Quest complete")
        || text.contains("Chapter reward")
    {
        return Some(SoundCue::Quest);
    }
    if text.contains("Nephalem Surge unleashed")
        || text.contains("Ashen pylon:")
        || text.contains("Critical")
        || text.contains("staggered")
        || text.contains("EXECUTE WINDOW")
        || text.contains("Execute window")
        || text.contains("Stormbrand chained")
        || text.contains("Legendary")
        || text.contains("legendary")
        || text.contains("Ancient")
        || text.contains("ancient")
        || text.contains("Primal")
        || text.contains("primal")
        || text.contains("BUILD POWER")
        || text.contains("POWER SPIKE")
        || text.contains("Level up")
        || text.contains("Level ")
    {
        return Some(SoundCue::Critical);
    }
    if text.contains("Potion restored")
        || text.contains("Renewal well restored")
        || text.contains("Elixir")
        || text.contains("Soulreaver restored")
        || text.contains("Aegisbrand granted")
    {
        return Some(SoundCue::Potion);
    }
    if text.contains(" Dash")
        || text.contains(" Rupture")
        || text.contains(" Nova")
        || text == "Evade"
        || text.contains("Relic shrine")
        || text.contains("Storm shrine")
        || text.contains("conduit lightning")
        || text.contains("Town portal")
        || text.contains("Sentinel")
    {
        return Some(SoundCue::Skill);
    }
    if text.contains("Cinder bolt")
        || text.contains("Arcane")
        || text.contains("Jailer")
        || text.contains("Frozen")
        || text.contains("Desecrator")
        || text.contains("Reflective")
        || text.contains("Molten")
        || text.contains("Hazard")
        || text.contains("burning")
    {
        return Some(SoundCue::Danger);
    }
    if text.contains("opened:") || text.contains("awakened:") {
        return Some(SoundCue::Utility);
    }
    if text.contains("Picked up")
        || text.contains("Equipped")
        || text.contains("Recovered")
        || text.contains("reward")
        || text.contains("Reward")
        || text.contains("cache")
        || text.contains("Cache")
        || text.contains("Bounty")
        || text.contains("opened")
        || text.contains("sealed")
        || text.contains("completed:")
        || text.contains("Quartermaster")
        || text.contains("Armory")
        || text.contains("Codex")
        || text.contains("Fortune shrine")
    {
        return Some(SoundCue::Loot);
    }
    if text.contains("recharging")
        || text.contains("No weapons")
        || text.contains("failed")
        || text.contains("not ready")
        || text.contains("not enough")
        || text.contains("evaded")
        || text.contains("absorbed by ward")
        || text.contains("Nephalem Surge charging")
        || text.contains("Nephalem Surge extended")
        || text.contains("Dash rune")
        || text.contains("Nova rune")
        || text.contains("Rupture rune")
        || text.contains("checkpoint attuned")
    {
        return Some(SoundCue::Utility);
    }
    if text.contains("slain") || text.contains("death pool") {
        return Some(SoundCue::Death);
    }
    if text.contains("hit for") || text.contains("ignited") || text.contains("chilled") {
        return Some(SoundCue::Hit);
    }
    None
}

fn spawn_floating_combat_text(
    mut commands: Commands,
    mut events: MessageReader<FloatingCombatTextEvent>,
) {
    for event in events.read() {
        let style = floating_text_style(&event.text, event.critical);
        commands.spawn((
            Text2d::new(event.text.clone()),
            TextFont {
                font_size: FontSize::Px(style.font_size),
                ..default()
            },
            TextColor(style.color),
            Transform::from_translation(event.position + Vec3::Y * 1.55).with_scale(style.scale),
            FloatingCombatText {
                timer: Timer::from_seconds(style.lifetime_secs, TimerMode::Once),
                lifetime_secs: style.lifetime_secs,
                velocity: style.velocity,
                base_scale: style.scale,
                color: style.color,
            },
        ));
    }
}

fn update_floating_combat_text(
    time: Res<Time>,
    mut commands: Commands,
    camera: Query<&Transform, (With<Camera3d>, Without<FloatingCombatText>)>,
    mut texts: Query<(
        Entity,
        &mut FloatingCombatText,
        &mut Transform,
        &mut TextColor,
    )>,
) {
    let camera_rotation = camera.single().ok().map(|transform| transform.rotation);
    for (entity, mut floating, mut transform, mut text_color) in &mut texts {
        floating.timer.tick(time.delta());
        if floating.timer.is_finished() {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.try_despawn();
            }
            continue;
        }
        let elapsed_ratio =
            (floating.timer.elapsed_secs() / floating.lifetime_secs).clamp(0.0, 1.0);
        transform.translation += floating.velocity * time.delta_secs();
        transform.scale = floating.base_scale * (1.0 + elapsed_ratio * 0.35);
        if let Some(rotation) = camera_rotation {
            transform.rotation = rotation;
        }
        text_color.0 = floating.color.with_alpha(1.0 - elapsed_ratio);
    }
}

fn attach_enemy_health_bars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    enemies: Query<(Entity, &Enemy), Without<EnemyHealthBar>>,
) {
    let background_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.01, 0.01),
        emissive: Color::srgb(0.04, 0.0, 0.0).into(),
        ..default()
    });
    let background_mesh = meshes.add(Cuboid::new(0.92, 0.06, 0.08));
    let fill_mesh = meshes.add(Cuboid::new(0.86, 0.07, 0.09));
    let focus_ring_mesh = meshes.add(Torus::new(0.58, 0.035));
    let focus_ring_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.78, 0.18, 0.78),
        emissive: Color::srgb(1.0, 0.45, 0.08).into(),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    for (enemy_entity, enemy) in &enemies {
        let profile = enemy_bar_profile(enemy);
        let fill_color = enemy_health_fill_color(1.0, enemy, None, None, None);
        let fill_material = materials.add(StandardMaterial {
            base_color: fill_color,
            emissive: fill_color.into(),
            ..default()
        });
        let fill = commands
            .spawn((
                Mesh3d(fill_mesh.clone()),
                MeshMaterial3d(fill_material),
                Transform::from_xyz(0.0, profile.bar_height + 0.02, 0.0).with_scale(Vec3::new(
                    profile.width,
                    1.0,
                    1.0,
                )),
                EnemyHealthBarFill,
                EnemyEntity,
                Name::new("Enemy Health Fill"),
            ))
            .id();
        let background = commands
            .spawn((
                Mesh3d(background_mesh.clone()),
                MeshMaterial3d(background_material.clone()),
                Transform::from_xyz(0.0, profile.bar_height, 0.0).with_scale(Vec3::new(
                    profile.width,
                    profile.thickness,
                    1.0,
                )),
                EnemyHealthBarBackground,
                EnemyEntity,
                Name::new("Enemy Health Bar"),
            ))
            .id();
        let focus_ring = commands
            .spawn((
                Mesh3d(focus_ring_mesh.clone()),
                MeshMaterial3d(focus_ring_material.clone()),
                Transform::from_xyz(0.0, 0.075, 0.0)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::splat(enemy_focus_ring_base_scale(enemy))),
                Visibility::Hidden,
                EnemyFocusRing,
                EnemyEntity,
                Name::new("Enemy Target Focus Ring"),
            ))
            .id();
        let nameplate = commands
            .spawn((
                Text2d::new(enemy_nameplate_text(enemy)),
                TextFont {
                    font_size: FontSize::Px(enemy_nameplate_font_size(enemy)),
                    ..default()
                },
                TextColor(enemy_nameplate_color(enemy)),
                Transform::from_xyz(0.0, profile.name_height, 0.0).with_scale(Vec3::splat(0.011)),
                EnemyNameplate {
                    enemy: enemy_entity,
                },
                EnemyEntity,
                Name::new("Enemy Nameplate"),
            ))
            .id();
        attach_child_or_cleanup(&mut commands, enemy_entity, background);
        attach_child_or_cleanup(&mut commands, enemy_entity, fill);
        attach_child_or_cleanup(&mut commands, enemy_entity, focus_ring);
        attach_child_or_cleanup(&mut commands, enemy_entity, nameplate);
        commands.entity(enemy_entity).try_insert(EnemyHealthBar {
            background,
            fill,
            nameplate,
            focus_ring,
            base_width: profile.width,
        });
    }
}

fn attach_child_or_cleanup(commands: &mut Commands, parent: Entity, child: Entity) {
    commands.queue(move |world: &mut World| {
        attach_child_or_cleanup_world(world, parent, child);
    });
}

fn attach_child_or_cleanup_world(world: &mut World, parent: Entity, child: Entity) {
    if world.get_entity(parent).is_ok() && world.get_entity(child).is_ok() {
        world.entity_mut(child).insert(ChildOf(parent));
    } else if let Ok(entity) = world.get_entity_mut(child) {
        entity.despawn();
    }
}

#[derive(Debug, Clone, Copy)]
struct EnemyBarProfile {
    width: f32,
    thickness: f32,
    bar_height: f32,
    name_height: f32,
}

fn enemy_bar_profile(enemy: &Enemy) -> EnemyBarProfile {
    if enemy.id == "keeper" {
        EnemyBarProfile {
            width: 1.95,
            thickness: 1.35,
            bar_height: 3.12,
            name_height: 3.48,
        }
    } else if enemy.id == "seal_warden" || enemy.id == "treasure_imp" || enemy.affixes.len() >= 2 {
        EnemyBarProfile {
            width: 1.35,
            thickness: 1.16,
            bar_height: 2.55,
            name_height: 2.86,
        }
    } else if !enemy.affixes.is_empty() {
        EnemyBarProfile {
            width: 1.15,
            thickness: 1.08,
            bar_height: 2.42,
            name_height: 2.70,
        }
    } else {
        EnemyBarProfile {
            width: 0.92,
            thickness: 1.0,
            bar_height: 2.22,
            name_height: 2.48,
        }
    }
}

fn enemy_nameplate_text(enemy: &Enemy) -> String {
    enemy_nameplate_text_with_status(enemy, None, None, None)
}

fn enemy_nameplate_text_with_status(
    enemy: &Enemy,
    health: Option<&Health>,
    chilled: Option<&Chilled>,
    staggered: Option<&Staggered>,
) -> String {
    let tier = enemy_tier_label(enemy);
    let status = enemy_status_label(chilled, staggered);
    let health_suffix = health
        .map(|health| {
            format!(
                " {:.0}%",
                (health.current / health.max * 100.0).clamp(0.0, 100.0)
            )
        })
        .unwrap_or_default();
    if status.is_empty() {
        format!("{tier} {}{health_suffix}", enemy.display_name)
    } else {
        format!("{tier} {} [{status}]{health_suffix}", enemy.display_name)
    }
}

fn enemy_nameplate_font_size(enemy: &Enemy) -> f32 {
    if enemy.id == "keeper" {
        22.0
    } else if enemy.id == "treasure_imp" || !enemy.affixes.is_empty() {
        18.0
    } else {
        14.0
    }
}

fn enemy_tier_label(enemy: &Enemy) -> &'static str {
    if enemy.id == "keeper" {
        "BOSS"
    } else if enemy.id == "seal_warden" {
        "WARDEN"
    } else if enemy.id == "treasure_imp" {
        "VAULT"
    } else if enemy.affixes.len() >= 2 {
        "ELITE"
    } else if !enemy.affixes.is_empty() {
        "RARE"
    } else {
        ""
    }
}

fn enemy_status_label(chilled: Option<&Chilled>, staggered: Option<&Staggered>) -> String {
    let mut labels = Vec::new();
    if staggered.is_some() {
        labels.push("stagger");
    }
    if chilled.is_some() {
        labels.push("chill");
    }
    labels.join(" ")
}

fn enemy_nameplate_color(enemy: &Enemy) -> Color {
    if enemy.id == "keeper" {
        Color::srgb(1.0, 0.38, 0.12)
    } else if enemy.id == "seal_warden" {
        Color::srgb(0.96, 0.56, 1.0)
    } else if enemy.id == "treasure_imp" {
        Color::srgb(1.0, 0.82, 0.22)
    } else if enemy.affixes.len() >= 2 {
        Color::srgb(0.82, 0.38, 1.0)
    } else if !enemy.affixes.is_empty() {
        Color::srgb(0.38, 0.62, 1.0)
    } else {
        Color::srgb(0.86, 0.82, 0.72)
    }
}

fn enemy_health_fill_color(
    ratio: f32,
    enemy: &Enemy,
    chilled: Option<&Chilled>,
    staggered: Option<&Staggered>,
    boss_phase: Option<&BossPhase>,
) -> Color {
    if staggered.is_some() {
        return Color::srgb(1.0, 0.82, 0.18);
    }
    if chilled.is_some() {
        return Color::srgb(0.35, 0.76, 1.0);
    }
    if boss_phase.is_some_and(BossPhase::enrage_started) {
        return Color::srgb(1.0, 0.08, 0.02);
    }
    if ratio <= 0.25 {
        return Color::srgb(0.95, 0.16, 0.08);
    }
    if enemy.id == "keeper" {
        Color::srgb(0.95, 0.34, 0.08)
    } else if enemy.id == "seal_warden" {
        Color::srgb(0.90, 0.32, 1.0)
    } else if enemy.affixes.len() >= 2 {
        Color::srgb(0.76, 0.20, 1.0)
    } else if enemy.affixes.contains(&EnemyAffix::Shielded) {
        Color::srgb(0.84, 0.70, 0.22)
    } else if !enemy.affixes.is_empty() {
        Color::srgb(0.24, 0.48, 1.0)
    } else {
        Color::srgb(0.86, 0.04, 0.04)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum EnemyBarVisibility {
    Hidden,
    Visible,
}

impl EnemyBarVisibility {
    fn visibility(self) -> Visibility {
        match self {
            Self::Hidden => Visibility::Hidden,
            Self::Visible => Visibility::Visible,
        }
    }
}

fn enemy_bar_visibility(
    ratio: f32,
    enemy: &Enemy,
    chilled: Option<&Chilled>,
    staggered: Option<&Staggered>,
    boss_phase: Option<&BossPhase>,
    hovered: bool,
) -> EnemyBarVisibility {
    let damaged = ratio < 0.995;
    let controlled = chilled.is_some() || staggered.is_some();
    let priority = enemy.id == "keeper"
        || enemy.id == "seal_warden"
        || enemy.id == "treasure_imp"
        || !enemy.affixes.is_empty()
        || boss_phase.is_some();
    if hovered || damaged || controlled || priority {
        EnemyBarVisibility::Visible
    } else {
        EnemyBarVisibility::Hidden
    }
}

fn enemy_bar_fill_thickness_scale(ratio: f32, enemy: &Enemy, staggered: Option<&Staggered>) -> f32 {
    let low_health = if ratio <= 0.25 { 1.22 } else { 1.0 };
    let priority = if enemy.id == "keeper" || enemy.id == "seal_warden" || enemy.affixes.len() >= 2
    {
        1.12
    } else {
        1.0
    };
    let stagger = if staggered.is_some() { 1.18 } else { 1.0 };
    low_health * priority * stagger
}

fn update_enemy_health_bars(
    mut materials: ResMut<Assets<StandardMaterial>>,
    hover: Res<EnemyHoverTarget>,
    enemies: EnemyBarQuery,
    mut fills: EnemyBarFillQuery,
    mut backgrounds: EnemyBarBackgroundQuery,
) {
    for (entity, health, enemy, bar, chilled, staggered, boss_phase) in &enemies {
        let Ok((mut fill_transform, material, mut fill_visibility)) = fills.get_mut(bar.fill)
        else {
            continue;
        };
        let ratio = (health.current / health.max).clamp(0.0, 1.0);
        let visibility = enemy_bar_visibility(
            ratio,
            enemy,
            chilled,
            staggered,
            boss_phase,
            hover.0 == Some(entity),
        )
        .visibility();
        fill_transform.scale.x = bar.base_width * ratio;
        fill_transform.scale.y = enemy_bar_fill_thickness_scale(ratio, enemy, staggered);
        fill_transform.translation.x = -0.43 * bar.base_width * (1.0 - ratio);
        *fill_visibility = visibility;
        if let Ok(mut background_visibility) = backgrounds.get_mut(bar.background) {
            *background_visibility = visibility;
        }
        let color = enemy_health_fill_color(ratio, enemy, chilled, staggered, boss_phase);
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.base_color = color;
            material.emissive = color.into();
        }
    }
}

fn update_enemy_nameplates(
    camera: Query<&Transform, (With<Camera3d>, Without<EnemyNameplate>)>,
    hover: Res<EnemyHoverTarget>,
    enemies: EnemyNameplateEnemyQuery,
    mut nameplates: EnemyNameplateVisualQuery,
) {
    let Some(rotation) = camera.single().ok().map(|transform| transform.rotation) else {
        return;
    };
    for (entity, nameplate, mut transform, mut text, mut color, mut visibility) in &mut nameplates {
        transform.rotation = rotation;
        let Ok((enemy, health, bar, chilled, staggered, boss_phase)) = enemies.get(nameplate.enemy)
        else {
            continue;
        };
        if bar.nameplate != entity {
            continue;
        }
        let ratio = (health.current / health.max).clamp(0.0, 1.0);
        *visibility = enemy_bar_visibility(
            ratio,
            enemy,
            chilled,
            staggered,
            boss_phase,
            hover.0 == Some(nameplate.enemy),
        )
        .visibility();
        *text = Text2d::new(enemy_nameplate_text_with_status(
            enemy,
            Some(health),
            chilled,
            staggered,
        ));
        color.0 = if staggered.is_some() {
            Color::srgb(1.0, 0.84, 0.22)
        } else if chilled.is_some() {
            Color::srgb(0.42, 0.78, 1.0)
        } else {
            enemy_nameplate_color(enemy)
        };
    }
}

fn update_enemy_hover_target(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    enemies: EnemyHoverQuery,
    mut hover: ResMut<EnemyHoverTarget>,
) {
    hover.0 = hovered_enemy_from_cursor(cursor_ground_point(&windows, &cameras), enemies.iter());
}

fn update_target_info(
    hover: Res<EnemyHoverTarget>,
    intent: Res<BasicAttackIntent>,
    enemies: TargetInfoEnemyQuery,
    mut target_info: ResMut<TargetInfo>,
) {
    let hovered = hover
        .0
        .and_then(|entity| enemies.get(entity).ok())
        .filter(|(_, _, health, ..)| health.current > 0.0);
    let intended = intent
        .target
        .and_then(|entity| enemies.get(entity).ok())
        .filter(|(_, _, health, ..)| health.current > 0.0);
    let focused = enemies
        .iter()
        .filter(|(_, _, health, ..)| health.current > 0.0)
        .filter_map(|target| target.7.map(|focus| (target, focus.intensity())))
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(target, _)| target);

    let Some((_, enemy, health, chilled, staggered, boss_phase, ward, _)) =
        hovered.or(intended).or(focused)
    else {
        *target_info = TargetInfo::default();
        return;
    };

    *target_info = target_info_from_enemy(enemy, health, chilled, staggered, boss_phase, ward);
}

fn update_enemy_focus_targets(
    time: Res<Time>,
    mut commands: Commands,
    hover: Res<EnemyHoverTarget>,
    intent: Res<BasicAttackIntent>,
    mut enemies: EnemyFocusQuery,
    mut rings: EnemyFocusRingQuery,
) {
    for (entity, enemy, _transform, _health, bar, focus) in &mut enemies {
        let Ok((mut ring_transform, mut ring_visibility)) = rings.get_mut(bar.focus_ring) else {
            continue;
        };
        if let Some(mut focus) = focus {
            focus.tick(time.delta());
            if focus.is_finished() {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.remove::<EnemyTargetFocus>();
                }
                *ring_visibility = Visibility::Hidden;
                continue;
            }
            let pose = enemy_focus_ring_pose(enemy, focus.intensity(), focus.critical());
            ring_transform.translation = pose.translation;
            ring_transform.scale = pose.scale;
            *ring_visibility = pose.visibility;
        } else if intent.target == Some(entity) {
            let pose = enemy_intent_focus_ring_pose(enemy, time.elapsed_secs());
            ring_transform.translation = pose.translation;
            ring_transform.scale = pose.scale;
            *ring_visibility = pose.visibility;
        } else if hover.0 == Some(entity) {
            let pose = enemy_hover_focus_ring_pose(enemy, time.elapsed_secs());
            ring_transform.translation = pose.translation;
            ring_transform.scale = pose.scale;
            *ring_visibility = pose.visibility;
        } else {
            *ring_visibility = Visibility::Hidden;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EnemyFocusRingPose {
    translation: Vec3,
    scale: Vec3,
    visibility: Visibility,
}

fn cursor_ground_point(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<Vec3> {
    let window = windows.single().ok()?;
    let cursor_position = window.cursor_position()?;
    let (camera, camera_transform) = cameras.single().ok()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()?;
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
    let mut point = ray.get_point(distance);
    point.y = 0.0;
    Some(point)
}

fn hovered_enemy_from_cursor<'a>(
    cursor_ground: Option<Vec3>,
    enemies: impl Iterator<Item = (Entity, &'a Enemy, &'a Transform, &'a Health)>,
) -> Option<Entity> {
    let cursor_ground = cursor_ground?;
    enemies
        .filter(|(_, enemy, transform, health)| {
            health.current > 0.0
                && cursor_targets_enemy(cursor_ground, transform.translation, enemy)
        })
        .min_by(|(_, _, left_transform, _), (_, _, right_transform, _)| {
            flat_distance(cursor_ground, left_transform.translation)
                .total_cmp(&flat_distance(cursor_ground, right_transform.translation))
        })
        .map(|(entity, ..)| entity)
}

fn cursor_targets_enemy(cursor_ground: Vec3, enemy_position: Vec3, enemy: &Enemy) -> bool {
    flat_distance(cursor_ground, enemy_position) <= enemy_cursor_pick_radius(enemy)
}

fn flat_distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

fn target_info_from_enemy(
    enemy: &Enemy,
    health: &Health,
    chilled: Option<&Chilled>,
    staggered: Option<&Staggered>,
    boss_phase: Option<&BossPhase>,
    ward: Option<&SealWardenWard>,
) -> TargetInfo {
    let health_percent = (health.current / health.max).clamp(0.0, 1.0) * 100.0;
    TargetInfo {
        visible: true,
        name: enemy.display_name.clone(),
        subtitle: enemy_target_subtitle(enemy, boss_phase),
        health_line: format!(
            "{:.0}/{:.0} HP  ({:.0}%)",
            health.current.max(0.0),
            health.max.max(1.0),
            health_percent
        ),
        details: enemy_target_details(enemy, chilled, staggered, boss_phase, ward),
        health_percent,
        threat_color: enemy_nameplate_color(enemy),
    }
}

fn enemy_target_subtitle(enemy: &Enemy, boss_phase: Option<&BossPhase>) -> String {
    if let Some(phase) = boss_phase {
        let phase_label = if phase.enrage_started() {
            "BOSS - ENRAGED"
        } else if phase.phase_two_started() {
            "BOSS - PHASE II"
        } else {
            "BOSS - PHASE I"
        };
        return phase_label.to_string();
    }
    let tier = enemy_tier_label(enemy);
    let role = enemy_role_label(enemy);
    let attack = enemy_attack_kind_label(&enemy.attack_kind);
    if let Some(role) = role {
        return if tier.is_empty() {
            format!("{role} - {attack}")
        } else if tier.eq_ignore_ascii_case(role) {
            format!("{tier} - {attack}")
        } else {
            format!("{tier} {role} - {attack}")
        };
    }
    if tier.is_empty() {
        attack.to_string()
    } else {
        format!("{tier} - {attack}")
    }
}

fn enemy_role_label(enemy: &Enemy) -> Option<&'static str> {
    match enemy.id.as_str() {
        "skeleton" => Some("Guard"),
        "bone_stalker" => Some("Chaser"),
        "cultist" => Some("Caster"),
        "seal_warden" => Some("Warden"),
        "ashen_marksman" => Some("Marksman"),
        "reliquary_brute" => Some("Heavy"),
        "treasure_imp" => Some("Treasure"),
        "keeper" => Some("Boss"),
        "nemesis" => Some("Nemesis"),
        _ => None,
    }
}

fn enemy_attack_kind_label(kind: &EnemyAttackKind) -> &'static str {
    match kind {
        EnemyAttackKind::Melee => "Melee",
        EnemyAttackKind::Projectile => "Projectile",
        EnemyAttackKind::Shockwave => "Shockwave",
    }
}

fn enemy_target_details(
    enemy: &Enemy,
    chilled: Option<&Chilled>,
    staggered: Option<&Staggered>,
    boss_phase: Option<&BossPhase>,
    ward: Option<&SealWardenWard>,
) -> String {
    let mut parts = Vec::new();
    if enemy.affixes.is_empty() {
        parts.push("no affixes".to_string());
    } else {
        parts.push(
            enemy
                .affixes
                .iter()
                .map(|affix| affix.label())
                .collect::<Vec<_>>()
                .join(" / "),
        );
        parts.push(enemy_affix_threat_summary(enemy));
        parts.push(enemy_affix_reaction_summary(enemy));
    }
    if let Some(tactic) = enemy_role_tactic(enemy) {
        parts.push(tactic);
    }
    if let Some(ward) = ward {
        parts.push(seal_warden_ward_detail(ward));
    }
    let status = enemy_status_label(chilled, staggered);
    if !status.is_empty() {
        parts.push(status);
    }
    if let Some(threat_action) = enemy_threat_action_summary(enemy, staggered, boss_phase) {
        parts.push(threat_action);
    }
    if let Some(phase) = boss_phase
        && phase.phase_two_started()
        && !phase.enrage_started()
    {
        parts.push(format!("enrage {:.0}s", phase.enrage_remaining_secs()));
    }
    parts.join(" | ")
}

fn seal_warden_ward_detail(ward: &SealWardenWard) -> String {
    if ward.broken {
        "ward broken: punish".to_string()
    } else {
        format!("ward {:.0}/{:.0}", ward.current.max(0.0), ward.max.max(1.0))
    }
}

fn enemy_role_tactic(enemy: &Enemy) -> Option<String> {
    let role = match enemy.id.as_str() {
        "skeleton" => "guard",
        "bone_stalker" => "chaser",
        "cultist" => "caster",
        "seal_warden" => "warden",
        "ashen_marksman" => "marksman",
        "reliquary_brute" => "heavy",
        "treasure_imp" => "treasure",
        "keeper" => "boss",
        "nemesis" => "nemesis",
        _ => return None,
    };
    Some(format!(
        "role: {role} | tip: {}",
        chapter_enemy_counterplay(&enemy.id)
    ))
}

fn enemy_threat_action_summary(
    enemy: &Enemy,
    staggered: Option<&Staggered>,
    boss_phase: Option<&BossPhase>,
) -> Option<String> {
    if staggered.is_some() {
        return Some("window: burst now".to_string());
    }
    if let Some(phase) = boss_phase {
        if phase.enrage_started() {
            return Some("danger: kite fire, burst after slam".to_string());
        }
        if phase.phase_two_started() && phase.enrage_remaining_secs() <= 6.0 {
            return Some("danger: break before enrage".to_string());
        }
    }
    let remaining = enemy.attack_timer.remaining_secs();
    if enemy.id == "seal_warden" && remaining <= enemy_attack_imminent_window(&enemy.attack_kind) {
        return Some(format!(
            "incoming: leave seal rune {:.1}s",
            remaining.max(0.0)
        ));
    }
    if remaining <= enemy_attack_imminent_window(&enemy.attack_kind) {
        return Some(format!(
            "incoming: {} {:.1}s",
            enemy_attack_response_label(&enemy.attack_kind),
            remaining.max(0.0)
        ));
    }
    if enemy.affixes.len() >= 2 {
        return Some("plan: control elite, then burst".to_string());
    }
    None
}

fn enemy_attack_imminent_window(kind: &EnemyAttackKind) -> f32 {
    match kind {
        EnemyAttackKind::Melee => 0.32,
        EnemyAttackKind::Projectile => 0.48,
        EnemyAttackKind::Shockwave => 0.62,
    }
}

fn enemy_attack_response_label(kind: &EnemyAttackKind) -> &'static str {
    match kind {
        EnemyAttackKind::Melee => "sidestep melee",
        EnemyAttackKind::Projectile => "strafe shot",
        EnemyAttackKind::Shockwave => "dodge wave",
    }
}

fn enemy_affix_threat_summary(enemy: &Enemy) -> String {
    let threats = enemy
        .affixes
        .iter()
        .map(|affix| enemy_affix_threat_label(*affix))
        .collect::<Vec<_>>()
        .join(", ");
    format!("threat: {threats}")
}

fn enemy_affix_reaction_summary(enemy: &Enemy) -> String {
    let reactions = enemy
        .affixes
        .iter()
        .map(|affix| enemy_affix_reaction_label(*affix))
        .collect::<Vec<_>>();
    let mut reactions = dedupe_labels(reactions);
    reactions.truncate(3);
    format!("react: {}", reactions.join(", "))
}

fn dedupe_labels(labels: Vec<&'static str>) -> Vec<&'static str> {
    labels.into_iter().fold(Vec::new(), |mut unique, label| {
        if !unique.contains(&label) {
            unique.push(label);
        }
        unique
    })
}

fn enemy_affix_threat_label(affix: EnemyAffix) -> &'static str {
    match affix {
        EnemyAffix::Frenzied => "fast engage",
        EnemyAffix::Vampiric => "lifesteal",
        EnemyAffix::Molten => "death pool",
        EnemyAffix::Shielded => "shield window",
        EnemyAffix::Arcane => "beam hazard",
        EnemyAffix::Jailer => "root trap",
        EnemyAffix::Frozen => "freeze burst",
        EnemyAffix::Desecrator => "ground fire",
        EnemyAffix::Reflective => "reflect",
    }
}

fn enemy_affix_reaction_label(affix: EnemyAffix) -> &'static str {
    match affix {
        EnemyAffix::Frenzied => "kite first swing",
        EnemyAffix::Vampiric => "burst or disengage",
        EnemyAffix::Molten => "step off death pool",
        EnemyAffix::Shielded => "wait shield, then burst",
        EnemyAffix::Arcane => "strafe beam",
        EnemyAffix::Jailer => "save Shift",
        EnemyAffix::Frozen => "leave circle",
        EnemyAffix::Desecrator => "move out",
        EnemyAffix::Reflective => "stop into reflect",
    }
}

fn enemy_focus_ring_pose(enemy: &Enemy, intensity: f32, critical: bool) -> EnemyFocusRingPose {
    let intensity = intensity.clamp(0.0, 1.0);
    let base = enemy_focus_ring_base_scale(enemy);
    let critical_boost = if critical { 0.18 } else { 0.0 };
    let pulse = 0.10 + intensity * (0.22 + critical_boost);
    EnemyFocusRingPose {
        translation: Vec3::new(0.0, 0.075, 0.0),
        scale: Vec3::splat(base * (1.0 + pulse)),
        visibility: if intensity > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    }
}

fn enemy_hover_focus_ring_pose(enemy: &Enemy, elapsed_secs: f32) -> EnemyFocusRingPose {
    let base = enemy_focus_ring_base_scale(enemy);
    let pulse = 0.10 + elapsed_secs.mul_add(4.2, 0.0).sin().abs() * 0.08;
    EnemyFocusRingPose {
        translation: Vec3::new(0.0, 0.075, 0.0),
        scale: Vec3::splat(base * (1.0 + pulse)),
        visibility: Visibility::Visible,
    }
}

fn enemy_intent_focus_ring_pose(enemy: &Enemy, elapsed_secs: f32) -> EnemyFocusRingPose {
    let base = enemy_focus_ring_base_scale(enemy);
    let pulse = 0.16 + elapsed_secs.mul_add(6.4, 0.0).sin().abs() * 0.07;
    EnemyFocusRingPose {
        translation: Vec3::new(0.0, 0.075, 0.0),
        scale: Vec3::splat(base * (1.0 + pulse)),
        visibility: Visibility::Visible,
    }
}

fn enemy_focus_ring_base_scale(enemy: &Enemy) -> f32 {
    if enemy.id == "keeper" {
        1.68
    } else if enemy.id == "treasure_imp" || enemy.affixes.len() >= 2 {
        1.30
    } else if !enemy.affixes.is_empty() {
        1.16
    } else {
        0.98
    }
}

fn update_camera_shake(
    time: Res<Time>,
    mut events: MessageReader<ScreenShakeEvent>,
    mut state: ResMut<CameraShakeState>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    for event in events.read() {
        if state.base_translation.is_none() {
            state.base_translation = Some(transform.translation);
        }
        if event.intensity >= state.intensity {
            state.timer = Some(Timer::from_seconds(
                event.duration_secs.max(0.01),
                TimerMode::Once,
            ));
            state.duration_secs = event.duration_secs.max(0.01);
            state.intensity = event.intensity.max(0.0);
        }
    }

    let base = state.base_translation.unwrap_or(transform.translation);
    let duration_secs = state.duration_secs;
    let intensity = state.intensity;
    let Some(timer) = state.timer.as_mut() else {
        return;
    };
    timer.tick(time.delta());
    if timer.is_finished() {
        transform.translation = base;
        state.timer = None;
        state.base_translation = None;
        state.duration_secs = 0.0;
        state.intensity = 0.0;
        return;
    }

    transform.translation =
        base + camera_shake_offset(timer.elapsed_secs(), duration_secs, intensity);
}

fn update_hit_stop(
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut state: ResMut<HitStopState>,
    mut events: MessageReader<HitStopEvent>,
) {
    for event in events.read() {
        if event.duration_secs >= state.remaining_secs {
            state.remaining_secs = event.duration_secs;
            state.duration_secs = event.duration_secs;
            state.relative_speed = event.relative_speed;
        }
    }

    if state.remaining_secs <= 0.0 {
        if (virtual_time.relative_speed() - 1.0).abs() > f32::EPSILON {
            virtual_time.set_relative_speed(1.0);
        }
        return;
    }

    state.remaining_secs = (state.remaining_secs - real_time.delta_secs()).max(0.0);
    if state.remaining_secs <= 0.0 {
        virtual_time.set_relative_speed(1.0);
        return;
    }

    virtual_time.set_relative_speed(hit_stop_recovery_speed(
        state.relative_speed,
        state.remaining_secs,
        state.duration_secs,
    ));
}

fn hit_stop_recovery_speed(relative_speed: f32, remaining_secs: f32, duration_secs: f32) -> f32 {
    let relative_speed = relative_speed.clamp(0.16, 0.82);
    let recovery = 1.0 - remaining_secs / duration_secs.max(0.001);
    let eased_recovery = recovery.clamp(0.0, 1.0).powi(2);
    relative_speed + (1.0 - relative_speed) * eased_recovery
}

fn reset_hit_stop(mut virtual_time: ResMut<Time<Virtual>>, mut state: ResMut<HitStopState>) {
    *state = HitStopState::default();
    virtual_time.set_relative_speed(1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_enemy(id: &str, affixes: Vec<crate::enemy::EnemyAffix>) -> Enemy {
        Enemy {
            id: id.to_string(),
            display_name: "Target".to_string(),
            affixes,
            attack_damage: 1.0,
            attack_kind: crate::data::EnemyAttackKind::Melee,
            attack_range: 1.0,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 1.0,
            move_speed: 1.0,
            gold_min: 1,
            gold_max: 1,
            xp_reward: 1,
        }
    }

    #[test]
    fn combat_log_keeps_recent_entries_only() {
        let mut log = CombatLog::default();
        for index in 0..10 {
            log.push(format!("event {index}"));
        }
        let lines = log.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), LOG_CAPACITY);
        assert_eq!(lines[0], "event 9");
        assert_eq!(lines[LOG_CAPACITY - 1], "event 4");
    }

    #[test]
    fn combat_log_event_filter_keeps_progress_and_drops_transient_spam() {
        let mut log = CombatLog::default();
        for event in [
            "Generated 18 fury",
            "Picked up rare Moonforged Cleaver (+9 damage)",
            "Critical basic hit for 12",
            "Combo Break: Rupture into Nova hit 3 targets +17 fury",
            "Potion restored 45 health",
            "Reap Dash",
            "Main Quest Complete: the Ashen Reliquary is cleansed",
            "HUD mode: clean",
            "Armory 1 saved: Stormcall Reliquary Brand",
        ] {
            log.push_event(event);
        }

        let lines = log.lines().collect::<Vec<_>>();
        assert_eq!(
            lines,
            vec![
                "Armory 1 saved: Stormcall Reliquary Brand",
                "HUD mode: clean",
                "Main Quest Complete: the Ashen Reliquary is cleansed",
            ]
        );
    }

    #[test]
    fn hit_stop_events_clamp_and_recover_toward_normal_time() {
        let event = HitStopEvent::new(0.02, 0.5);

        assert_eq!(event.relative_speed, 0.16);
        assert_eq!(event.duration_secs, 0.12);

        let early = hit_stop_recovery_speed(0.25, 0.09, 0.10);
        let late = hit_stop_recovery_speed(0.25, 0.01, 0.10);

        assert!(early < 0.35);
        assert!(late > 0.85);
        assert_eq!(hit_stop_recovery_speed(0.02, 0.10, 0.10), 0.16);
    }

    #[test]
    fn floating_combat_text_styles_critical_hits_larger() {
        assert!(floating_text_font_size(true) > floating_text_font_size(false));
        assert_ne!(floating_text_color(true), floating_text_color(false));
        let normal = floating_text_style("20", false);
        let critical = floating_text_style("CRIT 35", true);
        let elite = floating_text_style("ELITE SLAY", true);
        let boss = floating_text_style("BOSS DOWN", true);

        assert!(critical.font_size > normal.font_size);
        assert!(elite.font_size > critical.font_size);
        assert!(boss.font_size > elite.font_size);
        assert!(boss.lifetime_secs > normal.lifetime_secs);
        assert!(elite.scale.x > critical.scale.x);
    }

    #[test]
    fn camera_shake_offset_fades_to_zero() {
        let early = camera_shake_offset(0.05, 0.4, 0.2).length();
        let late = camera_shake_offset(0.38, 0.4, 0.2).length();

        assert!(early > late);
        assert_eq!(camera_shake_offset(0.4, 0.4, 0.2), Vec3::ZERO);
    }

    #[test]
    fn combat_event_text_maps_to_distinct_sound_cues() {
        assert_eq!(
            sound_cue_for_combat_event("Critical Strike hit for 42"),
            Some(SoundCue::Critical)
        );
        assert_eq!(
            sound_cue_for_combat_event("Desecrator pool hit for 12; burning"),
            Some(SoundCue::Danger)
        );
        assert_eq!(
            sound_cue_for_combat_event("Picked up rare Moonforged Cleaver and 15 gold"),
            Some(SoundCue::Loot)
        );
        assert_eq!(
            sound_cue_for_combat_event("Picked up legendary Soulreaver Reliquary Fang"),
            Some(SoundCue::Critical)
        );
        assert_eq!(
            sound_cue_for_combat_event("AUTO-EQUIP BUILD POWER | PWR +24"),
            Some(SoundCue::Critical)
        );
        assert_eq!(
            sound_cue_for_combat_event("INVENTORY FULL MAKE ROOM | common Iron Fang"),
            Some(SoundCue::Danger)
        );
        assert_eq!(
            sound_cue_for_combat_event(
                "Boss Phase II: BREAK MALREC before enrage 22s | stagger opens EXECUTE WINDOW"
            ),
            Some(SoundCue::Boss)
        );
        assert_eq!(
            sound_cue_for_combat_event("Ashen Enrage: floor burning | Primal cache downgraded"),
            Some(SoundCue::Boss)
        );
        assert_eq!(
            sound_cue_for_combat_event("Potion restored 45 health"),
            Some(SoundCue::Potion)
        );
        assert_eq!(
            sound_cue_for_combat_event("Frost Nova"),
            Some(SoundCue::Skill)
        );
        assert_eq!(
            sound_cue_for_combat_event("Combo Ready: finish Rupture with Dash or Nova"),
            Some(SoundCue::Combo)
        );
        assert_eq!(
            sound_cue_for_combat_event("Combo Break: Rupture into Nova hit 3 targets +17 fury"),
            Some(SoundCue::Combo)
        );
        assert_eq!(
            sound_cue_for_combat_event("Nephalem Surge ready"),
            Some(SoundCue::Combo)
        );
        assert_eq!(
            sound_cue_for_combat_event("Nephalem Surge unleashed"),
            Some(SoundCue::Critical)
        );
        assert_eq!(
            sound_cue_for_combat_event("Nephalem Surge charging 32/60"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Nephalem Surge extended +1.4s"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event(
                "Ashen pylon: +55% damage, +24% speed, +42 barrier, +38 fury"
            ),
            Some(SoundCue::Critical)
        );
        assert_eq!(
            sound_cue_for_combat_event("Ashen pylon reaping 4/12"),
            Some(SoundCue::Combo)
        );
        assert_eq!(
            sound_cue_for_combat_event("Shrine resonance x2: +16 barrier, +14 fury"),
            Some(SoundCue::Combo)
        );
        assert_eq!(
            sound_cue_for_combat_event("Relic shrine: +35% damage and +18% speed"),
            Some(SoundCue::Skill)
        );
        assert_eq!(
            sound_cue_for_combat_event("Storm shrine: conduit lightning awakened"),
            Some(SoundCue::Skill)
        );
        assert_eq!(
            sound_cue_for_combat_event("Fortune shrine: +50% gold, +25% XP, and better drops"),
            Some(SoundCue::Loot)
        );
        assert_eq!(
            sound_cue_for_combat_event("Sentinel Vanguard command hit 3 for 40"),
            Some(SoundCue::Skill)
        );
        assert_eq!(sound_cue_for_combat_event("Evade"), Some(SoundCue::Skill));
        assert_eq!(
            sound_cue_for_combat_event("Potion recharging 6s"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Dash rune: Reap"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Nova rune: Frost"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Rupture rune: Hemorrhage"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Need 25 fury for dash"),
            Some(SoundCue::Danger)
        );
        assert_eq!(
            sound_cue_for_combat_event("Low life: Ashbone Guard hit for 11"),
            Some(SoundCue::Danger)
        );
        assert_eq!(
            sound_cue_for_combat_event("Ashbone Guard absorbed by ward for 14"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Ashbone Guard evaded"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Soulreaver restored 12 health"),
            Some(SoundCue::Potion)
        );
        assert_eq!(
            sound_cue_for_combat_event("Aegisbrand granted 8 barrier"),
            Some(SoundCue::Potion)
        );
        assert_eq!(
            sound_cue_for_combat_event("Level 2 reached; gained 1 talent point"),
            Some(SoundCue::Critical)
        );
        assert_eq!(
            sound_cue_for_combat_event("Final seal claimed: the Keeper awakens"),
            Some(SoundCue::Boss)
        );
        assert_eq!(
            sound_cue_for_combat_event("BOSS Malrec Awakened | NEXT break stagger, deny enrage"),
            Some(SoundCue::Boss)
        );
        assert_eq!(
            sound_cue_for_combat_event("Reliquary seal restored: Quartermaster reward unlocked"),
            Some(SoundCue::Quest)
        );
        assert_eq!(
            sound_cue_for_combat_event(
                "MAIN Final Seal Claimed | NEXT face Malrec | REWARD essence + boss gate"
            ),
            Some(SoundCue::Quest)
        );
        assert_eq!(
            sound_cue_for_combat_event("Challenge complete: Ashen Vault cleared"),
            Some(SoundCue::Quest)
        );
        assert_eq!(
            sound_cue_for_combat_event(
                "Ember rift opened: defeat six invaders before it collapses"
            ),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event(
                "Ember rift sealed swiftly, Echo Keystone claimed: +80 gold +4 shards +1 essence"
            ),
            Some(SoundCue::Loot)
        );
        assert_eq!(
            sound_cue_for_combat_event("Ember rift collapsed before it was sealed"),
            Some(SoundCue::Danger)
        );
        assert_eq!(
            sound_cue_for_combat_event("Blood obelisk awakened: feed it four kills"),
            Some(SoundCue::Utility)
        );
        assert_eq!(
            sound_cue_for_combat_event("Blood obelisk completed: +70 gold +3 shards +1 essence"),
            Some(SoundCue::Loot)
        );
        assert_eq!(
            sound_cue_for_combat_event("Blood obelisk faded before it was fed"),
            Some(SoundCue::Danger)
        );
        assert_eq!(
            sound_cue_for_combat_event("Quartermaster restocked potions"),
            Some(SoundCue::Loot)
        );
        assert_eq!(
            sound_cue_for_combat_event("Ashbone Guard slain"),
            Some(SoundCue::Death)
        );
        assert_eq!(
            sound_cue_for_combat_event("Strike hit for 20"),
            Some(SoundCue::Hit)
        );
        assert_eq!(sound_cue_for_combat_event("Generated 12 fury"), None);
        assert_eq!(sound_cue_for_combat_event("No combat events yet"), None);
    }

    #[test]
    fn support_sound_cues_use_distinct_audio_files() {
        assert_eq!(sound_cue_file(SoundCue::Skill), "skill.wav");
        assert_eq!(sound_cue_file(SoundCue::Combo), "combo.wav");
        assert_eq!(sound_cue_file(SoundCue::Boss), "boss.wav");
        assert_eq!(sound_cue_file(SoundCue::Quest), "quest.wav");
        assert_eq!(sound_cue_file(SoundCue::Potion), "potion.wav");
        assert_eq!(sound_cue_file(SoundCue::Utility), "utility.wav");
        assert_ne!(
            sound_cue_file(SoundCue::Skill),
            sound_cue_file(SoundCue::Loot)
        );
        assert_ne!(
            sound_cue_file(SoundCue::Potion),
            sound_cue_file(SoundCue::Loot)
        );
        assert_ne!(
            sound_cue_file(SoundCue::Utility),
            sound_cue_file(SoundCue::Loot)
        );
        assert_ne!(
            sound_cue_file(SoundCue::Skill),
            sound_cue_file(SoundCue::Critical)
        );
        assert_ne!(
            sound_cue_file(SoundCue::Combo),
            sound_cue_file(SoundCue::Critical)
        );
        assert_ne!(
            sound_cue_file(SoundCue::Boss),
            sound_cue_file(SoundCue::Danger)
        );
        assert_ne!(
            sound_cue_file(SoundCue::Quest),
            sound_cue_file(SoundCue::Loot)
        );
    }

    #[test]
    fn audio_backend_status_labels_explain_runtime_failures() {
        let enabled = AudioSettings { enabled: true };
        let muted = AudioSettings { enabled: false };

        assert_eq!(AudioBackendStatus::Ready.status_label(&enabled), "audio on");
        assert_eq!(
            AudioBackendStatus::NoOutputDevice.status_label(&enabled),
            "audio no device"
        );
        assert_eq!(
            AudioBackendStatus::ThreadFailed.status_label(&enabled),
            "audio thread failed"
        );
        assert_eq!(
            AudioBackendStatus::Ready.status_label(&muted),
            "audio muted"
        );
    }

    #[test]
    fn audio_backend_retry_policy_recovers_after_failed_start_without_spam() {
        let enabled = AudioSettings { enabled: true };
        let muted = AudioSettings { enabled: false };

        assert!(!audio_backend_should_attempt(
            &muted,
            AudioBackendStatus::NoOutputDevice,
            false,
            true,
            true
        ));
        assert!(!audio_backend_should_attempt(
            &enabled,
            AudioBackendStatus::Ready,
            true,
            false,
            true
        ));
        assert!(audio_backend_should_attempt(
            &enabled,
            AudioBackendStatus::Starting,
            false,
            false,
            false
        ));
        assert!(audio_backend_should_attempt(
            &enabled,
            AudioBackendStatus::Muted,
            false,
            true,
            false
        ));
        assert!(!audio_backend_should_attempt(
            &enabled,
            AudioBackendStatus::NoOutputDevice,
            false,
            false,
            false
        ));
        assert!(audio_backend_should_attempt(
            &enabled,
            AudioBackendStatus::NoOutputDevice,
            false,
            false,
            true
        ));
        assert!(audio_backend_should_attempt(
            &enabled,
            AudioBackendStatus::ThreadFailed,
            false,
            false,
            true
        ));
    }

    #[test]
    fn audio_cue_limiter_prevents_hit_spam_but_allows_distinct_feedback() {
        let mut limiter = AudioCueLimiter::default();

        assert!(limiter.should_play(SoundCue::Hit, 1.0));
        assert!(!limiter.should_play(SoundCue::Hit, 1.02));
        assert!(limiter.should_play(SoundCue::Hit, 1.06));
        assert!(limiter.should_play(SoundCue::Critical, 1.061));
        assert!(limiter.should_play(SoundCue::Combo, 1.062));
        assert!(limiter.should_play(SoundCue::Danger, 2.0));
        assert!(!limiter.should_play(SoundCue::Danger, 2.12));
        assert!(limiter.should_play(SoundCue::Danger, 2.36));
        assert!(limiter.should_play(SoundCue::Boss, 2.37));
        assert!(!limiter.should_play(SoundCue::Boss, 2.60));
        assert!(limiter.should_play(SoundCue::Boss, 2.83));
        assert!(limiter.should_play(SoundCue::Victory, 2.361));
        assert!(limiter.should_play(SoundCue::Victory, 2.362));
    }

    #[test]
    fn sound_cue_cooldowns_prioritize_common_spam_sources() {
        assert!(sound_cue_cooldown_secs(SoundCue::Hit) > 0.0);
        assert!(sound_cue_cooldown_secs(SoundCue::Danger) > sound_cue_cooldown_secs(SoundCue::Hit));
        assert!(sound_cue_cooldown_secs(SoundCue::Boss) > sound_cue_cooldown_secs(SoundCue::Combo));
        assert!(
            sound_cue_cooldown_secs(SoundCue::Combo) > sound_cue_cooldown_secs(SoundCue::Skill)
        );
        assert_eq!(sound_cue_cooldown_secs(SoundCue::Victory), 0.0);
        assert_eq!(sound_cue_cooldown_secs(SoundCue::Defeat), 0.0);
    }

    #[test]
    fn sound_cue_gains_mix_common_hits_under_priority_cues() {
        for cue in ALL_SOUND_CUES {
            let gain = sound_cue_gain(cue);
            assert!(gain > 0.0, "{cue:?} should be audible");
            assert!(gain <= 1.4, "{cue:?} should not clip the generated mix");
        }

        assert!(sound_cue_gain(SoundCue::Hit) < sound_cue_gain(SoundCue::Skill));
        assert!(sound_cue_gain(SoundCue::Skill) < sound_cue_gain(SoundCue::Combo));
        assert!(sound_cue_gain(SoundCue::Combo) < sound_cue_gain(SoundCue::Critical));
        assert!(sound_cue_gain(SoundCue::Danger) > sound_cue_gain(SoundCue::Critical));
        assert!(sound_cue_gain(SoundCue::Boss) > sound_cue_gain(SoundCue::Danger));
        assert!(sound_cue_gain(SoundCue::Victory) > sound_cue_gain(SoundCue::Boss));
    }

    #[test]
    fn enemy_health_bar_children_attach_or_cleanup_without_invalid_parent_commands() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let child = world.spawn_empty().id();

        attach_child_or_cleanup_world(&mut world, parent, child);

        assert_eq!(
            world.get::<ChildOf>(child).map(|child_of| child_of.0),
            Some(parent)
        );
        assert!(
            world
                .get::<Children>(parent)
                .is_some_and(|children| children.contains(&child))
        );

        let dead_parent = world.spawn_empty().id();
        world.entity_mut(dead_parent).despawn();
        let orphan_child = world.spawn_empty().id();

        attach_child_or_cleanup_world(&mut world, dead_parent, orphan_child);

        assert!(world.get_entity(orphan_child).is_err());
    }

    #[test]
    fn all_sound_cues_resolve_to_existing_audio_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/audio");
        for cue in ALL_SOUND_CUES {
            let path = root.join(sound_cue_file(cue));
            assert!(path.exists(), "missing audio file {}", path.display());
        }
    }

    #[test]
    fn enemy_nameplates_scale_for_bosses_and_elites() {
        fn enemy(id: &str, display_name: &str, affixes: Vec<crate::enemy::EnemyAffix>) -> Enemy {
            Enemy {
                id: id.to_string(),
                display_name: display_name.to_string(),
                affixes,
                attack_damage: 1.0,
                attack_kind: crate::data::EnemyAttackKind::Melee,
                attack_range: 1.0,
                attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
                aggro_range: 1.0,
                move_speed: 1.0,
                gold_min: 1,
                gold_max: 1,
                xp_reward: 1,
            }
        }

        let normal = enemy("skeleton", "Ashbone Guard", vec![]);
        let elite = enemy(
            "skeleton",
            "Frenzied Molten Ashbone Guard",
            vec![
                crate::enemy::EnemyAffix::Frenzied,
                crate::enemy::EnemyAffix::Molten,
            ],
        );
        let boss = enemy("keeper", "Malrec, Keeper of Ash", vec![]);

        assert!(enemy_nameplate_font_size(&elite) > enemy_nameplate_font_size(&normal));
        assert!(enemy_nameplate_font_size(&boss) > enemy_nameplate_font_size(&elite));
        assert_ne!(
            enemy_nameplate_color(&elite),
            enemy_nameplate_color(&normal)
        );
        assert!(enemy_nameplate_text(&boss).contains("BOSS Malrec, Keeper of Ash"));
        assert!(enemy_nameplate_text(&elite).contains("ELITE"));
    }

    #[test]
    fn enemy_nameplates_surface_health_and_control_states() {
        let enemy = Enemy {
            id: "skeleton".to_string(),
            display_name: "Frenzied Ashbone Guard".to_string(),
            affixes: vec![crate::enemy::EnemyAffix::Frenzied],
            attack_damage: 1.0,
            attack_kind: crate::data::EnemyAttackKind::Melee,
            attack_range: 1.0,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 1.0,
            move_speed: 1.0,
            gold_min: 1,
            gold_max: 1,
            xp_reward: 1,
        };
        let health = Health {
            current: 25.0,
            max: 100.0,
        };
        let chilled = Chilled {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            slow_multiplier: 0.5,
        };
        let staggered = Staggered {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            damage_multiplier: 1.5,
        };

        let text = enemy_nameplate_text_with_status(
            &enemy,
            Some(&health),
            Some(&chilled),
            Some(&staggered),
        );

        assert!(text.contains("RARE"));
        assert!(text.contains("stagger chill"));
        assert!(text.contains("25%"));
        assert_ne!(
            enemy_health_fill_color(0.8, &enemy, Some(&chilled), None, None),
            enemy_health_fill_color(0.8, &enemy, None, None, None)
        );
    }

    #[test]
    fn target_info_surfaces_enemy_tier_affixes_health_and_status() {
        let enemy = Enemy {
            id: "cultist".to_string(),
            display_name: "Frenzied Arcane Cultist".to_string(),
            affixes: vec![EnemyAffix::Frenzied, EnemyAffix::Arcane],
            attack_damage: 8.0,
            attack_kind: crate::data::EnemyAttackKind::Projectile,
            attack_range: 6.0,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 8.0,
            move_speed: 1.0,
            gold_min: 1,
            gold_max: 2,
            xp_reward: 3,
        };
        let health = Health {
            current: 44.0,
            max: 80.0,
        };
        let chilled = Chilled {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            slow_multiplier: 0.5,
        };

        let info = target_info_from_enemy(&enemy, &health, Some(&chilled), None, None, None);

        assert!(info.visible);
        assert_eq!(info.name, "Frenzied Arcane Cultist");
        assert_eq!(info.subtitle, "ELITE Caster - Projectile");
        assert!(info.health_line.contains("44/80 HP"));
        assert!(info.health_line.contains("55%"));
        assert!(info.details.contains("Frenzied / Arcane"));
        assert!(info.details.contains("threat: fast engage, beam hazard"));
        assert!(
            info.details
                .contains("react: kite first swing, strafe beam")
        );
        assert!(info.details.contains("chill"));
        assert_eq!(info.health_percent, 55.0);
    }

    #[test]
    fn target_info_names_chapter_enemy_roles_and_counterplay() {
        let health = Health {
            current: 70.0,
            max: 100.0,
        };

        let mut stalker = test_enemy("bone_stalker", vec![]);
        stalker.attack_kind = crate::data::EnemyAttackKind::Melee;
        let stalker_info = target_info_from_enemy(&stalker, &health, None, None, None, None);
        assert_eq!(stalker_info.subtitle, "Chaser - Melee");
        assert!(
            stalker_info
                .details
                .contains("role: chaser | tip: kite rush, punish whiffs")
        );

        let mut marksman = test_enemy("ashen_marksman", vec![]);
        marksman.attack_kind = crate::data::EnemyAttackKind::Projectile;
        let marksman_info = target_info_from_enemy(&marksman, &health, None, None, None, None);
        assert_eq!(marksman_info.subtitle, "Marksman - Projectile");
        assert!(
            marksman_info
                .details
                .contains("role: marksman | tip: close gap, sidestep bolts")
        );

        let mut brute = test_enemy("reliquary_brute", vec![EnemyAffix::Molten]);
        brute.attack_kind = crate::data::EnemyAttackKind::Shockwave;
        let brute_info = target_info_from_enemy(&brute, &health, None, None, None, None);
        assert_eq!(brute_info.subtitle, "RARE Heavy - Shockwave");
        assert!(
            brute_info
                .details
                .contains("role: heavy | tip: bait shockwave, punish slam")
        );

        let guard_info = target_info_from_enemy(
            &test_enemy("skeleton", vec![]),
            &health,
            None,
            None,
            None,
            None,
        );
        assert_eq!(guard_info.subtitle, "Guard - Melee");
        assert!(
            guard_info
                .details
                .contains("role: guard | tip: flank shield guard")
        );

        let mut cultist = test_enemy("cultist", vec![]);
        cultist.attack_kind = crate::data::EnemyAttackKind::Projectile;
        let cultist_info = target_info_from_enemy(&cultist, &health, None, None, None, None);
        assert_eq!(cultist_info.subtitle, "Caster - Projectile");
        assert!(
            cultist_info
                .details
                .contains("role: caster | tip: dash through fire")
        );

        let mut warden = test_enemy("seal_warden", vec![]);
        warden.attack_kind = crate::data::EnemyAttackKind::Projectile;
        let ward = SealWardenWard {
            current: 18.0,
            max: 45.0,
            broken: false,
        };
        let warden_info = target_info_from_enemy(&warden, &health, None, None, None, Some(&ward));
        assert_eq!(warden_info.subtitle, "WARDEN - Projectile");
        assert!(
            warden_info
                .details
                .contains("role: warden | tip: break ward, sidestep runes")
        );
        assert!(warden_info.details.contains("ward 18/45"));
        let broken_ward = SealWardenWard {
            current: 0.0,
            max: 45.0,
            broken: true,
        };
        let broken_warden_info =
            target_info_from_enemy(&warden, &health, None, None, None, Some(&broken_ward));
        assert!(broken_warden_info.details.contains("ward broken: punish"));

        let keeper_info = target_info_from_enemy(
            &test_enemy("keeper", vec![]),
            &health,
            None,
            None,
            None,
            None,
        );
        assert_eq!(keeper_info.subtitle, "BOSS - Melee");
        assert!(
            keeper_info
                .details
                .contains("role: boss | tip: break stagger, deny enrage")
        );
    }

    #[test]
    fn target_info_surfaces_immediate_threat_actions() {
        let mut enemy = Enemy {
            id: "cultist".to_string(),
            display_name: "Arcane Cultist".to_string(),
            affixes: vec![EnemyAffix::Arcane, EnemyAffix::Jailer],
            attack_damage: 8.0,
            attack_kind: crate::data::EnemyAttackKind::Projectile,
            attack_range: 6.0,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 8.0,
            move_speed: 1.0,
            gold_min: 1,
            gold_max: 2,
            xp_reward: 3,
        };
        enemy.attack_timer.tick(Duration::from_secs_f32(0.72));
        let health = Health {
            current: 44.0,
            max: 80.0,
        };

        let info = target_info_from_enemy(&enemy, &health, None, None, None, None);

        assert!(info.details.contains("incoming: strafe shot 0.3s"));

        enemy.id = "seal_warden".to_string();
        enemy.display_name = "Seal Warden Vhal".to_string();
        let warden_info = target_info_from_enemy(&enemy, &health, None, None, None, None);
        assert!(warden_info.details.contains("incoming: leave seal rune"));

        let staggered = Staggered {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            damage_multiplier: 1.4,
        };
        let stagger_info =
            target_info_from_enemy(&enemy, &health, None, Some(&staggered), None, None);
        assert!(stagger_info.details.contains("window: burst now"));

        let boss_phase = BossPhase::new_phase_two(5.8);
        let boss_info =
            target_info_from_enemy(&enemy, &health, None, None, Some(&boss_phase), None);
        assert!(boss_info.details.contains("danger: break before enrage"));

        let enraged = BossPhase::new_enraged();
        let enraged_info =
            target_info_from_enemy(&enemy, &health, None, None, Some(&enraged), None);
        assert!(
            enraged_info
                .details
                .contains("danger: kite fire, burst after slam")
        );
    }

    #[test]
    fn enemy_affix_threat_summary_names_every_combat_hazard() {
        let enemy = test_enemy(
            "elite",
            vec![
                EnemyAffix::Vampiric,
                EnemyAffix::Molten,
                EnemyAffix::Shielded,
                EnemyAffix::Jailer,
                EnemyAffix::Frozen,
                EnemyAffix::Desecrator,
                EnemyAffix::Reflective,
            ],
        );

        let summary = enemy_affix_threat_summary(&enemy);

        for expected in [
            "lifesteal",
            "death pool",
            "shield window",
            "root trap",
            "freeze burst",
            "ground fire",
            "reflect",
        ] {
            assert!(summary.contains(expected));
        }
    }

    #[test]
    fn enemy_affix_reaction_summary_guides_player_response_without_repetition() {
        let enemy = test_enemy(
            "elite",
            vec![
                EnemyAffix::Jailer,
                EnemyAffix::Frozen,
                EnemyAffix::Arcane,
                EnemyAffix::Desecrator,
            ],
        );

        let summary = enemy_affix_reaction_summary(&enemy);

        assert_eq!(summary, "react: save Shift, leave circle, strafe beam");
        assert!(!summary.contains("move out"));
    }

    #[test]
    fn enemy_bars_hide_full_health_trash_but_show_priority_targets() {
        fn enemy(id: &str, display_name: &str, affixes: Vec<crate::enemy::EnemyAffix>) -> Enemy {
            Enemy {
                id: id.to_string(),
                display_name: display_name.to_string(),
                affixes,
                attack_damage: 1.0,
                attack_kind: crate::data::EnemyAttackKind::Melee,
                attack_range: 1.0,
                attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
                aggro_range: 1.0,
                move_speed: 1.0,
                gold_min: 1,
                gold_max: 1,
                xp_reward: 1,
            }
        }

        let normal = enemy("skeleton", "Ashbone Guard", vec![]);
        let elite = enemy(
            "skeleton",
            "Frenzied Molten Ashbone Guard",
            vec![
                crate::enemy::EnemyAffix::Frenzied,
                crate::enemy::EnemyAffix::Molten,
            ],
        );
        let boss = enemy("keeper", "Malrec, Keeper of Ash", vec![]);

        assert_eq!(
            enemy_bar_visibility(1.0, &normal, None, None, None, false),
            EnemyBarVisibility::Hidden
        );
        assert_eq!(
            enemy_bar_visibility(1.0, &normal, None, None, None, true),
            EnemyBarVisibility::Visible
        );
        assert_eq!(
            enemy_bar_visibility(0.72, &normal, None, None, None, false),
            EnemyBarVisibility::Visible
        );
        assert_eq!(
            enemy_bar_visibility(1.0, &elite, None, None, None, false),
            EnemyBarVisibility::Visible
        );
        assert_eq!(
            enemy_bar_visibility(1.0, &boss, None, None, None, false),
            EnemyBarVisibility::Visible
        );
    }

    #[test]
    fn enemy_bar_thickness_emphasizes_low_health_elites_and_stagger() {
        let enemy = Enemy {
            id: "skeleton".to_string(),
            display_name: "Ashbone Guard".to_string(),
            affixes: vec![
                crate::enemy::EnemyAffix::Frenzied,
                crate::enemy::EnemyAffix::Molten,
            ],
            attack_damage: 1.0,
            attack_kind: crate::data::EnemyAttackKind::Melee,
            attack_range: 1.0,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 1.0,
            move_speed: 1.0,
            gold_min: 1,
            gold_max: 1,
            xp_reward: 1,
        };
        let staggered = Staggered {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            damage_multiplier: 1.5,
        };

        let healthy = enemy_bar_fill_thickness_scale(0.9, &enemy, None);
        let low = enemy_bar_fill_thickness_scale(0.2, &enemy, None);
        let broken = enemy_bar_fill_thickness_scale(0.2, &enemy, Some(&staggered));

        assert!(low > healthy);
        assert!(broken > low);
    }

    #[test]
    fn enemy_focus_ring_pose_scales_priority_and_critical_targets() {
        let normal = test_enemy("skeleton", vec![]);
        let elite = test_enemy(
            "skeleton",
            vec![
                crate::enemy::EnemyAffix::Frenzied,
                crate::enemy::EnemyAffix::Molten,
            ],
        );
        let boss = test_enemy("keeper", vec![]);

        let normal_pose = enemy_focus_ring_pose(&normal, 0.8, false);
        let elite_pose = enemy_focus_ring_pose(&elite, 0.8, false);
        let boss_pose = enemy_focus_ring_pose(&boss, 0.8, false);
        let crit_pose = enemy_focus_ring_pose(&normal, 0.8, true);
        let hidden = enemy_focus_ring_pose(&normal, 0.0, false);

        assert_eq!(normal_pose.visibility, Visibility::Visible);
        assert!(elite_pose.scale.x > normal_pose.scale.x);
        assert!(boss_pose.scale.x > elite_pose.scale.x);
        assert!(crit_pose.scale.x > normal_pose.scale.x);
        assert_eq!(hidden.visibility, Visibility::Hidden);
    }

    #[test]
    fn hover_focus_selects_nearest_living_enemy_under_cursor() {
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let dead = Entity::from_raw_u32(3).unwrap();
        let enemy = test_enemy("skeleton", vec![]);
        let near_transform = Transform::from_xyz(0.32, 0.0, 0.0);
        let far_transform = Transform::from_xyz(0.58, 0.0, 0.0);
        let dead_transform = Transform::from_xyz(0.12, 0.0, 0.0);
        let living_health = Health {
            current: 10.0,
            max: 10.0,
        };
        let dead_health = Health {
            current: 0.0,
            max: 10.0,
        };

        let hovered = hovered_enemy_from_cursor(
            Some(Vec3::ZERO),
            [
                (far, &enemy, &far_transform, &living_health),
                (dead, &enemy, &dead_transform, &dead_health),
                (near, &enemy, &near_transform, &living_health),
            ]
            .into_iter(),
        );

        assert_eq!(hovered, Some(near));
        assert_eq!(
            hovered_enemy_from_cursor(
                None,
                [(near, &enemy, &near_transform, &living_health)].into_iter()
            ),
            None
        );
    }

    #[test]
    fn hover_focus_ring_is_visible_but_weaker_than_hit_focus() {
        let enemy = test_enemy("skeleton", vec![]);
        let hover = enemy_hover_focus_ring_pose(&enemy, 0.2);
        let hit = enemy_focus_ring_pose(&enemy, 0.8, false);

        assert_eq!(hover.visibility, Visibility::Visible);
        assert!(hover.scale.x < hit.scale.x);
        assert_eq!(hover.translation, hit.translation);
    }

    #[test]
    fn intent_focus_ring_is_stronger_than_hover_but_weaker_than_hit_focus() {
        let enemy = test_enemy("skeleton", vec![]);
        let hover = enemy_hover_focus_ring_pose(&enemy, 0.2);
        let intent = enemy_intent_focus_ring_pose(&enemy, 0.2);
        let hit = enemy_focus_ring_pose(&enemy, 0.8, false);

        assert_eq!(intent.visibility, Visibility::Visible);
        assert!(hover.scale.x < intent.scale.x);
        assert!(intent.scale.x < hit.scale.x);
        assert_eq!(intent.translation, hit.translation);
    }
}
