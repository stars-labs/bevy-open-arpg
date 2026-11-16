# Mobile and on-demand delivery

Status: local final acceptance passed for portrait class selection, mobile
entry/settings/downloads and cold-process offline entry. Desktop final capture
and public CI/deployment verification are in progress.

## Player behavior

- Start saves the engine, fonts and shared interface (about39.4MiB compressed
  transfer). Starting a descent separately prepares the shared expedition pack
  (28.7MiB). Credits/support files are optional. Already verified bytes are reused.
- Every file has a SHA-256 identity and immutable URL. An explicit version choice
  prepares changed installed content; a live run is never automatically refreshed.
  Failed preparation keeps the previous version. Settings manages offline content
  and protects resources leased by an active run.
- The build fingerprints compiled inputs and literal embedded files. Matching
  published engines are reused byte-for-byte for independent content updates.
  Rust changes can still require the whole compressed WASM; there is no binary
  delta claim. Existing embedded RON tuning remains a compiled input.
- English/Chinese switching works before the engine starts. PWA guidance follows
  the actual browser installation affordance. Readiness comes from real asset
  decoding and game state, not an elapsed-time escape hatch.
- A floating movement pad captures one finger and suppresses emulated attacks.
  Inventory/talent/class lists accept finger scrolling. Short landscape menus
  fit, and canvas CSS/backing dimensions agree so visible-center taps work.
- Portrait class cards and Confirm/Back fit the viewport. Card presses choose a
  preview; only explicit Confirm starts the descent, so a swipe cannot launch it.
- The existing settings entry is accessible during gameplay and owns the pause
  it introduces. Closing preserves an earlier manual pause.

## Evidence

`docs/verification/mobile-demand-2026-11-16/` contains actual screenshots,
network receipts and logs. The local build was made before the commit; its
compiled-input hash in local-build.json identifies the tested source tree.

- Final fmt, all-feature check, strict all-target/all-feature Clippy passed.
- Final tests:899unit +27integration =926passed,0failed.
- Current-source ChapterII headless gameplay smoke passed with one player,
 49procedural chunks,431scenery objects and15interactables.
-9real build-wrapper/fingerprint/reuse tests pass, including changed content
  reusing exact JS/WASM bytes and changed compiled sources forcing a build.
-21real service-worker scenarios and6production-launcher fixture scenarios
  passed earlier in this rollout. Fixtures are not used as runtime proof.
- Final real WebGPU portrait: visible controls, finger scroll to the next class,
  no accidental descent. Final mobile: menu→class→explicit preparation→game,
  pad movement/cancel, settings and actual download manager, errors=[].
- A separate full Chromium process started offline before navigation and entered
  the prepared real game, errors=[]. The menu never requests combat models.
- Browser checks use Chromium156 touch emulation and Vulkan/SwiftShader on private
  Xvfb displays. Physical Android/iOS installation is not verified.

## Publication gate

The source push uses the existing release-to-Pages workflow. All exact-head CI,
CodeQL (when emitted) and deployment checks must pass. `tools/verify_published.py`
checks the public build and manifest revision, worker manifest, and size/SHA-256
of every immutable file. Final hosted mobile/desktop acceptance uses fresh
profiles and asserts the installed worker revision, preventing an old cached
version from being mistaken for the new release.
