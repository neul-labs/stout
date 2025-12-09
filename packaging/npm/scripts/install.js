#!/usr/bin/env node
/**
 * Post-install script for stout npm package
 * Downloads the appropriate binary for the current platform
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');
const zlib = require('zlib');

const REPO = 'neul-labs/stout';
const BINARY_NAME = 'stout';

function getPlatform() {
  const platform = os.platform();
  switch (platform) {
    case 'darwin': return 'darwin';
    case 'linux': return 'linux';
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }
}

function getArch() {
  const arch = os.arch();
  switch (arch) {
    case 'x64': return 'x86_64';
    case 'arm64': return 'aarch64';
    default:
      throw new Error(`Unsupported architecture: ${arch}`);
  }
}

function getTarget(platform, arch) {
  const targets = {
    'darwin-x86_64': 'x86_64-apple-darwin',
    'darwin-aarch64': 'aarch64-apple-darwin',
    'linux-x86_64': 'x86_64-unknown-linux-gnu',
    'linux-aarch64': 'aarch64-unknown-linux-gnu',
  };
  const key = `${platform}-${arch}`;
  const target = targets[key];
  if (!target) {
    throw new Error(`Unsupported platform/arch combination: ${key}`);
  }
  return target;
}

function getLatestVersion() {
  return new Promise((resolve, reject) => {
    const url = `https://api.github.com/repos/${REPO}/releases/latest`;
    const options = {
      headers: {
        'User-Agent': 'stout-npm-installer',
        'Accept': 'application/vnd.github.v3+json'
      }
    };

    https.get(url, options, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          const release = JSON.parse(data);
          resolve(release.tag_name);
        } catch (e) {
          reject(new Error('Failed to parse release info'));
        }
      });
    }).on('error', reject);
  });
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const options = {
      headers: {
        'User-Agent': 'stout-npm-installer'
      }
    };

    const request = (url) => {
      https.get(url, options, (res) => {
        if (res.statusCode === 302 || res.statusCode === 301) {
          request(res.headers.location);
          return;
        }

        if (res.statusCode !== 200) {
          reject(new Error(`Download failed with status ${res.statusCode}`));
          return;
        }

        const file = fs.createWriteStream(dest);
        res.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      }).on('error', reject);
    };

    request(url);
  });
}

function extractTarGz(tarPath, destDir) {
  // Use tar command for extraction
  execSync(`tar -xzf "${tarPath}" -C "${destDir}"`, { stdio: 'inherit' });
}

async function main() {
  console.log('Installing stout...');

  try {
    const platform = getPlatform();
    const arch = getArch();
    const target = getTarget(platform, arch);

    console.log(`Platform: ${platform}-${arch} (${target})`);

    // Get version from package.json or fetch latest
    const packageJson = require('../package.json');
    let version = `v${packageJson.version}`;

    // Try to get the latest version if this is a fresh install
    try {
      const latestVersion = await getLatestVersion();
      if (latestVersion) {
        version = latestVersion;
      }
    } catch (e) {
      console.log(`Using package version: ${version}`);
    }

    console.log(`Downloading stout ${version}...`);

    const archiveName = `stout-${target}.tar.gz`;
    const downloadUrl = `https://github.com/${REPO}/releases/download/${version}/${archiveName}`;

    const binDir = path.join(__dirname, '..', 'bin');
    const tmpDir = os.tmpdir();
    const archivePath = path.join(tmpDir, archiveName);

    // Create bin directory
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    // Download archive
    await downloadFile(downloadUrl, archivePath);

    // Extract
    console.log('Extracting...');
    extractTarGz(archivePath, binDir);

    // Make executable
    const binaryPath = path.join(binDir, BINARY_NAME);
    fs.chmodSync(binaryPath, 0o755);

    // Cleanup
    fs.unlinkSync(archivePath);

    console.log(`stout installed successfully to ${binaryPath}`);

    // Verify
    try {
      const versionOutput = execSync(`"${binaryPath}" --version`, { encoding: 'utf8' });
      console.log(`Installed: ${versionOutput.trim()}`);
    } catch (e) {
      // Binary may not run on all systems (e.g., missing glibc)
      console.log('Binary installed. Run "stout --version" to verify.');
    }

  } catch (error) {
    console.error('Installation failed:', error.message);
    console.error('You can install stout manually from: https://github.com/neul-labs/stout/releases');
    process.exit(1);
  }
}

main();
