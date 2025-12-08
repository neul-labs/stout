# stout Roadmap

This document outlines planned extensions for stout, following the **preprocessed index architecture**.

## Architecture Principle

stout achieves its speed by preprocessing data at build time, not runtime:

```
┌─────────────────────────────────────────────────────────────────┐
│                  INDEX BUILD TIME (CI/CD)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   Source API ──▶ sync_*.py ──▶ SQLite Index + Compressed JSON   │
│                      │                                           │
│            - Transform to stout format                           │
│            - Build FTS5 search index                             │
│            - Compress with zstd                                  │
│            - Upload to stout-index repo                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  CLIENT RUNTIME (stout)                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   Download index ──▶ Query SQLite ──▶ Fetch artifact ──▶ Install│
│   (~3MB, cached)     (instant)        (from CDN)                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Every new feature must follow this pattern:**
1. **Sync script** - Preprocesses source data into SQLite + compressed JSON
2. **Index schema** - SQLite tables with FTS5 for search
3. **Rust client** - Queries local index, fetches on-demand
4. **Hosted index** - GitHub repo or CDN

---

## Index Structure

### Current: stout-index

```
stout-index/
├── manifest.json           # Version, checksums, counts
├── index.db.zst            # SQLite with FTS5 (~1.5MB compressed)
└── formulas/
    ├── a/
    │   ├── aom.json.zst
    │   └── ...
    ├── b/
    └── ...
```

### Proposed: Unified stout-index

```
stout-index/
├── manifest.json           # Combined manifest
│
├── formulas/
│   ├── index.db.zst        # Formula SQLite index
│   └── data/
│       └── <letter>/<name>.json.zst
│
├── casks/
│   ├── index.db.zst        # Cask SQLite index
│   └── data/
│       └── <letter>/<token>.json.zst
│
├── linux-apps/
│   ├── index.db.zst        # AppImage/Flatpak index
│   └── data/
│       └── <id>.json.zst
│
└── vulnerabilities/
    └── index.db.zst        # CVE mappings (no individual files needed)
```

---

## Phase 1: Foundation & Quick Wins ✅ COMPLETED

### 1.1 Enhanced Dependency Visualization ✅

**No index changes needed** - uses existing formula index.

#### New Commands

```bash
stout deps --graph jq           # DOT output
stout deps --graph jq --json    # JSON adjacency list
stout why openssl               # Reverse dependency chain
```

#### Implementation

| Component | Changes |
|-----------|---------|
| `src/cli/deps.rs` | Add `--graph`, `--format` options |
| `src/cli/why.rs` | New command (uses existing dependency table) |

**Effort:** 1-2 days

---

### 1.2 Rollback/History Support ✅

**Client-side only** - history stored locally in `~/.stout/history.json`.

#### New Commands

```bash
stout history jq                # Show version history
stout rollback jq               # Revert to previous
stout switch jq 1.6             # Switch installed version
```

#### Implementation

| Component | Changes |
|-----------|---------|
| `crates/stout-state/src/history.rs` | New module |
| `src/cli/history.rs` | New command |
| `src/cli/rollback.rs` | New command |
| `src/cli/upgrade.rs` | Record history on upgrade |

**Storage:** `~/.stout/history.json`
```json
{
  "jq": [
    {"version": "1.7.1", "action": "upgrade", "timestamp": "...", "from": "1.7"},
    {"version": "1.7", "action": "install", "timestamp": "..."}
  ]
}
```

**Effort:** 2-3 days

---

## Phase 2: Cask Support ✅ COMPLETED

### 2.1 Cask Index (Preprocessing) ✅

**New sync script:** `scripts/sync_casks.py`

#### Source
- Homebrew Cask API: `https://formulae.brew.sh/api/cask.json`
- ~6000+ casks

#### Transform: Homebrew Cask → stout Cask

```python
def transform_cask(hb_cask: dict) -> dict:
    """Transform Homebrew cask JSON to stout format."""
    return {
        "token": hb_cask["token"],              # firefox
        "name": hb_cask.get("name", []),        # ["Firefox"]
        "version": hb_cask.get("version"),
        "sha256": hb_cask.get("sha256"),        # or "no_check"
        "url": hb_cask.get("url"),
        "homepage": hb_cask.get("homepage"),
        "desc": hb_cask.get("desc"),
        "artifacts": extract_artifacts(hb_cask),
        "caveats": hb_cask.get("caveats"),
        "depends_on": hb_cask.get("depends_on", {}),
        "conflicts_with": hb_cask.get("conflicts_with", []),
        "auto_updates": hb_cask.get("auto_updates", False),
        "deprecated": hb_cask.get("deprecated", False),
        "disabled": hb_cask.get("disabled", False),
    }

def extract_artifacts(hb_cask: dict) -> list:
    """Extract and normalize artifact definitions."""
    artifacts = []
    for artifact in hb_cask.get("artifacts", []):
        if "app" in artifact:
            artifacts.append({"type": "app", "source": artifact["app"][0]})
        elif "pkg" in artifact:
            artifacts.append({"type": "pkg", "path": artifact["pkg"][0]})
        elif "binary" in artifact:
            artifacts.append({"type": "binary", **artifact["binary"]})
        elif "zap" in artifact:
            artifacts.append({"type": "zap", **artifact["zap"]})
        elif "uninstall" in artifact:
            artifacts.append({"type": "uninstall", **artifact["uninstall"]})
    return artifacts
```

#### SQLite Schema: `casks/index.db`

```sql
-- Cask metadata
CREATE TABLE casks (
    token TEXT PRIMARY KEY,           -- firefox
    name TEXT,                        -- Firefox
    version TEXT NOT NULL,
    sha256 TEXT,                      -- or "no_check"
    url TEXT,
    homepage TEXT,
    desc TEXT,
    auto_updates INTEGER DEFAULT 0,
    deprecated INTEGER DEFAULT 0,
    disabled INTEGER DEFAULT 0,
    json_hash TEXT,
    updated_at INTEGER
);

-- Full-text search
CREATE VIRTUAL TABLE casks_fts USING fts5(
    token, name, desc,
    content='casks',
    content_rowid='rowid'
);

-- Artifacts (for quick queries)
CREATE TABLE artifacts (
    cask TEXT NOT NULL,
    type TEXT NOT NULL,              -- app, pkg, binary, zap, uninstall
    path TEXT,
    PRIMARY KEY (cask, type, path),
    FOREIGN KEY (cask) REFERENCES casks(token)
);

-- Dependencies
CREATE TABLE cask_dependencies (
    cask TEXT NOT NULL,
    dep_type TEXT NOT NULL,          -- formula, cask, macos
    dep_value TEXT NOT NULL,
    PRIMARY KEY (cask, dep_type, dep_value),
    FOREIGN KEY (cask) REFERENCES casks(token)
);
```

