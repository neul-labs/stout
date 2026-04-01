//! Create command - create formula or cask from URL

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use console::style;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    /// URL to the source archive or application
    pub url: String,

    /// Create a cask instead of a formula
    #[arg(long)]
    pub cask: bool,

    /// Output directory (default: current directory)
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// Formula/cask name (auto-detected from URL if not provided)
    #[arg(long)]
    pub name: Option<String>,

    /// Version string (auto-detected from URL if not provided)
    #[arg(long)]
    pub version: Option<String>,

    /// Homepage URL
    #[arg(long)]
    pub homepage: Option<String>,

    /// Description
    #[arg(long)]
    pub desc: Option<String>,

    /// License (e.g., MIT, Apache-2.0)
    #[arg(long)]
    pub license: Option<String>,

    /// Skip downloading and calculating checksum
    #[arg(long)]
    pub skip_download: bool,
}

pub async fn run(args: Args) -> Result<()> {
    if args.cask {
        create_cask(args).await
    } else {
        create_formula(args).await
    }
}

async fn create_formula(args: Args) -> Result<()> {
    println!(
        "\n{} formula from {}...\n",
        style("Creating").cyan().bold(),
        args.url
    );

    // Extract name from URL if not provided
    let name = match &args.name {
        Some(n) => n.clone(),
        None => extract_name_from_url(&args.url)?,
    };

    // Extract version from URL if not provided
    let version = match &args.version {
        Some(v) => v.clone(),
        None => extract_version_from_url(&args.url).unwrap_or_else(|| "1.0.0".to_string()),
    };

    println!("  Name: {}", style(&name).green());
    println!("  Version: {}", style(&version).green());

    // Calculate SHA256 if needed
    let sha256 = if args.skip_download {
        "REPLACE_WITH_SHA256".to_string()
    } else {
        println!("  {}...", style("Calculating SHA256").dim());
        calculate_sha256(&args.url).await?
    };
    println!("  SHA256: {}", style(&sha256).dim());

    // Generate formula content
    let formula = generate_formula(
        &name,
        &version,
        &args.url,
        &sha256,
        args.desc.as_deref(),
        args.homepage.as_deref(),
        args.license.as_deref(),
    );

    // Determine output path
    let output_dir = args.output.unwrap_or_else(|| PathBuf::from("."));
    let output_path = output_dir.join(format!("{}.rb", name));

    // Write formula
    std::fs::write(&output_path, &formula)?;

    println!(
        "\n{} Formula created: {}\n",
        style("✓").green().bold(),
        output_path.display()
    );

    println!("{}", style("Next steps:").bold());
    println!("  1. Edit the formula to add build instructions");
    println!("  2. Test with: stout test {}", name);
    println!(
        "  3. Audit with: stout audit --formula {}",
        output_path.display()
    );

    Ok(())
}

async fn create_cask(args: Args) -> Result<()> {
    println!(
        "\n{} cask from {}...\n",
        style("Creating").cyan().bold(),
        args.url
    );

    // Extract name from URL if not provided
    let name = match &args.name {
        Some(n) => n.clone(),
        None => extract_name_from_url(&args.url)?,
    };

    // Extract version from URL if not provided
    let version = match &args.version {
        Some(v) => v.clone(),
        None => extract_version_from_url(&args.url).unwrap_or_else(|| "1.0.0".to_string()),
    };

    println!("  Token: {}", style(&name).green());
    println!("  Version: {}", style(&version).green());

    // Calculate SHA256 if needed
    let sha256 = if args.skip_download {
        "REPLACE_WITH_SHA256".to_string()
    } else {
        println!("  {}...", style("Calculating SHA256").dim());
        calculate_sha256(&args.url).await?
    };
    println!("  SHA256: {}", style(&sha256).dim());

    // Determine app name
    let app_name = args.desc.as_deref().unwrap_or(&name);

    // Generate cask content
    let cask = generate_cask(
        &name,
        &version,
        &args.url,
        &sha256,
        app_name,
        args.homepage.as_deref(),
    );

    // Determine output path
    let output_dir = args.output.unwrap_or_else(|| PathBuf::from("."));
    let output_path = output_dir.join(format!("{}.rb", name));

    // Write cask
    std::fs::write(&output_path, &cask)?;

    println!(
        "\n{} Cask created: {}\n",
        style("✓").green().bold(),
        output_path.display()
    );

    println!("{}", style("Next steps:").bold());
    println!("  1. Edit the cask to specify artifacts (app, pkg, binary, etc.)");
    println!(
        "  2. Audit with: stout audit --cask {}",
        output_path.display()
    );

    Ok(())
}

