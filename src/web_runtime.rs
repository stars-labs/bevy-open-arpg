//! Keep combat asset reads dormant while the player is choosing an expedition.
//! Handles remain real AssetServer handles; the gate delays IO, not identity.
#[cfg(any(target_arch = "wasm32", test))]
use std::{
    future::poll_fn,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Poll, Waker},
};

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Default)]
struct ReadGate {
    open: AtomicBool,
    waiters: Mutex<Vec<Waker>>,
}
#[cfg(any(target_arch = "wasm32", test))]
impl ReadGate {
    async fn wait(&self) {
        poll_fn(|context| {
            let mut waiters = self.waiters.lock().unwrap();
            if self.open.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                if !waiters.iter().any(|w| w.will_wake(context.waker())) {
                    waiters.push(context.waker().clone());
                }
                Poll::Pending
            }
        })
        .await
    }
    fn release(&self) {
        self.open.store(true, Ordering::Release);
        for waker in self.waiters.lock().unwrap().drain(..) {
            waker.wake();
        }
    }
}
#[cfg(any(target_arch = "wasm32", test))]
fn menu_asset(path: &std::path::Path) -> bool {
    ["fonts", "images/generated/ui", "data"]
        .iter()
        .any(|root| path.starts_with(root))
}

#[cfg(any(target_arch = "wasm32", test))]
use bevy::asset::io::{AssetReader, AssetReaderError, PathStream, Reader};
#[cfg(any(target_arch = "wasm32", test))]
use std::path::Path;
#[cfg(any(target_arch = "wasm32", test))]
struct DemandReader<R> {
    inner: R,
    gate: std::sync::Arc<ReadGate>,
}
#[cfg(any(target_arch = "wasm32", test))]
impl<R: AssetReader> AssetReader for DemandReader<R> {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if !menu_asset(path) {
            self.gate.wait().await;
        }
        self.inner.read(path).await
    }
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if !menu_asset(path) {
            self.gate.wait().await;
        }
        self.inner.read_meta(path).await
    }
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.inner.read_directory(path).await
    }
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.inner.is_directory(path).await
    }
}