#### Output

```
casks/
├── index.db.zst          # ~500KB compressed
└── data/
    ├── f/
    │   ├── firefox.json.zst
    │   └── figma.json.zst
    └── ...
```

**Effort:** 2-3 days

---

### 2.2 Cask Client (macOS) ✅

**New crate:** `crates/stout-cask/`

#### Commands

```bash
stout install --cask firefox
stout search --cask browser
stout info --cask firefox
stout list --cask
stout uninstall --cask firefox
stout upgrade --cask
stout uninstall --cask --zap firefox   # Remove all traces
```

#### Crate Structure

```
crates/stout-cask/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── cask.rs              # Cask data types
    ├── database.rs          # SQLite cask index queries
    ├── sync.rs              # Index download/update
    └── install/
        ├── mod.rs
        ├── download.rs      # Download DMG/ZIP/PKG
        ├── dmg.rs           # Mount DMG, copy .app
        ├── pkg.rs           # Run installer -pkg
        ├── zip.rs           # Extract ZIP
        └── quarantine.rs    # Handle xattr quarantine
```

#### Installation Flow

```rust
pub async fn install_cask(token: &str, paths: &Paths) -> Result<()> {
    // 1. Query cask index
    let cask = db.get_cask(token)?;

    // 2. Download artifact
    let artifact_path = download_artifact(&cask.url, &cask.sha256).await?;

    // 3. Install based on artifact type
    match detect_artifact_type(&artifact_path) {
        ArtifactType::Dmg => install_from_dmg(&artifact_path, &cask).await?,
        ArtifactType::Zip => install_from_zip(&artifact_path, &cask).await?,
        ArtifactType::Pkg => install_from_pkg(&artifact_path, &cask).await?,
    }

    // 4. Handle quarantine
    remove_quarantine(&install_path)?;

    // 5. Write receipt and update state
    write_cask_receipt(token, &cask)?;
    update_installed_casks(token, &cask)?;

    Ok(())
}
```

#### Cask State Storage

Installed casks tracked in `~/.stout/casks.json`:

```json
{
  "firefox": {
    "version": "130.0",
    "installed_at": "2024-01-15T10:00:00Z",
    "artifact_path": "/Applications/Firefox.app",
    "auto_updates": true
  },
  "visual-studio-code": {
    "version": "1.85.0",
    "installed_at": "2024-01-14T15:30:00Z",
    "artifact_path": "/Applications/Visual Studio Code.app",
    "auto_updates": true
  }
}
```

#### DMG Installation (macOS)

```rust
pub async fn install_from_dmg(dmg_path: &Path, cask: &Cask) -> Result<PathBuf> {
    // 1. Mount DMG
    let mount_point = mount_dmg(dmg_path)?;

    // 2. Find .app bundle
    let app_bundle = find_app_in_mount(&mount_point)?;

    // 3. Copy to /Applications
    let dest = PathBuf::from("/Applications").join(app_bundle.file_name().unwrap());
    copy_dir_all(&app_bundle, &dest)?;

    // 4. Unmount DMG
    unmount_dmg(&mount_point)?;

    Ok(dest)
}

fn mount_dmg(dmg_path: &Path) -> Result<PathBuf> {
    let output = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mount_point)
        .arg(dmg_path)
        .output()?;
    // ...
}
```

**Effort:** 5-7 days

---

### 2.3 Linux Apps Index (Preprocessing) ✅

**New sync script:** `scripts/sync_linux_apps.py`

#### Sources

1. **AppImageHub API**: `https://appimage.github.io/feed.json`
2. **Flathub API**: `https://flathub.org/api/v2/appstream`

#### Transform

```python
def sync_linux_apps(output_dir: Path):
    """Sync AppImage and Flatpak data."""
    apps = {}

    # Fetch AppImages
    appimages = fetch_appimage_hub()
    for app in appimages:
        token = normalize_token(app["name"])
        apps[token] = {
            "token": token,
            "name": app["name"],
            "desc": app.get("description"),
            "homepage": app.get("links", [{}])[0].get("url"),
            "appimage": {
                "url": find_latest_release_url(app),
                "sha256": None,  # AppImages often don't provide
            },
            "flatpak": None,
        }

    # Fetch Flatpaks (enrich existing or add new)
    flatpaks = fetch_flathub()
    for app in flatpaks:
        token = normalize_token(app["name"])
        if token in apps:
            apps[token]["flatpak"] = {
                "app_id": app["id"],  # org.mozilla.firefox
                "remote": "flathub",
            }
        else:
            apps[token] = {
                "token": token,
                "name": app["name"],
                "desc": app.get("summary"),
                "homepage": app.get("url"),
                "appimage": None,
                "flatpak": {
                    "app_id": app["id"],
                    "remote": "flathub",
                },
            }

    # Create mappings from macOS cask tokens
    cask_mappings = load_cask_to_linux_mappings()  # Curated file
    for cask_token, linux_token in cask_mappings.items():
        if linux_token in apps:
            apps[linux_token]["cask_alias"] = cask_token

    return apps
```

#### SQLite Schema: `linux-apps/index.db`

```sql
CREATE TABLE linux_apps (
    token TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    desc TEXT,
    homepage TEXT,
    -- AppImage
    appimage_url TEXT,
    appimage_sha256 TEXT,
    -- Flatpak
    flatpak_app_id TEXT,           -- org.mozilla.firefox
    flatpak_remote TEXT,           -- flathub
    -- Cross-reference
    cask_alias TEXT,               -- Maps to macOS cask token
    json_hash TEXT,
    updated_at INTEGER
);

CREATE VIRTUAL TABLE linux_apps_fts USING fts5(
    token, name, desc,
    content='linux_apps'
);

-- Cask token mapping for cross-platform installs
CREATE TABLE cask_mappings (
    cask_token TEXT PRIMARY KEY,   -- firefox (macOS cask)
    linux_token TEXT NOT NULL,     -- firefox (linux app)
    FOREIGN KEY (linux_token) REFERENCES linux_apps(token)
);
```