fn extract_name_from_url(url: &str) -> Result<String> {
    // Try to extract name from URL
    // Patterns:
    // - github.com/owner/repo/archive/v1.0.0.tar.gz -> repo
    // - example.com/app-1.0.0.tar.gz -> app
    // - example.com/app.dmg -> app

    if url.contains("github.com") {
        // GitHub pattern: /owner/repo/...
        let parts: Vec<&str> = url.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "github.com" && i + 2 < parts.len() {
                return Ok(parts[i + 2].to_lowercase());
            }
        }
    }

    // Try to get filename
    let filename = url.rsplit('/').next().unwrap_or("unknown");

    // Remove common extensions and version patterns
    let name = filename
        .trim_end_matches(".tar.gz")
        .trim_end_matches(".tgz")
        .trim_end_matches(".tar.xz")
        .trim_end_matches(".tar.bz2")
        .trim_end_matches(".zip")
        .trim_end_matches(".dmg")
        .trim_end_matches(".pkg");

    // Remove version suffix (e.g., -1.0.0 or _1.0.0)
    let name = if let Some(idx) = name.rfind(['-', '_']) {
        let suffix = &name[idx + 1..];
        if suffix
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            &name[..idx]
        } else {
            name
        }
    } else {
        name
    };

    if name.is_empty() || name == "unknown" {
        bail!("Could not determine name from URL. Please provide --name");
    }

    Ok(name.to_lowercase().replace(' ', "-"))
}

fn extract_version_from_url(url: &str) -> Option<String> {
    // Try common version patterns
    // - v1.0.0, v1.0, v1
    // - 1.0.0, 1.0, 1

    // Check for GitHub release pattern
    if url.contains("/releases/") || url.contains("/archive/") {
        for part in url.split('/') {
            // Strip common archive extensions first
            let stripped = part
                .trim_end_matches(".tar.gz")
                .trim_end_matches(".tgz")
                .trim_end_matches(".zip")
                .trim_end_matches(".tar.xz")
                .trim_end_matches(".tar.bz2");
            let cleaned = stripped.trim_start_matches('v');
            if cleaned
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                // Check if it looks like a version
                let has_dots = cleaned.contains('.');
                let all_valid = cleaned.chars().all(|c| c.is_ascii_digit() || c == '.');
                if has_dots && all_valid {
                    return Some(cleaned.to_string());
                }
            }
        }
    }

    // Try filename pattern
    let filename = url.rsplit('/').next()?;
    for pattern in ["-", "_"] {
        if let Some(idx) = filename.rfind(pattern) {
            let suffix = &filename[idx + 1..];
            let version = suffix
                .trim_start_matches('v')
                .trim_end_matches(".tar.gz")
                .trim_end_matches(".tgz")
                .trim_end_matches(".zip")
                .trim_end_matches(".dmg");

            if version
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                return Some(version.to_string());
            }
        }
    }

    None
}

async fn calculate_sha256(url: &str) -> Result<String> {
    let client = Client::builder().user_agent("stout/0.1.0").build()?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to download file")?;

    if !response.status().is_success() {
        bail!("Failed to download: HTTP {}", response.status());
    }

    let bytes = response.bytes().await?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());

    Ok(hash)
}

