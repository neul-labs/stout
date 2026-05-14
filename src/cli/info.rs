//! Info command

use std::collections::HashSet;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_index::{Database, Formula, IndexSync};
use stout_state::{Config, InstalledPackages, Paths, TapManager};

#[derive(ClapArgs)]
pub struct Args {
    /// Formula or cask name
    pub name: String,

    /// Show cask info (if both formula and cask exist)
    #[arg(long)]
    pub cask: bool,

    /// Show formula info (if both formula and cask exist)
    #[arg(long, conflicts_with = "cask")]
    pub formula: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;

    // Open the database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    // Try formula first (unless --cask specified)
    if !args.cask {
        if let Some(info) = db.get_formula(&args.name)? {
            return show_formula_info(&args.name, &info, &sync, &paths, &db).await;
        }

        // Formula not in index DB, try fetching directly via Homebrew
        if let Ok(formula) = sync.fetch_formula(&args.name).await {
            let formula_info = stout_index::FormulaInfo {
                name: formula.name.clone(),
                version: formula.version.clone(),
                revision: formula.revision,
                desc: formula.desc.clone(),
                homepage: formula.homepage.clone(),
                license: formula.license.clone(),
                tap: formula.tap.clone(),
                deprecated: false,
                disabled: false,
                has_bottle: !formula.bottles.is_empty(),
                json_hash: None,
            };
            return show_formula_info(&args.name, &formula_info, &sync, &paths, &db).await;
        }
    }

    // Try cask (unless --formula specified)
    if !args.formula {
        if let Some(cask_info) = db.get_cask(&args.name)? {
            return show_cask_info(&args.name, &cask_info, &sync, &paths, &db).await;
        }

        // Cask not in index DB, try fetching directly via Homebrew
        if let Ok(cask) = sync.fetch_cask(&args.name).await {
            let cask_info = stout_index::CaskInfo {
                token: cask.token.clone(),
                name: cask.name.first().cloned(),
                version: cask.version.clone(),
                desc: cask.desc.clone(),
                homepage: cask.homepage.clone(),
                tap: cask.tap.clone(),
                deprecated: cask.deprecated,
                disabled: cask.disabled,
                artifact_type: Some(cask.primary_artifact_type().to_string()),
                json_hash: None,
            };
            return show_cask_info(&args.name, &cask_info, &sync, &paths, &db).await;
        }
    }

    // Not found in index — search configured taps
    if !args.cask {
        let tap_manager = TapManager::load(&paths)?;
        for tap in tap_manager.list() {
            let tap_parts: Vec<&str> = tap.name.split('/').collect();
            if tap_parts.len() == 2 {
                let tap_user = tap_parts[0];
                let tap_repo = if tap_parts[1].starts_with("homebrew-") {
                    &tap_parts[1]["homebrew-".len()..]
                } else {
                    tap_parts[1]
                };
                let full_name = format!("{}/{}/{}", tap_user, tap_repo, args.name);
                if let Ok(formula) = sync.fetch_formula(&full_name).await {
                    return display_formula_info(&formula, &paths, &db);
                }
            }
        }
    }

    // Not found - show suggestions
    let formula_suggestions = db.find_similar(&args.name, 3)?;
    let cask_suggestions = db.find_similar_casks(&args.name, 3)?;

    eprintln!(
        "\n{} '{}' not found",
        style("error:").red().bold(),
        args.name
    );

    if !formula_suggestions.is_empty() {
        eprintln!("\n{} (formulas):", style("Did you mean?").yellow());
        for s in &formula_suggestions {
            eprintln!("  {} {}", style("•").dim(), s);
        }
    }

    if !cask_suggestions.is_empty() {
        eprintln!("\n{} (casks):", style("Did you mean?").yellow());
        for s in &cask_suggestions {
            eprintln!("  {} {}", style("•").dim(), s);
        }
    }

    eprintln!(
        "\n{}",
        style("Run 'stout search <query>' to find packages").dim()
    );
    std::process::exit(1);
}

async fn show_formula_info(
    name: &str,
    info: &stout_index::FormulaInfo,
    sync: &IndexSync,
    paths: &Paths,
    db: &Database,
) -> Result<()> {
    let formula = sync
        .fetch_formula_cached(name, info.json_hash.as_deref())
        .await
        .context("Failed to fetch formula details")?;
    display_formula_info(&formula, paths, db)
}