#### Curated Mappings File: `mappings/cask_to_linux.json`

```json
{
  "firefox": "firefox",
  "google-chrome": "google-chrome",
  "visual-studio-code": "vscode",
  "slack": "slack",
  "discord": "discord",
  "spotify": "spotify",
  "vlc": "vlc",
  "gimp": "gimp",
  "obs": "obs-studio"
}
```

**Effort:** 3-4 days

---

### 2.4 Cask Client (Linux) ✅

#### Cross-Platform Install Logic

```rust
pub async fn install_cask(token: &str, paths: &Paths) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        install_macos_cask(token, paths).await
    }

    #[cfg(target_os = "linux")]
    {
        // Look up Linux equivalent
        let linux_app = match db.get_linux_app_by_cask(token)? {
            Some(app) => app,
            None => bail!("No Linux equivalent for cask '{}'", token),
        };

        install_linux_app(&linux_app, paths).await
    }
}
```

#### AppImage Installation

```rust
pub async fn install_appimage(app: &LinuxApp, paths: &Paths) -> Result<()> {
    let appimage = app.appimage.as_ref()
        .ok_or_else(|| anyhow!("No AppImage available"))?;

    // 1. Download to ~/.local/share/stout/appimages/
    let appimage_dir = paths.data_dir.join("appimages");
    let dest = appimage_dir.join(format!("{}.AppImage", app.token));
    download_file(&appimage.url, &dest).await?;

    // 2. Make executable
    std::fs::set_permissions(&dest, Permissions::from_mode(0o755))?;

    // 3. Create symlink in ~/.local/bin/
    let bin_link = paths.local_bin.join(&app.token);
    std::os::unix::fs::symlink(&dest, &bin_link)?;

    // 4. Extract .desktop file (optional)
    if let Ok(desktop) = extract_desktop_file(&dest) {
        let desktop_dest = paths.applications_dir.join(format!("{}.desktop", app.token));
        std::fs::write(&desktop_dest, desktop)?;
    }

    Ok(())
}
```

#### Flatpak Installation

```rust
pub async fn install_flatpak(app: &LinuxApp, _paths: &Paths) -> Result<()> {
    let flatpak = app.flatpak.as_ref()
        .ok_or_else(|| anyhow!("No Flatpak available"))?;

    // Use flatpak CLI
    let status = Command::new("flatpak")
        .args(["install", "--user", "-y", &flatpak.remote, &flatpak.app_id])
        .status()?;

    if !status.success() {
        bail!("flatpak install failed");
    }

    Ok(())
}
```

#### User Choice

```bash
# Auto-select best available
stout install --cask firefox

# Force specific format
stout install --cask firefox --appimage
stout install --cask firefox --flatpak
```

**Effort:** 4-5 days

---

### 2.5 Update Command Changes

After cask support, `stout update` updates all indexes:

```bash
stout update                    # Update all indexes (formulas, casks, linux-apps)
stout update --formulas-only    # Only formula index
stout update --casks-only       # Only cask index
```

**Implementation:** Modify `src/cli/update.rs` to:
1. Download formula index (existing)
2. Download cask index (new)
3. Download linux-apps index (new, Linux only)
4. Download vulnerability index (after Phase 4)

---

## Phase 3: Brewfile Support ✅ COMPLETED

**Depends on:** Phase 2 (Cask Support) for full `cask` entry support.

### 3.1 Brewfile Parser

**Client-side only** - no index needed.

#### brew bundle Compatibility

```ruby
# Taps
tap "homebrew/cask"
tap "homebrew/cask-fonts"

# Formulas
brew "jq"
brew "ripgrep", link: true
brew "postgresql@15", restart_service: :changed

# Casks
cask "firefox"
cask "visual-studio-code", args: { appdir: "~/Applications" }

# Mac App Store (optional, skip on Linux)
mas "Xcode", id: 497799835

# Whalebrew (optional)
whalebrew "whalebrew/wget"
```

#### Parser Implementation

**Strategy:** Try Ruby first (full compatibility), fall back to Rust parser (common cases). No intermediate files.

```
Brewfile ──▶ Ruby available? ──▶ Yes ──▶ ruby -e "..." ──▶ JSON ──▶ Rust
                   │
                   └──▶ No ──▶ Rust regex parser ──▶ Rust
```

**Ruby Parser (embedded, executed via `ruby -e`):**

```ruby
require 'json'
$e={taps:[],brews:[],casks:[],mas:[],whalebrew:[],vscode:[]}
def tap(n,**o) $e[:taps]<<{name:n}.merge(o) end
def brew(n,**o) $e[:brews]<<{name:n}.merge(o) end
def cask(n,**o) $e[:casks]<<{name:n}.merge(o) end
def mas(n,id:) $e[:mas]<<{name:n,id:id} end
def whalebrew(n) $e[:whalebrew]<<{name:n} end
def vscode(n) $e[:vscode]<<{name:n} end
eval(File.read(ARGV[0]))
puts $e.to_json
```

**Rust Implementation:**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Brewfile {
    #[serde(default)] pub taps: Vec<TapEntry>,
    #[serde(default)] pub brews: Vec<BrewEntry>,
    #[serde(default)] pub casks: Vec<CaskEntry>,
    #[serde(default)] pub mas: Vec<MasEntry>,
    #[serde(default)] pub whalebrew: Vec<WhalebrewEntry>,
    #[serde(default)] pub vscode: Vec<VscodeEntry>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BrewEntry {
    pub name: String,
    #[serde(default)] pub args: Vec<String>,
    #[serde(default)] pub link: Option<bool>,
    #[serde(default)] pub restart_service: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CaskEntry {
    pub name: String,
    #[serde(default)] pub greedy: bool,
}

// ... TapEntry, MasEntry, etc.