fn generate_formula(
    name: &str,
    _version: &str,
    url: &str,
    sha256: &str,
    desc: Option<&str>,
    homepage: Option<&str>,
    license: Option<&str>,
) -> String {
    let class_name = to_class_name(name);
    // Use provided values or clear placeholders that must be filled in
    let desc = desc.unwrap_or("REPLACE: Add a one-line description of this software");
    let homepage = homepage.unwrap_or("REPLACE: https://example.com");
    let license = license.unwrap_or("REPLACE: e.g., MIT, Apache-2.0");

    format!(
        r#"class {class_name} < Formula
  desc "{desc}"
  homepage "{homepage}"
  url "{url}"
  sha256 "{sha256}"
  license "{license}"

  # Uncomment dependencies as needed:
  # depends_on "cmake" => :build
  # depends_on "pkg-config" => :build
  # depends_on "openssl"

  def install
    # TODO: Add build instructions
    # Common patterns:
    #
    # For autotools:
    # system "./configure", "--prefix={{{{prefix}}}}"
    # system "make", "install"
    #
    # For CMake:
    # system "cmake", "-S", ".", "-B", "build", *std_cmake_args
    # system "cmake", "--build", "build"
    # system "cmake", "--install", "build"
    #
    # For Cargo (Rust):
    # system "cargo", "install", *std_cargo_args
    #
    # For simple binaries:
    # bin.install "binary_name"
  end

  test do
    # TODO: Add test
    # Example:
    # system "{{{{bin}}}}/formula_name", "--version"
  end
end
"#,
        class_name = class_name,
        desc = desc,
        homepage = homepage,
        url = url,
        sha256 = sha256,
        license = license,
    )
}

fn generate_cask(
    token: &str,
    version: &str,
    url: &str,
    sha256: &str,
    app_name: &str,
    homepage: Option<&str>,
) -> String {
    // Use provided homepage or clear placeholder
    let homepage = homepage.unwrap_or("REPLACE: https://example.com");

    // Determine likely artifact type from URL
    let artifact_type = if url.ends_with(".dmg") {
        "app"
    } else if url.ends_with(".pkg") {
        "pkg"
    } else {
        "app"
    };

    format!(
        r#"cask "{token}" do
  version "{version}"
  sha256 "{sha256}"

  url "{url}"
  name "{app_name}"
  desc "REPLACE: Add a one-line description of this application"
  homepage "{homepage}"

  # TODO: Specify the correct artifact
  # For .app bundles:
  # app "AppName.app"
  #
  # For .pkg installers:
  # pkg "AppName.pkg"
  #
  # For binaries:
  # binary "{{{{staged_path}}}}/binary_name"
  {artifact_type} "{app_name}.app"

  # Optional: Add uninstall stanza
  # uninstall quit: "com.example.app",
  #           delete: "/Applications/AppName.app"

  # Optional: Add zap stanza for complete removal
  # zap trash: [
  #   "~/Library/Application Support/AppName",
  #   "~/Library/Preferences/com.example.plist",
  # ]
end
"#,
        token = token,
        version = version,
        sha256 = sha256,
        url = url,
        app_name = app_name,
        homepage = homepage,
        artifact_type = artifact_type,
    )
}

fn to_class_name(name: &str) -> String {
    // Convert kebab-case or snake_case to PascalCase
    name.split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_name_from_github_url() {
        let url = "https://github.com/jqlang/jq/archive/refs/tags/jq-1.7.1.tar.gz";
        assert_eq!(extract_name_from_url(url).unwrap(), "jq");
    }

    #[test]
    fn test_extract_version_from_url() {
        let url = "https://github.com/jqlang/jq/archive/refs/tags/v1.7.1.tar.gz";
        assert_eq!(extract_version_from_url(url), Some("1.7.1".to_string()));
    }

    #[test]
    fn test_to_class_name() {
        assert_eq!(to_class_name("hello-world"), "HelloWorld");
        assert_eq!(to_class_name("my_package"), "MyPackage");
        assert_eq!(to_class_name("simple"), "Simple");
    }
}