fn display_formula_info(formula: &Formula, paths: &Paths, db: &Database) -> Result<()> {
    // Display
    println!();
    println!(
        "{} {} {}",
        style(&formula.name).green().bold(),
        style(&formula.version).cyan(),
        style("(formula)").dim()
    );

    if let Some(desc) = &formula.desc {
        println!("{}", style(desc).dim());
    }

    println!();

    // Metadata
    if let Some(homepage) = &formula.homepage {
        println!("{:12} {}", style("Homepage:").dim(), homepage);
    }
    if let Some(license) = &formula.license {
        println!("{:12} {}", style("License:").dim(), license);
    }
    println!("{:12} {}", style("Tap:").dim(), formula.tap);

    // Dependencies
    if !formula.dependencies.runtime.is_empty() || !formula.dependencies.build.is_empty() {
        println!("\n{}:", style("Dependencies").cyan());

        let deps = &formula.dependencies;

        for (i, dep) in deps.runtime.iter().enumerate() {
            let is_last = i == deps.runtime.len() - 1 && deps.build.is_empty();
            let prefix = if is_last { "└──" } else { "├──" };
            println!("  {} {} {}", prefix, dep, style("(runtime)").dim());
        }

        for (i, dep) in deps.build.iter().enumerate() {
            let is_last = i == deps.build.len() - 1;
            let prefix = if is_last { "└──" } else { "├──" };
            println!("  {} {} {}", prefix, dep, style("(build)").dim());
        }
    }

    // Dependents (packages that depend on this formula)
    let dependents = db.get_dependents(&formula.name, &[])?;
    if !dependents.is_empty() {
        let installed_pkgs = InstalledPackages::load(paths)?;
        let installed_count = dependents
            .iter()
            .filter(|d| installed_pkgs.is_installed(&d.formula))
            .count();

        println!("\n{}:", style("Dependents").cyan());

        // Group by type: runtime/recommended first, then build, then others
        let mut runtime_deps: Vec<&stout_index::Dependent> = Vec::new();
        let mut build_deps: Vec<&stout_index::Dependent> = Vec::new();
        let mut other_deps: Vec<&stout_index::Dependent> = Vec::new();

        for dep in &dependents {
            match dep.dep_type {
                stout_index::DependencyType::Runtime | stout_index::DependencyType::Recommended => {
                    runtime_deps.push(dep)
                }
                stout_index::DependencyType::Build => build_deps.push(dep),
                _ => other_deps.push(dep),
            }
        }

        let total = dependents.len();
        let mut idx = 0;
        for dep in &runtime_deps {
            print_dependent(dep, idx == total - 1, &installed_pkgs, "(runtime)");
            idx += 1;
        }
        for dep in &build_deps {
            print_dependent(dep, idx == total - 1, &installed_pkgs, "(build)");
            idx += 1;
        }
        for dep in &other_deps {
            let label = format!("({})", dep.dep_type.as_str());
            print_dependent(dep, idx == total - 1, &installed_pkgs, &label);
            idx += 1;
        }

        println!(
            "  {} {}/{} installed",
            style("→").dim(),
            installed_count,
            dependents.len()
        );
    }

    // Bottles
    if !formula.bottles.is_empty() {
        println!("\n{}:", style("Bottles").cyan());
        let platforms: Vec<_> = formula.bottles.keys().collect();
        let mut line = String::from("  ");
        for platform in platforms {
            line.push_str(&format!("{} {}  ", style("✓").green(), platform));
        }
        println!("{}", line);
    }

    // Caveats
    if let Some(caveats) = &formula.caveats {
        println!("\n{}:", style("Caveats").yellow());
        for line in caveats.lines() {
            println!("  {}", line);
        }
    }

    // Install status — check both Cellar filesystem and InstalledPackages state
    // (tap formulas are recorded under the tap-qualified name, e.g. "user/tap/formula")
    println!();
    let installed_pkgs = InstalledPackages::load(paths).unwrap_or_default();
    let tap_qualified = format!("{}/{}", formula.tap, formula.name);
    let installed = paths.is_installed(&formula.name, &formula.version)
        || installed_pkgs.is_installed(&formula.name)
        || installed_pkgs.is_installed(&tap_qualified);
    if installed {
        let version = installed_pkgs
            .get(&formula.name)
            .or_else(|| installed_pkgs.get(&tap_qualified))
            .map(|p| p.version.as_str())
            .unwrap_or(&formula.version);
        println!(
            "{}: {} {}",
            style("Installed").green(),
            formula.name,
            version
        );
    } else {
        println!("{}: {}", style("Installed").dim(), style("No").dim());
    }

    println!();
    Ok(())
}

fn print_dependent(
    dep: &stout_index::Dependent,
    is_last: bool,
    installed: &InstalledPackages,
    label: &str,
) {
    let prefix = if is_last { "└──" } else { "├──" };
    let marker = if installed.is_installed(&dep.formula) {
        style("✓").green()
    } else {
        style("○").dim()
    };
    println!(
        "  {} {} {} {}",
        prefix,
        marker,
        dep.formula,
        style(label).dim()
    );
}