impl Brewfile {
    pub fn parse(path: &Path) -> Result<Self> {
        // Try Ruby first (full compatibility)
        match Self::parse_with_ruby(path) {
            Ok(bf) => return Ok(bf),
            Err(e) => {
                log::debug!("Ruby parser failed: {}, trying Rust parser", e);
            }
        }

        // Fall back to Rust parser
        Self::parse_with_rust(path)
    }

    fn parse_with_ruby(path: &Path) -> Result<Self> {
        const RUBY_SCRIPT: &str = r#"
require 'json'
$e={taps:[],brews:[],casks:[],mas:[],whalebrew:[],vscode:[]}
def tap(n,**o) $e[:taps]<<{name:n}.merge(o) end
def brew(n,**o) $e[:brews]<<{name:n}.merge(o) end
def cask(n,**o) $e[:casks]<<{name:n}.merge(o) end
def mas(n,id:) $e[:mas]<<{name:n,id:id} end
def whalebrew(n) $e[:whalebrew]<<{name:n} end
def vscode(n) $e[:vscode]<<{name:n} end
eval(File.read(ARGV[0]))
puts $e.to_json
"#;

        let output = Command::new("ruby")
            .arg("-e").arg(RUBY_SCRIPT)
            .arg(path)
            .output()?;

        if !output.status.success() {
            bail!("Ruby not available or parse error");
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    fn parse_with_rust(path: &Path) -> Result<Self> {
        eprintln!("Note: Ruby not found, using basic parser (some options may be ignored)");

        let content = std::fs::read_to_string(path)?;
        let mut bf = Brewfile::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }

            if let Some(rest) = line.strip_prefix("tap ") {
                bf.taps.push(TapEntry { name: extract_string(rest)?, ..Default::default() });
            } else if let Some(rest) = line.strip_prefix("brew ") {
                bf.brews.push(BrewEntry { name: extract_string(rest)?, ..Default::default() });
            } else if let Some(rest) = line.strip_prefix("cask ") {
                bf.casks.push(CaskEntry { name: extract_string(rest)?, ..Default::default() });
            }
            // mas, whalebrew, vscode...
        }

        Ok(bf)
    }
}

fn extract_string(s: &str) -> Result<String> {
    let s = s.trim();
    // Handle: "name" or 'name' or "name", options...
    if let Some(start) = s.find('"') {
        if let Some(end) = s[start+1..].find('"') {
            return Ok(s[start+1..start+1+end].to_string());
        }
    }
    if let Some(start) = s.find('\'') {
        if let Some(end) = s[start+1..].find('\'') {
            return Ok(s[start+1..start+1+end].to_string());
        }
    }
    bail!("Could not extract string from: {}", s)
}
```

**Behavior:**

```bash
$ stout bundle
==> Parsing Brewfile...
==> Installing 5 brews, 3 casks...

# If Ruby not available:
$ stout bundle
Note: Ruby not found, using basic parser (some options may be ignored)
==> Installing 5 brews, 3 casks...
```

#### Commands

```bash
stout bundle                    # Install from ./Brewfile
stout bundle --file=Brewfile.dev
stout bundle dump               # Generate from installed
stout bundle check              # Verify satisfied
stout bundle cleanup            # Remove unlisted
stout bundle list               # List entries
```

**Effort:** 3-4 days

---

### 3.2 Snapshot/Restore

**Client-side only** - uses local storage.

**Note:** stout already has `stout lock` for reproducible environments (Brewfile.lock.json).
Snapshots are different - they capture a named point-in-time state for quick switching.

| Feature | Lock | Snapshot |
|---------|------|----------|
| Purpose | Reproducible builds | Quick state switching |
| Storage | `Brewfile.lock.json` (project) | `~/.stout/snapshots/` (user) |
| Includes | Exact versions + checksums | Package list + metadata |
| Use case | CI/CD, team sharing | Personal backup, experiments |

```bash
stout snapshot create mysetup
stout snapshot create mysetup --description "Before upgrade"
stout snapshot list
stout snapshot show mysetup
stout snapshot restore mysetup
stout snapshot restore mysetup --dry-run
stout snapshot delete mysetup
stout snapshot export mysetup > backup.json
stout snapshot import < backup.json
```

**Storage:** `~/.stout/snapshots/<name>.json`

```json
{
  "name": "mysetup",
  "description": "Before upgrade",
  "created_at": "2024-01-15T10:00:00Z",
  "stout_version": "0.2.0",
  "formulas": [
    {"name": "jq", "version": "1.7.1", "revision": 0, "requested": true},
    {"name": "oniguruma", "version": "6.9.9", "revision": 0, "requested": false}
  ],
  "casks": [
    {"token": "firefox", "version": "130.0"}
  ],
  "pinned": ["postgresql@15"]
}
```

**Effort:** 2-3 days (after Brewfile)

---

## Phase 4: Vulnerability Index ✅ COMPLETED

### 4.1 Vulnerability Index (Preprocessing) ✅

**New sync script:** `scripts/sync_vulns.py`

#### Source

- **OSV (Open Source Vulnerabilities)**: `https://osv-vulnerabilities.storage.googleapis.com/`
- Ecosystem: "Homebrew" (not currently in OSV, may need to use "npm", "PyPI" as proxies)
- Alternative: NVD CVE database with CPE matching

#### Approach

Since OSV doesn't have a Homebrew ecosystem, we'll:
1. Map formula names to known package ecosystems (npm, PyPI, crates.io, etc.)
2. Query OSV for those packages
3. Build a pre-computed vulnerability index

```python
def sync_vulnerabilities(formulas_db: Path, output_dir: Path):
    """Build vulnerability index for known formulas."""
    conn = sqlite3.connect(formulas_db)

    vulns = {}

    # Load ecosystem mappings
    mappings = load_ecosystem_mappings()  # formula -> (ecosystem, package)

    for formula, (ecosystem, pkg_name) in mappings.items():
        # Query OSV
        response = requests.post(
            "https://api.osv.dev/v1/query",
            json={"package": {"name": pkg_name, "ecosystem": ecosystem}}
        )

        if response.ok:
            for vuln in response.json().get("vulns", []):
                vulns.setdefault(formula, []).append({
                    "id": vuln["id"],
                    "summary": vuln.get("summary"),
                    "severity": extract_severity(vuln),
                    "affected": vuln.get("affected", []),
                    "fixed": extract_fixed_version(vuln),
                })

    return vulns
```

