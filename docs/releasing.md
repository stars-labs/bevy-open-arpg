# 发布流程（GitHub）

本项目的发布按 Bevy 风格分为两类：

- `web-latest`：每次 `main` 推送后自动生成滚动预览发布并更新 GitHub Pages。
- `vX.Y.Z`：对语义化版本 tag 的正式发布，附带三平台原生二进制附件。

## 零、仓库一次性配置

GitHub Pages 的发布源必须设为 **GitHub Actions**，不能保留旧的 `gh-pages`
分支模式。受保护的 `github-pages` environment 应允许 `main` 分支和 `v*`
tag 部署。当前仓库已经完成该配置，可用以下命令复核：

```bash
gh api repos/stars-labs/bevy-open-arpg/pages \
  --jq '{html_url, status, build_type, https_enforced}'
gh api repos/stars-labs/bevy-open-arpg/environments/github-pages/deployment-branch-policies \
  --jq '.branch_policies | map({name, type})'
```

预期 `build_type` 为 `workflow`，部署规则为 `main`/`branch` 和 `v*`/`tag`。
workflow 在部署前也会执行同一发布源预检，避免直到 `deploy-pages` 才失败。

## 一、发起 web-latest 预览

### GitHub Actions 页面触发
- 进入 **Actions → Build, Release, and Deploy WebGPU**。
- 选择 `workflow_dispatch`。
- `release_type` 选 `preview`（默认）。
- 也可填写 `checkout_ref`（例如 `main` 或任意 commit/tag）。

### CLI 触发
```bash
./scripts/release_github.sh preview
./scripts/release_github.sh preview main
```

## 二、发起版本发布（vX.Y.Z）

### 触发条件
- `tag` 必须是 `v<semver>`（如 `v0.1.0`）。
- tag 会和 `Cargo.toml` 里的 `package.version` 严格比对。

### CLI 一键
```bash
./scripts/release_github.sh tag --push v0.1.0
```

该命令会先检查工作区是否干净、创建/校验 tag（缺省偏向签名标签，失败时回退到普通注解标签），然后触发 GitHub Workflow 的 `release_type=tag`。

### 手动方式
```bash
./scripts/release_github.sh tag v0.1.0   # tag 已存在时触发发布
# 或
# gh workflow run deploy-wasm-pages.yml -f release_type=tag -f release_tag=v0.1.0
```

## 三、发布产物与验收

发布会上传以下附件（`web-latest` 也会上传 web 相关产物）：

- `web-dist.zip`
- `web-dist.zip.sha256`
- `web-release-manifest.csv`
- `release-asset-manifest.csv`
- `release-asset-manifest.csv.sha256`

正式版本还会附加：

- `bevy-open-arpg-vX.Y.Z-linux-x86_64.tar.gz`
- `bevy-open-arpg-vX.Y.Z-windows-x86_64.zip`
- `bevy-open-arpg-vX.Y.Z-macos-x86_64.tar.gz`
- 对应的 `.sha256` 文件

每个原生压缩包都包含可执行文件、完整 `assets/` 运行时资源目录、`README.md`、`Cargo.toml` 和 `LICENSE`。发布附件同时生成 GitHub build-provenance attestation。

### 发布完成后快速验收

```bash
gh release view v0.1.0
gh run list --workflow deploy-wasm-pages.yml --limit 3 --repo stars-labs/bevy-open-arpg

gh release download v0.1.0 --pattern 'web-dist.zip*' --repo stars-labs/bevy-open-arpg
sha256sum -c web-dist.zip.sha256

gh release download v0.1.0 --pattern 'release-asset-manifest.csv*' --repo stars-labs/bevy-open-arpg
sha256sum -c release-asset-manifest.csv.sha256

gh attestation verify web-dist.zip --repo stars-labs/bevy-open-arpg
```

## 四、和 Bevy 习惯一致的约束

- 版本发布必须来自 tag；预览发布允许在任意 ref。
- 预发布 `web-latest` 为滚动 prerelease；正式 `v*` 为非预发布。
- 发布任务会在缺失附件时失败，避免“只发半套包”。
- `web-latest` tag 会显式移动到实际构建的 commit，Pages 再从该 Release 下载并部署同一份 `web-dist.zip`。
- Actions 使用固定 commit SHA，写权限只授予 Release/Pages 所需的 job。
- Pages 使用 GitHub Actions 发布源，并通过仅允许 `main` 与 `v*` 的 `github-pages` environment 保护部署。
- `deploy-pages` 完成后会绕过 CDN 缓存重试比对线上 HTML、JS、gzip WASM 和 Chapter II 模型的 SHA-256，确保 Pages 与 Release ZIP 是同一份内容。
