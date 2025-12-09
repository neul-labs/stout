#!/usr/bin/env node
/**
 * Pre-uninstall script for stout npm package
 * Cleans up the downloaded binary
 */

const fs = require('fs');
const path = require('path');

const binDir = path.join(__dirname, '..', 'bin');

try {
  if (fs.existsSync(binDir)) {
    fs.rmSync(binDir, { recursive: true, force: true });
    console.log('stout binary removed');
  }
} catch (error) {
  // Ignore cleanup errors
}