#### SQLite Schema: `vulnerabilities/index.db`

```sql
CREATE TABLE vulnerabilities (
    id TEXT PRIMARY KEY,              -- CVE-2023-50246 or GHSA-xxx
    summary TEXT,
    severity TEXT,                    -- critical, high, medium, low
    published TEXT,
    modified TEXT
);

CREATE TABLE affected_packages (
    vuln_id TEXT NOT NULL,
    formula TEXT NOT NULL,            -- stout formula name
    affected_versions TEXT,           -- Version range expression
    fixed_version TEXT,
    PRIMARY KEY (vuln_id, formula),
    FOREIGN KEY (vuln_id) REFERENCES vulnerabilities(id)
);

CREATE INDEX idx_affected_formula ON affected_packages(formula);
```

#### Ecosystem Mappings: `mappings/formula_ecosystems.json`

```json
{
  "node": {"ecosystem": "npm", "package": "node"},
  "python@3.12": {"ecosystem": "PyPI", "package": "cpython"},
  "openssl@3": {"ecosystem": "OSS-Fuzz", "package": "openssl"},
  "curl": {"ecosystem": "OSS-Fuzz", "package": "curl"},
  "jq": {"ecosystem": "GitHub", "package": "jqlang/jq"}
}
```

**Effort:** 3-4 days

---

### 4.2 Audit Client ✅

```bash
stout audit                     # Scan all installed
stout audit jq                  # Scan specific
stout audit --format json       # Machine output
stout audit --update            # Update database first
stout audit --fail-on high      # CI/CD integration
```

#### Implementation

```rust
pub fn audit_packages(packages: &[String], db: &VulnDatabase) -> AuditReport {
    let mut report = AuditReport::default();

    for pkg in packages {
        let installed_version = get_installed_version(pkg)?;
        let vulns = db.get_vulnerabilities(pkg)?;

        for vuln in vulns {
            if vuln.affects_version(&installed_version) {
                report.add_finding(pkg, &vuln);
            }
        }
    }

    report
}
```

**Effort:** 2-3 days

---

## Phase 5: Offline Mode ✅ COMPLETED

Supports both **shared internal server** and **single air-gapped machine** deployments.

### 5.1 Mirror Configuration

#### Config Schema

```toml
# ~/.stout/config.toml

[mirror]
# Mirror URL (file:// for local, http:// for server)
url = "http://internal-mirror.company.com:8080"

# Fallback behavior when package not in mirror
# "error" (default) - Hard error, fail immediately
# "warn" - Warn and try upstream (if network available)
# "silent" - Silently try upstream
fallback = "error"

# SHA256 verification (disabled by default for speed)
verify_checksums = false

# Auto-update check interval (default: 7 days)
# Set to 0 to disable auto-update checks
update_check_interval_days = 7

[mirror.server]
# Default port for 'stout mirror serve'
port = 8080
bind = "0.0.0.0"
```

### 5.2 Mirror Creation

#### Commands

```bash
# Create mirror with specific packages (includes all dependencies)
stout mirror create ./mirror jq wget curl

# From Brewfile (recommended for teams)
stout mirror create ./mirror --from-brewfile Brewfile

# All currently installed packages
stout mirror create ./mirror --all-installed

# Include casks (macOS GUI apps)
stout mirror create ./mirror --cask firefox vscode

# Include Linux apps (AppImage/Flatpak)
stout mirror create ./mirror --linux-app firefox vscode

# Specify platforms (default: current platform only)
stout mirror create ./mirror jq --platforms=arm64_sonoma,x86_64_linux

# All platforms (warning: large download)
stout mirror create ./mirror jq --all-platforms

# Dry run - show what would be downloaded
stout mirror create ./mirror jq --dry-run

# Skip dependencies (advanced)
stout mirror create ./mirror jq --no-deps
```

#### Mirror Structure

```
mirror/
├── manifest.json                 # Master manifest with checksums
├── formulas/
│   ├── index.db.zst              # Filtered SQLite index
│   ├── data/
│   │   ├── j/jq.json.zst
│   │   ├── o/oniguruma.json.zst  # Dependency
│   │   └── w/wget.json.zst
│   └── bottles/
│       ├── jq-1.7.1.arm64_sonoma.bottle.tar.gz
│       ├── jq-1.7.1.x86_64_linux.bottle.tar.gz
│       ├── oniguruma-6.9.9.arm64_sonoma.bottle.tar.gz
│       └── wget-1.24.5.arm64_sonoma.bottle.tar.gz
│
├── casks/                        # macOS GUI applications
│   ├── index.db.zst
│   ├── data/
│   │   └── f/firefox.json.zst
│   └── artifacts/
│       └── Firefox-130.0.dmg
│
└── linux-apps/                   # Linux applications
    ├── index.db.zst
    ├── data/
    │   └── firefox.json.zst
    └── artifacts/
        ├── firefox.AppImage
        └── (flatpak handled via flatpak CLI)
```

#### Manifest Schema

```json
{
  "version": "2024.01.15.1200",
  "created_at": "2024-01-15T12:00:00Z",
  "stout_version": "0.2.0",
  "platforms": ["arm64_sonoma", "x86_64_linux"],

  "update_schedule": {
    "frequency": "weekly",
    "next_update": "2024-01-22T12:00:00Z"
  },

  "formulas": {
    "count": 15,
    "packages": {
      "jq": {
        "version": "1.7.1",
        "revision": 0,
        "json_path": "formulas/data/j/jq.json.zst",
        "bottles": {
          "arm64_sonoma": {
            "path": "formulas/bottles/jq-1.7.1.arm64_sonoma.bottle.tar.gz",
            "sha256": "abc123def456...",
            "size": 1234567
          },
          "x86_64_linux": {
            "path": "formulas/bottles/jq-1.7.1.x86_64_linux.bottle.tar.gz",
            "sha256": "789xyz...",
            "size": 2345678
          }
        }
      }
    }
  },

  "casks": {
    "count": 5,
    "packages": {
      "firefox": {
        "version": "130.0",
        "json_path": "casks/data/f/firefox.json.zst",
        "artifact": {
          "path": "casks/artifacts/Firefox-130.0.dmg",
          "sha256": "...",
          "size": 134217728
        }
      }
    }
  },

  "linux_apps": {
    "count": 3,
    "packages": {
      "firefox": {
        "json_path": "linux-apps/data/firefox.json.zst",
        "appimage": {
          "path": "linux-apps/artifacts/firefox.AppImage",
          "sha256": "...",
          "size": 98765432
        },
        "flatpak_id": "org.mozilla.firefox"
      }
    }
  },

  "checksums": {
    "formulas/index.db.zst": "sha256:...",
    "casks/index.db.zst": "sha256:...",
    "linux-apps/index.db.zst": "sha256:..."
  },

  "total_size": 5368709120
}
```

