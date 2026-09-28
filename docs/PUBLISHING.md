# Publishing & Maintaining `fralsare-daily` on GitHub

Reference guide for the one-time initial upload and the day-to-day workflow.

Project path on this machine:

```
/run/media/fralsare/500gbNvmeExt/FralsareFiles/ApplicationCreation/NewsApp
```

---

## 1. One-time: initial upload to GitHub

### 1.1 Create an empty repository

1. Go to <https://github.com/new> (or `+` in the top-right → "New repository").
2. **Repository name**: `fralsare-daily`
3. **Description**:
   `A minimal cross-platform desktop news reader organized by topics — Tauri 2 + Vanilla TypeScript`
4. Visibility: **Public**
5. ⚠️ Leave **all** checkboxes **unchecked** — do **not** initialize with a
   README, .gitignore, or license. They already exist in the local repo, and
   initializing them on GitHub forces a merge on first push.
6. Click **Create repository**.

### 1.2 Push the local repo

```bash
cd /run/media/fralsare/500gbNvmeExt/FralsareFiles/ApplicationCreation/NewsApp

# Add the remote (pick ONE, replacing YOUR-USERNAME):
git remote add origin git@github.com:YOUR-USERNAME/fralsare-daily.git      # SSH
# or
git remote add origin https://github.com/YOUR-USERNAME/fralsare-daily.git  # HTTPS

# Push and track the branch:
git push -u origin main
```

### 1.3 Authentication notes

- **SSH URL**: works if your SSH key is registered on GitHub
  (Settings → *SSH and GPG keys*). Test with:

  ```bash
  ssh -T git@github.com   # should greet you by username
  ```

- **HTTPS URL**: GitHub does not accept account passwords. Create a
  **Personal Access Token**:
  Settings → *Developer settings* → *Personal access tokens* →
  *Fine-grained tokens* → select only the `fralsare-daily` repo →
  permission **Contents: Read and write**. Paste the token as the
  "password" when prompted. (The token can also be stored in a git
  credential helper to avoid re-entering it.)

### 1.4 Verify

1. Open `github.com/YOUR-USERNAME/fralsare-daily` — you should see `src/`,
   `src-tauri/`, `README.md`, `LICENSE`, and `.github/workflows/`.
2. **Actions** tab: the `CI` workflow starts automatically on the first
   push — it builds and tests on Linux, macOS, and Windows. The first run
   is slow (cold Rust build on each runner).
3. **Insights → Traffic**: watch visits over the following days.

---

## 2. Day-to-day workflow

```bash
cd /run/media/fralsare/500gbNvmeExt/FralsareFiles/ApplicationCreation/NewsApp

# local checks before committing:
source ~/.cargo/env
cargo test --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
npm run build

# commit + push:
git add -A
git commit -m "Short summary of the change"
git push
```

Useful checks:

```bash
git status          # what changed / uncommitted
git log --oneline   # history
git diff            # uncommitted changes in detail
```

`CI` re-runs automatically on every push to `main`.

---

## 3. Creating a release (buildable binaries)

The `release.yml` workflow builds installers for Windows and Linux
(`.deb`, `.rpm`, `.AppImage`, `.msi`, `.exe`, …) whenever a tag is pushed:

```bash
git tag v0.1.0        # or v0.2.0, v1.0.0, ...
git push origin v0.1.0
```

Then open **Releases** on the repo — the artifacts attach to the release
once the workflow finishes (allow several minutes).

Keep the version in sync before tagging:

- `src-tauri/tauri.conf.json` → `"version"`
- `src-tauri/Cargo.toml` → `version`
- `package.json` → `version`

---

## 4. Optional polish (one-time, after the first push)

- **Screenshot**: take one with `gnome-screenshot` (or `scrot`), save it as
  `docs/screenshot.png`, then:

  ```bash
  git add docs/screenshot.png
  git commit -m "Add app screenshot to README"
  git push
  ```

  (The README already references `docs/screenshot.png`.)

- **Topics** (recommended set for this repo): on the repo page, under the
  About panel, paste these (comma-separated, up to 25 allowed):

  ```
  news, news-reader, rss, rss-reader, news-aggregator, tauri, rust,
  typescript, desktop-app, cross-platform, linux, windows, privacy, minimal
  ```

  Why these:
  - `news`, `news-reader`, `rss`, `rss-reader`, `news-aggregator` — the
    core what-it-is; these are the terms people actually search.
  - `tauri`, `rust`, `typescript` — stack topics; `tauri` in particular
    has an active community that discovers apps this way.
  - `desktop-app`, `cross-platform`, `linux`, `windows` — platform
    discoverability (GitHub filters by these).
  - `privacy`, `minimal` — the product's selling points (no accounts,
    no API keys, no tracking).

  Avoid: `open-source` (redundant for a public repo), `web`, `webapp`
  (misleading — this is a native window), `electron` (it's not Electron;
  at most `electron-alternative` if you want to attract that audience).

- **Promo images** (already in the repo, regenerate with
  `python3 scripts/gen-banner.py`):
  - `docs/banner.png` (1600×500, dark) — wide web banner; good for a
    GitHub topic page, personal site, or link-posts on forums/HN.
  - `docs/og.png` (1200×630, light) — the standard Open Graph size; use
    it when sharing the repo on social media, and it is the size GitHub
    uses for link previews.

- **Pinning**: the repo can be pinned on your GitHub profile
  (profile → Customization → Pin repos).

---

## 5. Troubleshooting

| Symptom | Fix |
| --- | --- |
| `error: remote origin already exists` | `git remote set-url origin <url>` instead of `add` |
| `Permission denied (publickey)` | SSH key not registered — add it or switch to HTTPS + token |
| `Authentication failed` on HTTPS | Use a Personal Access Token, not the account password |
| Push rejected: remote has commits | You initialized the GitHub repo with a README — see §1.1, recreate it empty, or `git pull --rebase origin main` and resolve |
| CI failing on Windows/macOS only | Check the workflow logs in **Actions**; usually missing platform deps — the workflows already install the standard Tauri prerequisites |