async fn show_cask_info(
    token: &str,
    info: &stout_index::CaskInfo,
    sync: &IndexSync,
    paths: &Paths,
    db: &Database,
) -> Result<()> {
    // Fetch full cask data
    let cask = sync
        .fetch_cask_cached(token, info.json_hash.as_deref())
        .await
        .context("Failed to fetch cask details")?;

    // Display
    println!();
    println!(
        "{} {} {}",
        style(&cask.token).magenta().bold(),
        style(&cask.version).cyan(),
        style("(cask)").dim()
    );

    // Display name if different from token
    if let Some(name) = cask.name.first() {
        if name != &cask.token {
            println!("{}", style(name).bold());
        }
    }

    if let Some(desc) = &cask.desc {
        println!("{}", style(desc).dim());
    }

    println!();

    // Metadata
    if let Some(homepage) = &cask.homepage {
        println!("{:12} {}", style("Homepage:").dim(), homepage);
    }
    println!(
        "{:12} {}",
        style("Tap:").dim(),
        if cask.tap.is_empty() {
            "homebrew/cask"
        } else {
            &cask.tap
        }
    );
    println!(
        "{:12} {}",
        style("Type:").dim(),
        cask.primary_artifact_type()
    );

    // Apps
    let apps = cask.apps();
    if !apps.is_empty() {
        println!("\n{}:", style("Artifacts").cyan());
        for app in apps {
            println!("  {} {}", style("•").green(), app);
        }
    }

    // Dependencies
    if !cask.depends_on.formula.is_empty() || !cask.depends_on.cask.is_empty() {
        println!("\n{}:", style("Dependencies").cyan());

        for dep in &cask.depends_on.formula {
            println!("  ├── {} {}", dep, style("(formula)").dim());
        }
        for (i, dep) in cask.depends_on.cask.iter().enumerate() {
            let is_last = i == cask.depends_on.cask.len() - 1;
            let prefix = if is_last { "└──" } else { "├──" };
            println!("  {} {} {}", prefix, dep, style("(cask)").dim());
        }
    }

    // Dependents (formulas that this cask's formula dependencies depend on)
    let formula_deps = &cask.depends_on.formula;
    if !formula_deps.is_empty() {
        let mut seen = HashSet::new();
        let mut all_dependents: Vec<stout_index::Dependent> = Vec::new();
        for dep in formula_deps {
            if let Ok(deps) = db.get_dependents(dep, &[]) {
                for d in deps {
                    if seen.insert(d.formula.clone()) {
                        all_dependents.push(d);
                    }
                }
            }
        }

        if !all_dependents.is_empty() {
            let cask_dependents = db.get_cask_dependents(token)?;
            println!("\n{}:", style("Required by").cyan());
            for dep in &all_dependents {
                let marker = style("•").dim();
                println!(
                    "  {} {} {}",
                    marker,
                    dep.formula,
                    style(format!("({})", dep.dep_type.as_str())).dim()
                );
            }
            if !cask_dependents.is_empty() {
                for cask_dep in &cask_dependents {
                    println!(
                        "  {} {} {}",
                        style("•").green(),
                        cask_dep,
                        style("(cask)").dim()
                    );
                }
            }
        }
    } else {
        // Cask has no formula deps, but may have cask dependents
        let cask_dependents = db.get_cask_dependents(token)?;
        if !cask_dependents.is_empty() {
            println!("\n{}:", style("Required by").cyan());
            for cask_dep in &cask_dependents {
                println!(
                    "  {} {} {}",
                    style("•").green(),
                    cask_dep,
                    style("(cask)").dim()
                );
            }
        }
    }

    // Download URL
    if let Some(url) = cask.download_url() {
        println!("\n{:12} {}", style("URL:").dim(), url);
    }

    // Caveats
    if let Some(caveats) = &cask.caveats {
        println!("\n{}:", style("Caveats").yellow());
        for line in caveats.lines() {
            println!("  {}", line);
        }
    }

    // Install status
    println!();
    let cask_state_path = paths.stout_dir.join("casks.json");
    if let Ok(installed_casks) = stout_cask::InstalledCasks::load(&cask_state_path) {
        if let Some(inst) = installed_casks.get(token) {
            println!(
                "{}: {} {} ({})",
                style("Installed").green().bold(),
                style("Yes").green(),
                style(&inst.version).dim(),
                inst.artifact_path.display()
            );
        } else {
            println!("{}: {}", style("Installed").dim(), style("No").dim());
        }
    } else {
        println!("{}: {}", style("Installed").dim(), style("No").dim());
    }

    println!();
    Ok(())
}