#### Implementation

```rust
pub struct MirrorConfig {
    pub output: PathBuf,
    pub packages: Vec<String>,
    pub casks: Vec<String>,
    pub linux_apps: Vec<String>,
    pub platforms: Vec<String>,
    pub include_deps: bool,           // Default: true
    pub brewfile: Option<PathBuf>,
}

pub async fn create_mirror(config: MirrorConfig) -> Result<MirrorManifest> {
    let mut manifest = MirrorManifest::new();

    // 1. Resolve packages with dependencies
    let packages = if config.include_deps {
        resolve_with_deps(&config.packages)?
    } else {
        config.packages.clone()
    };

    // 2. Create directory structure
    create_mirror_dirs(&config.output)?;

    // 3. Build filtered formula index
    let formula_db = filter_formula_index(&packages)?;
    let formula_index_path = config.output.join("formulas/index.db.zst");
    compress_and_write(&formula_db, &formula_index_path)?;
    manifest.add_checksum("formulas/index.db.zst", sha256_file(&formula_index_path)?);

    // 4. Download formula JSON and bottles
    for pkg in &packages {
        // Copy formula JSON
        let json_path = copy_formula_json(pkg, &config.output)?;

        // Download bottles for each platform
        for platform in &config.platforms {
            if let Some(bottle) = get_bottle_info(pkg, platform)? {
                let bottle_path = download_bottle(&bottle, &config.output).await?;
                manifest.add_formula_bottle(pkg, platform, &bottle_path, &bottle)?;
            }
        }
    }

    // 5. Process casks (if any)
    if !config.casks.is_empty() {
        let cask_db = filter_cask_index(&config.casks)?;
        compress_and_write(&cask_db, &config.output.join("casks/index.db.zst"))?;

        for cask in &config.casks {
            let cask_info = get_cask_info(cask)?;
            copy_cask_json(cask, &config.output)?;

            // Download artifact (DMG/PKG/ZIP)
            let artifact_path = download_cask_artifact(&cask_info, &config.output).await?;
            manifest.add_cask(cask, &artifact_path, &cask_info)?;
        }
    }

    // 6. Process Linux apps (if any)
    if !config.linux_apps.is_empty() {
        process_linux_apps(&config, &mut manifest).await?;
    }

    // 7. Write manifest
    let manifest_path = config.output.join("manifest.json");
    manifest.write(&manifest_path)?;

    Ok(manifest)
}
```

**Effort:** 4-5 days

---

### 5.3 Mirror Updates

```bash
# Check for outdated packages in mirror
stout mirror outdated ./mirror

# Update all packages to latest versions
stout mirror update ./mirror

# Update specific packages
stout mirror update ./mirror jq wget

# Update from Brewfile (add new, update existing)
stout mirror update ./mirror --from-brewfile Brewfile

# Prune old versions (keep only latest)
stout mirror prune ./mirror

# Prune keeping N versions
stout mirror prune ./mirror --keep=2
```

#### Update Schedule Defaults

| Setting | Default | Description |
|---------|---------|-------------|
| `update_check_interval_days` | 7 | Check for updates weekly |
| Formula updates | On check | Update to latest stable |
| Cask updates | On check | Update to latest version |
| Auto-prune | Disabled | Must run `stout mirror prune` manually |

**Effort:** 2-3 days

---

### 5.4 Mirror Server

#### Built-in Server

```bash
# Start server with defaults (port 8080, bind 0.0.0.0)
stout mirror serve ./mirror

# Custom port and bind
stout mirror serve ./mirror --port 9000 --bind 127.0.0.1

# With access logging
stout mirror serve ./mirror --log-access

# Background mode (daemon)
stout mirror serve ./mirror --daemon --pid-file=/var/run/stout-mirror.pid
```

#### External Server (nginx example)

```nginx
server {
    listen 8080;
    server_name mirror.internal;

    root /var/stout-mirror;

    location / {
        autoindex on;
        gzip_static on;
    }

    # Cache control for bottles (immutable)
    location ~* \.tar\.gz$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

#### Systemd Service (for production)

```ini
# /etc/systemd/system/stout-mirror.service
[Unit]
Description=stout Mirror Server
After=network.target

[Service]
Type=simple
User=stout
ExecStart=/usr/local/bin/stout mirror serve /var/stout-mirror --port 8080
Restart=always

[Install]
WantedBy=multi-user.target
```

**Effort:** 2-3 days

---

### 5.5 Mirror Client

#### Usage

```bash
# One-time override
stout --mirror=http://mirror.internal:8080 install jq

# File-based mirror (USB drive, local mount)
stout --mirror=file:///mnt/usb/stout-mirror install jq

# Configure as default
stout config set mirror.url "http://mirror.internal:8080"

# Then normal commands use mirror automatically
stout install jq
stout upgrade
stout search json
```

#### Client Behavior

```rust
pub struct MirrorClient {
    base_url: Url,
    config: MirrorClientConfig,
}

pub struct MirrorClientConfig {
    /// How to handle packages not in mirror
    /// Default: Fallback::Error
    pub fallback: Fallback,

    /// Verify SHA256 checksums
    /// Default: false (skip for speed)
    pub verify_checksums: bool,
}

#[derive(Default)]
pub enum Fallback {
    #[default]
    Error,      // Hard error if package not in mirror
    Warn,       // Warn and try upstream if network available
    Silent,     // Silently try upstream
}

