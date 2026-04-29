//! Info command

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_index::{Database, IndexSync};
use stout_state::{Config, InstalledPackages, Paths};

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
    }

    // Try cask (unless --formula specified)
    if !args.formula {
        if let Some(cask_info) = db.get_cask(&args.name)? {
            return show_cask_info(&args.name, &cask_info, &sync, &paths, &db).await;
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
    // Fetch full formula data
    let formula = sync
        .fetch_formula_cached(name, info.json_hash.as_deref())
        .await
        .context("Failed to fetch formula details")?;

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

        for (i, dep) in runtime_deps.iter().enumerate() {
            let is_last =
                i == runtime_deps.len() - 1 && build_deps.is_empty() && other_deps.is_empty();
            let prefix = if is_last { "└──" } else { "├──" };
            let marker = if installed_pkgs.is_installed(&dep.formula) {
                style("✓").green()
            } else {
                style("○").dim()
            };
            println!(
                "  {} {} {} {}",
                prefix,
                marker,
                dep.formula,
                style("(runtime)").dim()
            );
        }
        for (i, dep) in build_deps.iter().enumerate() {
            let is_last = i == build_deps.len() - 1 && other_deps.is_empty();
            let prefix = if is_last { "└──" } else { "├──" };
            let marker = if installed_pkgs.is_installed(&dep.formula) {
                style("✓").green()
            } else {
                style("○").dim()
            };
            println!(
                "  {} {} {} {}",
                prefix,
                marker,
                dep.formula,
                style("(build)").dim()
            );
        }
        for (i, dep) in other_deps.iter().enumerate() {
            let is_last = i == other_deps.len() - 1;
            let prefix = if is_last { "└──" } else { "├──" };
            let marker = if installed_pkgs.is_installed(&dep.formula) {
                style("✓").green()
            } else {
                style("○").dim()
            };
            println!(
                "  {} {} {} {}",
                prefix,
                marker,
                dep.formula,
                style(format!("({})", dep.dep_type.as_str())).dim()
            );
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

    // Install status
    println!();
    let installed = paths.is_installed(&formula.name, &formula.version);
    if installed {
        println!(
            "{}: {} {}",
            style("Installed").green(),
            formula.name,
            formula.version
        );
    } else {
        println!("{}: {}", style("Installed").dim(), style("No").dim());
    }

    println!();
    Ok(())
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
        let mut all_dependents: Vec<stout_index::Dependent> = Vec::new();
        for dep in formula_deps {
            if let Ok(deps) = db.get_dependents(dep, &[]) {
                all_dependents.extend(deps);
            }
        }
        // Deduplicate
        all_dependents.sort_by(|a, b| a.formula.cmp(&b.formula));
        all_dependents.dedup_by(|a, b| a.formula == b.formula);

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
