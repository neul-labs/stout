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