use crate::{
    GameState,
    assets::{GameAssets, game_assets_load_summary},
};
use bevy::{
    asset::io::{AssetSourceBuilder, AssetSourceId, wasm::HttpWasmAssetReader},
    prelude::*,
};
use js_sys::wasm_bindgen::JsCast;
use std::sync::{Arc, atomic::AtomicBool as Flag};
static COMBAT_OPEN: Flag = Flag::new(false);
#[derive(Resource)]
struct WebGate {
    reader: Arc<ReadGate>,
    requested: bool,
    approved: bool,
    last_state: GameState,
}
pub(crate) fn combat_open() -> bool {
    COMBAT_OPEN.load(Ordering::Acquire)
}
pub(crate) fn install(app: &mut App) {
    let reader = Arc::new(ReadGate::default());
    let source = reader.clone();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(DemandReader {
                inner: HttpWasmAssetReader::new("assets"),
                gate: source.clone(),
            })
        }),
    )
    .insert_resource(WebGate {
        reader,
        requested: false,
        approved: false,
        last_state: GameState::Loading,
    })
    .add_systems(PreUpdate, (sync_viewport, prepare_transition).chain())
    .add_systems(Update, report_state);
}
fn value(key: &str) -> js_sys::wasm_bindgen::JsValue {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(w.as_ref(), &key.into()).ok())
        .unwrap_or_default()
}
fn call(key: &str, argument: f64) {
    if let Some(window) = web_sys::window() {
        if let Some(f) = value(key).dyn_ref::<js_sys::Function>() {
            let _ = f.call1(window.as_ref(), &argument.into());
        }
    }
}
pub(crate) fn menu_ready(server: &AssetServer, a: &GameAssets) -> bool {
    let mut ids = vec![
        a.ui_font_regular.id().untyped(),
        a.ui_font_bold.id().untyped(),
        a.ui_font_chinese.id().untyped(),
        a.ui_main_menu.id().untyped(),
        a.ui_panel_frame.id().untyped(),
        a.ui_slot_frame.id().untyped(),
        a.ui_utility_icons.id().untyped(),
    ];
    ids.extend(a.ui_class_portraits.iter().map(|h| h.id().untyped()));
    ids.into_iter()
        .all(|id| server.is_loaded_with_dependencies(id))
}
fn prepare_transition(
    mut gate: ResMut<WebGate>,
    mut next: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
    assets: Option<Res<GameAssets>>,
    server: Res<AssetServer>,
) {
    if *state.get() == GameState::MainMenu && gate.last_state != GameState::MainMenu {
        gate.approved = false;
        call("arpgReleaseChapter", 0.);
    }
    gate.last_state = *state.get();
    if matches!(*next, NextState::Pending(GameState::InGame)) && !gate.approved {
        *next = NextState::Unchanged;
        if !gate.requested {
            call("arpgPrepareChapter", 0.);
            gate.requested = true;
        }
    }
    if !gate.requested {
        return;
    }
    let status = js_sys::Reflect::get(&value("arpgChapterDownload"), &"status".into())
        .ok()
        .and_then(|s| s.as_string());
    if status.as_deref() == Some("cancelled") {
        gate.requested = false;
        return;
    }
    if status.as_deref() == Some("ready") {
        gate.reader.release();
        COMBAT_OPEN.store(true, Ordering::Release);
        if assets
            .as_ref()
            .is_some_and(|a| game_assets_load_summary(&server, a).ready())
        {
            gate.requested = false;
            gate.approved = true;
            if *state.get() == GameState::MainMenu {
                next.set(GameState::InGame);
            }
        }
    }
}
pub(crate) fn open_downloads() {
    call("arpgOpenDownloads", 0.);
}
pub(crate) fn downloads_open() -> bool {
    value("arpgDownloadsOpen").as_bool().unwrap_or(false)
}
pub(crate) fn save_language(language: crate::ui::localization::UiLanguage) {
    if let Some(window) = web_sys::window() {
        let text = if language == crate::ui::localization::UiLanguage::Chinese {
            "zh-CN"
        } else {
            "en"
        };
        if let Some(f) = value("arpgSetLanguage").dyn_ref::<js_sys::Function>() {
            let _ = f.call1(window.as_ref(), &text.into());
        }
    }
}
fn report_state(
    state: Res<State<GameState>>,
    mut language: ResMut<crate::ui::localization::UiLanguage>,
    mut initialized: Local<bool>,
    assets: Option<Res<GameAssets>>,
    server: Res<AssetServer>,
) {
    if let Some(window) = web_sys::window() {
        if assets
            .as_ref()
            .is_some_and(|a| game_assets_load_summary(&server, a).failed > 0)
            && value("arpgLoadError").as_bool() != Some(true)
        {
            call("arpgAssetFailure", 0.);
        }
        let label = match state.get() {
            GameState::MainMenu => "menu",
            GameState::InGame => "game",
            _ => "loading",
        };
        let _ = js_sys::Reflect::set(window.as_ref(), &"arpgViewState".into(), &label.into());
        if !*initialized || *state.get() == GameState::Loading {
            let saved = window
                .local_storage()
                .ok()
                .flatten()
                .and_then(|s| s.get_item("open-arpg-language").ok().flatten());
            *language = if saved.as_deref() == Some("zh-CN") {
                crate::ui::localization::UiLanguage::Chinese
            } else {
                crate::ui::localization::UiLanguage::English
            };
            *initialized = true;
        }
    }
}

// Browser zoom and mobile emulation can report a device-pixel box that
// disagrees with CSS hit coordinates. Use the visible canvas as the authority.
fn sync_viewport(mut windows: Query<&mut Window>) {
    let Some(browser) = web_sys::window() else {
        return;
    };
    let Some(canvas) = browser
        .document()
        .and_then(|doc| doc.query_selector("canvas").ok().flatten())
    else {
        return;
    };
    let rect = canvas.get_bounding_client_rect();
    let scale = ((browser.device_pixel_ratio() * 1000.0).round() / 1000.0) as f32;
    if rect.width() <= 0.0 || rect.height() <= 0.0 || !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let width = (rect.width() * f64::from(scale)).round() as u32;
    let height = (rect.height() * f64::from(scale)).round() as u32;
    if window.resolution.scale_factor_override() != Some(scale) {
        window.resolution.set_scale_factor_override(Some(scale));
    }
    if window.physical_width() != width || window.physical_height() != height {
        window.resolution.set_physical_resolution(width, height);
    }
}