impl MirrorClient {
    pub async fn install(&self, package: &str) -> Result<()> {
        // 1. Check manifest for package
        let manifest = self.fetch_manifest().await?;

        match manifest.get_formula(package) {
            Some(info) => {
                // Package in mirror - install from mirror
                self.install_from_mirror(package, info).await
            }
            None => {
                // Package not in mirror - handle based on fallback config
                match self.config.fallback {
                    Fallback::Error => {
                        bail!(
                            "Package '{}' not found in mirror.\n\
                             Available packages: stout --mirror=... search\n\
                             To add to mirror: stout mirror update <path> {}",
                            package, package
                        )
                    }
                    Fallback::Warn => {
                        eprintln!(
                            "Warning: '{}' not in mirror, trying upstream...",
                            package
                        );
                        self.install_from_upstream(package).await
                    }
                    Fallback::Silent => {
                        self.install_from_upstream(package).await
                    }
                }
            }
        }
    }

    async fn install_from_mirror(&self, package: &str, info: &ManifestFormula) -> Result<()> {
        let platform = detect_platform();
        let bottle_info = info.bottles.get(&platform)
            .ok_or_else(|| anyhow!(
                "No bottle for platform '{}' in mirror. Available: {:?}",
                platform,
                info.bottles.keys().collect::<Vec<_>>()
            ))?;

        // Download bottle from mirror
        let bottle_url = self.base_url.join(&bottle_info.path)?;
        let bottle_bytes = fetch_url(&bottle_url).await?;

        // Verify checksum (if enabled)
        if self.config.verify_checksums {
            let actual = sha256_bytes(&bottle_bytes);
            if actual != bottle_info.sha256 {
                bail!(
                    "Checksum mismatch for {}:\n  Expected: {}\n  Got: {}",
                    package, bottle_info.sha256, actual
                );
            }
        }

        // Save to cache and install
        let cache_path = save_to_cache(&bottle_bytes, package)?;
        extract_and_install(&cache_path)?;

        Ok(())
    }
}
```

#### Error Messages

```
# Package not in mirror (hard error - default)
$ stout --mirror=file:///mnt/mirror install unknown-pkg
Error: Package 'unknown-pkg' not found in mirror.

Available packages: stout --mirror=file:///mnt/mirror search
To add to mirror: stout mirror update /mnt/mirror unknown-pkg

# Platform not available
$ stout --mirror=file:///mnt/mirror install jq
Error: No bottle for platform 'arm64_ventura' in mirror.
Available platforms: arm64_sonoma, x86_64_linux

To add platform: stout mirror update /mnt/mirror jq --platforms=arm64_ventura
```

**Effort:** 3-4 days

---

### 5.6 Mirror Verification

```bash
# Verify mirror integrity
stout mirror verify ./mirror

# Output:
# Verifying mirror at ./mirror
#   ✓ manifest.json valid
#   ✓ formulas/index.db.zst (sha256 match)
#   ✓ 15 formula bottles verified
#   ✓ 5 cask artifacts verified
#
# Mirror verified: 20 files, 5.2 GB total

# Verify with detailed output
stout mirror verify ./mirror --verbose

# Verify specific packages only
stout mirror verify ./mirror jq wget
```

**Effort:** 1 day

---

### Offline Mode Summary

| Component | Effort | Priority |
|-----------|--------|----------|
| Mirror creation | 4-5 days | P0 |
| Mirror updates | 2-3 days | P0 |
| Mirror server | 2-3 days | P1 |
| Mirror client | 3-4 days | P0 |
| Verification | 1 day | P2 |
| **Total** | **13-16 days** | |

### Configuration Defaults

| Setting | Default | Rationale |
|---------|---------|-----------|
| `fallback` | `"error"` | Predictable behavior in air-gapped env |
| `verify_checksums` | `false` | Speed over security (user choice) |
| `update_check_interval_days` | `7` | Weekly is reasonable for most teams |
| `include_deps` | `true` | Avoid broken installs |
| `platforms` | Current only | Minimize mirror size |

---

## Phase 6: Developer Tools ✅ COMPLETED

### 6.1 Build Improvements ✅

No index changes - client-side only.

```bash
stout install foo -s --jobs=8
stout install foo -s --cc=clang
stout bottle create foo           # Create local bottle
```

**Effort:** 4-5 days

---

### 6.2 Formula/Cask Creation ✅

```bash
stout create https://github.com/user/project/archive/v1.0.tar.gz
stout create --cask https://example.com/app.dmg
stout audit --formula ./foo.rb
stout test foo
```

**Effort:** 4-5 days

---

### 6.3 Analytics (Opt-in) ✅

Simple HTTP POST - no index needed.

```bash
stout analytics on|off|status
```

**Effort:** 1-2 days

---

## Phase 7: Advanced ✅ COMPLETED

### 7.1 Multi-Prefix Support ✅

```bash
# Create isolated prefix
stout prefix create ~/project/.stout

# Manage prefixes
stout prefix list
stout prefix info ~/project/.stout
stout prefix default ~/project/.stout
stout prefix remove ~/project/.stout

# Use custom prefix with any command
stout --prefix=~/project/.stout install jq
stout --prefix=~/project/.stout list

# Or set via environment variable
export STOUT_PREFIX=~/project/.stout
stout install jq
```

**Effort:** 3-4 days

---

## Implementation Summary

### New Sync Scripts

| Script | Source | Output |
|--------|--------|--------|
| `sync.py` (existing) | Homebrew Formula API | `formulas/index.db` |
| `sync_casks.py` | Homebrew Cask API | `casks/index.db` |
| `sync_linux_apps.py` | AppImageHub + Flathub | `linux-apps/index.db` |
| `sync_vulns.py` | OSV Database | `vulnerabilities/index.db` |

### New Rust Crates

| Crate | Purpose | Phase |
|-------|---------|-------|
| `stout-cask` | Cask index queries + installation | 2 |
| `stout-bundle` | Brewfile parsing | 3 |
| `stout-audit` | Vulnerability scanning | 4 |
| `stout-mirror` | Offline mirror creation/serving | 5 |

### Curated Mapping Files

| File | Purpose |
|------|---------|
| `mappings/cask_to_linux.json` | macOS cask → Linux app mapping |
| `mappings/formula_ecosystems.json` | Formula → OSV ecosystem mapping |

---

## Timeline

| Phase | Features | Effort | Status |
|-------|----------|--------|--------|
| 1. Foundation | Dep viz, Rollback, Why | ~4 days | ✅ Done |
| 2. Casks | Index + macOS + Linux | ~15 days | ✅ Done |
| 3. Brewfile | Parser + Snapshot | ~6 days | ✅ Done |
| 4. Vulnerabilities | Index + Audit | ~6 days | ✅ Done |
| 5. Offline | Mirror create/serve/client | ~14 days | ✅ Done |
| 6. Developer | Build, Create, Analytics | ~10 days | ✅ Done |
| 7. Advanced | Multi-prefix | ~4 days | ✅ Done |
| **Total** | | **~59 days** | **✅ Complete** |

---

## Phase Dependencies

```
Phase 1 (Foundation)
    │
    ├──▶ Phase 2 (Casks) ──▶ Phase 3 (Brewfile)
    │         │
    │         └──────────────▶ Phase 5 (Offline) ◀── Phase 4 (Vulns)
    │
    └──▶ Phase 6 (Developer Tools)
              │
              └──▶ Phase 7 (Multi-prefix)
