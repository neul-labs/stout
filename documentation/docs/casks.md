# Casks (Applications)

Casks allow you to install macOS applications and Linux desktop apps.

---

## What are Casks?

Casks extend Stout to install:

- **macOS:** DMG, PKG, and ZIP application bundles
- **Linux:** AppImage, Flatpak, and other desktop applications

Unlike formulas (command-line tools), casks install GUI applications.

---

## Basic Usage

### Search for Applications

```bash
stout cask search firefox
```

### Install an Application

```bash
stout cask install firefox
```

Or use the `--cask` flag with install:

```bash
stout install --cask firefox
```

### List Installed Applications

```bash
stout cask list
```

### Get Application Info

```bash
stout cask info visual-studio-code
```

---

## Managing Applications

### Uninstall

Remove an application:

```bash
stout cask uninstall firefox
```

### Uninstall with Zap

Remove application and all associated files (preferences, caches, etc.):

```bash
stout cask uninstall --zap firefox
```

!!! warning
    The `--zap` option removes user preferences and data. Use with caution.

### Check for Updates

```bash
stout cask outdated
```

### Upgrade Applications

Upgrade a specific application:

```bash
stout cask upgrade firefox
```

Upgrade all applications:

```bash
stout cask upgrade
```

---

## Installation Locations

### macOS

- **Applications:** `/Applications` or `~/Applications`
- **Binaries:** Symlinked to `/opt/homebrew/bin`

### Linux

- **AppImages:** `~/.local/share/appimages`
- **Binaries:** Symlinked to `~/.local/bin`

---

## Common Casks

### Browsers

```bash
stout cask install firefox
stout cask install google-chrome
stout cask install brave-browser
```

### Development Tools

```bash
stout cask install visual-studio-code
stout cask install iterm2
stout cask install docker
stout cask install postman
```

### Productivity

```bash
stout cask install slack
stout cask install notion
stout cask install 1password
```

### Media

```bash
stout cask install vlc
stout cask install spotify
stout cask install obs
```

---

## Cask Options

### Force Reinstall

```bash
stout cask install --force firefox
```

### Skip Quarantine (macOS)

By default, macOS quarantines downloaded applications. To skip:

```bash
stout cask install --no-quarantine some-app
```

---

## Casks in Brewfiles

Include casks in your Brewfile:

```ruby
# Brewfile

# Formulas (CLI tools)
brew "git"
brew "node"

# Casks (Applications)
cask "firefox"
cask "visual-studio-code"
cask "docker"
```

Install everything:

```bash
stout bundle install
```

---

## Troubleshooting

### Application won't open (macOS)

If macOS blocks the application:

1. Open System Preferences > Security & Privacy
2. Click "Open Anyway" for the blocked application

Or remove quarantine attribute:

```bash
xattr -d com.apple.quarantine /Applications/SomeApp.app
```

### Application not found after install

Check if the cask includes a binary:

```bash
stout cask info some-app
```

If it doesn't add to PATH, launch from Applications folder.

### Cask conflicts with existing installation

Uninstall the existing version first, or use `--force`:

```bash
stout cask install --force some-app
```

---

## Preferred Syntax: `--cask` Flag

The `stout cask <command>` subcommand exists for muscle-memory compatibility
with `brew cask` but is deprecated in current releases. Each invocation now
prints a deprecation notice pointing at the canonical form, which uses the
`--cask` flag on the top-level command:

| Deprecated | Preferred |
|------------|-----------|
| `stout cask install firefox` | `stout install --cask firefox` |
| `stout cask uninstall firefox` | `stout uninstall --cask firefox` |
| `stout cask search browser` | `stout search --cask browser` |
| `stout cask info visual-studio-code` | `stout info --cask visual-studio-code` |
| `stout cask list` | `stout list --cask` |
| `stout cask outdated` | `stout outdated --cask` |
| `stout cask upgrade` | `stout upgrade --cask` |

Cask logic lives in the `stout-cask` crate and is dispatched from
`src/cli/cask.rs` for the deprecated path and from each top-level command
module for the canonical path.

---

## Linux Cask Handling

On Linux the cask abstraction wraps three different distribution formats:

| Format | Install target | Detection |
|--------|----------------|-----------|
| AppImage | `~/.local/share/appimages/<app>.AppImage` (chmod +x, symlinked into `~/.local/bin`) | `.AppImage` artefact in the cask manifest |
| Flatpak | Delegated to the host `flatpak` CLI under the user installation | `flatpak` provider declared in the manifest |
| Tarball/zip | Extracted into `~/.local/share/<app>/`, binaries symlinked into `~/.local/bin` | Default fallback |

The same `--cask` flag is used regardless of which underlying format the
package uses — stout picks the right backend based on the manifest.

---

## Quarantine and Code Signing (macOS)

When stout installs a cask on macOS it preserves whatever code signature the
upstream artefact carries and applies an `xattr` quarantine flag identical
to what a browser download would attach. This means Gatekeeper still runs
the first time you launch the app.

`stout doctor` includes a Mach-O signature check that walks every binary
under the Cellar and Caskroom and surfaces anything missing or invalid.
Pair it with `stout doctor --fix` to re-sign or reinstall affected packages
in one pass.
