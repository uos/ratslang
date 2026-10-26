use clap::{Args, Parser, Subcommand};
use ratslang::{
    Definition,
    tooling::{self, Configuration},
};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Ratslang configuration tools
#[derive(Parser)]
#[command(
    name = "ratslang",
    version,
    after_help = "The CLI reads local schemas only. The editor also fetches HTTPS ones."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate syntax, units, constraints, required and deprecated settings
    Check {
        #[command(flatten)]
        layers: Layers,
        /// Fail on warnings as well as errors
        #[arg(long)]
        strict: bool,
    },
    /// Format indentation and assignment spacing, keeping comments
    Fmt {
        /// The configuration file
        file: PathBuf,
        /// Write the result back to FILE
        #[arg(long)]
        write: bool,
    },
    /// Show where a setting's value comes from, layer by layer
    Explain {
        #[command(flatten)]
        layers: Layers,
        /// The setting to explain, such as rat.mood
        setting: String,
    },
    /// Print a starter file with the required and recommended settings of a schema
    Generate {
        /// The schema
        file: PathBuf,
        /// Include every setting that is not deprecated
        #[arg(long)]
        full: bool,
        /// Point the generated # @schema at URL instead of FILE
        #[arg(long, value_name = "URL")]
        schema_url: Option<String>,
    },
    /// Comment out overrides that equal the default
    Minimize(Edit),
    /// Move settings marked @renamed_to, keeping the originals as comments
    Migrate(Edit),
}

#[derive(Args)]
struct Layers {
    /// The configuration file
    file: PathBuf,
    /// Local schema file, instead of the # @schema comment in FILE
    #[arg(long, value_name = "FILE")]
    schema: Option<PathBuf>,
    /// Override a setting, may be repeated
    #[arg(long = "set", value_name = "KEY=VALUE")]
    overrides: Vec<String>,
}

#[derive(Args)]
struct Edit {
    /// The configuration file
    file: PathBuf,
    /// Local schema file, instead of the # @schema comment in FILE
    #[arg(long, value_name = "FILE")]
    schema: Option<PathBuf>,
    /// Write the result back to FILE
    #[arg(long)]
    write: bool,
}

fn main() {
    if let Err(error) = run(Cli::parse().command) {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Check { layers, strict } => {
            let config = load(&layers)?;
            let issues = config.issues()?;
            for issue in &issues {
                eprintln!(
                    "{}: {}",
                    if issue.is_error() { "error" } else { "warning" },
                    issue.message
                );
            }
            anyhow::ensure!(
                !issues.iter().any(|i| strict || i.is_error()),
                "configuration validation failed"
            );
            println!("{}: valid", layers.file.display());
        }
        Command::Fmt { file, write } => {
            let source = std::fs::read_to_string(&file)?;
            output(&file, &source, &tooling::format_document(&source)?, write)?;
        }
        Command::Explain { layers, setting } => {
            let config = load(&layers)?;
            let explanation = config
                .explain(&setting)?
                .ok_or_else(|| anyhow::anyhow!("Unknown setting {setting}"))?;
            println!("{} = {}", explanation.name, explanation.value.to_source()?);
            println!(
                "Winner [{}]: {}",
                config.layer(&explanation.winner),
                origin(&explanation.winner)
            );
            println!("Assignment history:");
            for definition in &explanation.history {
                show_definition(&config, definition)?;
            }
            if !explanation.dependencies.is_empty() {
                println!("Referenced definitions (at assignment time):");
                for definition in &explanation.dependencies {
                    show_definition(&config, definition)?;
                }
            }
        }
        Command::Generate {
            file,
            full,
            schema_url,
        } => {
            let defaults = ratslang::compile_file(&file, None, None)?;
            let schema = match schema_url {
                Some(url) => url,
                None => file.canonicalize()?.display().to_string(),
            };
            anyhow::ensure!(
                !schema.contains(['\n', '\r']),
                "schema reference contains a newline"
            );
            println!("# @schema {schema}");
            print!("{}", tooling::generate(&defaults, full)?);
        }
        Command::Minimize(edit) => {
            let (source, config) = load_source(&edit.file, edit.schema, &[])?;
            output(&edit.file, &source, &config.minimize()?, edit.write)?;
        }
        Command::Migrate(edit) => {
            let (source, config) = load_source(&edit.file, edit.schema, &[])?;
            output(&edit.file, &source, &config.migrate()?, edit.write)?;
        }
    }
    Ok(())
}

fn load(layers: &Layers) -> anyhow::Result<Configuration> {
    Ok(load_source(&layers.file, layers.schema.clone(), &layers.overrides)?.1)
}

/// Loads FILE over its schema, taken from --schema or the # @schema comment.
fn load_source(
    file: &Path,
    schema: Option<PathBuf>,
    overrides: &[String],
) -> anyhow::Result<(String, Configuration)> {
    let source = std::fs::read_to_string(file)?;
    let schema = match schema {
        Some(schema) => Some(schema),
        None => schema_reference(&source)
            .map(|reference| {
                anyhow::ensure!(
                    !reference.contains("://") || reference.starts_with("file://"),
                    "CLI schemas must be local. Download {reference} and pass --schema FILE."
                );
                let path = PathBuf::from(reference.strip_prefix("file://").unwrap_or(&reference));
                Ok(if path.is_absolute() {
                    path
                } else {
                    file.parent().unwrap_or(Path::new(".")).join(path)
                })
            })
            .transpose()?,
    };
    let config = Configuration::load(file, schema.as_deref(), overrides)?;
    Ok((source, config))
}

fn schema_reference(source: &str) -> Option<String> {
    let mut found = None;
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(comment) = line.strip_prefix('#') else {
            break;
        };
        if let Some(rest) = comment.trim_start().strip_prefix("@schema") {
            if rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == '=') {
                let rest = rest.trim();
                let rest = rest.strip_prefix('=').unwrap_or(rest).trim();
                if !rest.is_empty() {
                    found = Some(rest.trim_matches('"').to_owned());
                }
            }
        }
    }
    found
}

fn origin(definition: &Definition) -> String {
    match &definition.location {
        Some(location) => format!(
            "{}:{}:{}",
            location
                .file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or("<command line>".into()),
            location.line,
            location.column
        ),
        None => "<initial state>".into(),
    }
}

fn show_definition(config: &Configuration, definition: &Definition) -> anyhow::Result<()> {
    println!(
        "  [{}] {}: {} = {} -> {}",
        config.layer(definition),
        origin(definition),
        definition.name,
        definition.expression.to_source()?,
        definition.value.to_source()?
    );
    for include in &definition.included_from {
        println!(
            "    included from {}:{}",
            include
                .file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or("<layers>".into()),
            include.line
        );
    }
    Ok(())
}

fn output(path: &Path, original: &str, edited: &str, write: bool) -> anyhow::Result<()> {
    if !write {
        print!("{edited}");
        return Ok(());
    }
    anyhow::ensure!(
        std::fs::read_to_string(path)? == original,
        "file changed during editing; refusing to overwrite it"
    );
    let path = path.canonicalize()?;
    let temporary = path.with_file_name(format!(
        ".{}.ratslang-{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    let result = (|| -> anyhow::Result<()> {
        file.set_permissions(std::fs::metadata(&path)?.permissions())?;
        file.write_all(edited.as_bytes())?;
        file.sync_all()?;
        anyhow::ensure!(
            std::fs::read_to_string(&path)? == original,
            "file changed during editing; refusing to overwrite it"
        );
        std::fs::rename(&temporary, &path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