```

**Critical path:** Phase 1 → Phase 2 → Phase 3 → Phase 5

---

## New CLI Commands Summary

Commands to add to `src/cli/mod.rs`:

| Phase | Command | Description |
|-------|---------|-------------|
| 1 | `why` | Show why a package is installed |
| 1 | `history` | Show package version history |
| 1 | `rollback` | Revert to previous version |
| 1 | `switch` | Switch between installed versions |
| 3 | `bundle` | Brewfile management |
| 3 | `snapshot` | State snapshots |
| 4 | (extends `audit`) | Vulnerability scanning via existing audit |
| 5 | `mirror` | Offline mirror management |
| 6 | `bottle` | Create and manage bottles |
| 6 | `create` | Create formula/cask from URL |
| 6 | `test` | Test installed packages |
| 6 | `analytics` | Manage opt-in analytics |
| 7 | `prefix` | Manage multiple installation prefixes |

**Note:** Cask commands use existing commands with `--cask` flag, not new subcommands.

---

## Index Hosting

All indexes hosted in single repo: `github.com/neul-labs/stout-index`

```
stout-index/
├── manifest.json              # Combined version info
├── formulas/                  # ~3MB
├── casks/                     # ~2MB (estimated)
├── linux-apps/                # ~1MB (estimated)
└── vulnerabilities/           # ~500KB (estimated)
```

Total index size: ~7MB compressed

CI/CD rebuilds index daily (or on Homebrew API changes).

---

## Design Decisions

Resolved decisions for implementation:

### Architecture
1. **Single vs separate index DBs**: **Separate SQLite files**
   - Each index (formulas, casks, linux-apps, vulnerabilities) is a separate `.db.zst` file
   - Rationale: Smaller downloads, users can opt-in to what they need

2. **Linux app priority**: **Flatpak preferred**
   - When both AppImage and Flatpak available, default to Flatpak
   - Rationale: Better sandboxing, desktop integration, automatic updates
   - Override with `--appimage` flag when needed

### Compatibility
3. **Brewfile parsing**: **Use Ruby for full compatibility**
   - Strategy: Shell out to Ruby to parse Brewfile and output structured YAML/JSON
   - stout reads the parsed output, not the Ruby DSL directly
   - Rationale: 100% compatibility with brew bundle, handles all edge cases

   ```bash
   # Internal implementation
   ruby -e "require 'yaml'; eval(File.read('Brewfile')); puts $entries.to_yaml" | stout bundle --from-yaml
   ```

   ```ruby
   # Helper script: stout-parse-brewfile.rb
   $entries = {taps: [], brews: [], casks: [], mas: []}

   def tap(name, **opts)
     $entries[:taps] << {name: name, **opts}
   end

   def brew(name, **opts)
     $entries[:brews] << {name: name, **opts}
   end

   def cask(name, **opts)
     $entries[:casks] << {name: name, **opts}
   end

   def mas(name, id:)
     $entries[:mas] << {name: name, id: id}
   end

   eval(File.read(ARGV[0] || 'Brewfile'))
   puts $entries.to_yaml
   ```

4. **Cask artifacts**: **Full compatibility - all artifact types**
   - Support all Homebrew cask artifact types:
     - `app` - Application bundles
     - `pkg` - macOS installer packages
     - `binary` - Executable binaries
     - `installer` - Custom installer scripts
     - `suite` - Application suites
     - `artifact` - Generic artifacts
     - `prefpane` - System preference panes
     - `qlplugin` - Quick Look plugins
     - `mdimporter` - Spotlight importers
     - `colorpicker` - Color pickers
     - `dictionary` - Dictionary files
     - `font` - Font files
     - `input_method` - Input methods
     - `internet_plugin` - Browser plugins
     - `audio_unit_plugin` - Audio plugins
     - `vst_plugin`, `vst3_plugin` - VST plugins
     - `screen_saver` - Screen savers
     - `service` - macOS services
     - `zap` - Deep uninstall
     - `uninstall` - Uninstall stanzas
   - Rationale: Full brew compatibility is a core goal

### Operations
5. **Mirror authentication**: **No authentication support**
   - Mirrors are public/internal network only
   - Rationale: Simplicity; use network-level security (VPN, firewall) instead
   - Future: Can add if enterprise demand requires it

6. **Vulnerability severity threshold**: **Warn only (like brew)**
   - Default: Show warnings but don't block installs
   - `stout install --audit` - Warn about vulnerabilities
   - `stout install --audit --fail-on=critical` - Block on critical
   - `stout install --audit --fail-on=high` - Block on high+
   - Rationale: Matches brew behavior, non-breaking default

### User Experience
7. **Cask upgrade behavior**: **Like brew - skip auto-updating casks**
   - `stout upgrade --cask` - Skip casks with `auto_updates: true`
   - `stout upgrade --cask --greedy` - Include auto-updating casks
   - Rationale: Matches brew behavior, respects app self-update mechanisms

8. **Error recovery**: **Atomic by default, best-effort optional**
   - Default: If any package fails, rollback all changes
   - `stout install --best-effort pkg1 pkg2 pkg3` - Continue on failures
   - Rationale: Predictable default, power users can opt into partial installs

   ```rust
   pub enum InstallMode {
       Atomic,      // Default: rollback all on failure
       BestEffort,  // Continue, report failures at end
   }
   ```
