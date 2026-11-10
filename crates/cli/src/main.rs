mod catalogue;

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use catalogue::{LICENSE, PRESENTATION, RECIPES, Recipe};

const USAGE: &str = "GPUI Design source recipes

Usage:
  gpui-design list
  gpui-design add <recipe>... --to <module-directory>

Example:
  gpui-design add data-table avatar --to src/ui

Adds recipe dependencies, presentation tokens, module declarations and license.
Existing installed recipe and token edits are kept. Modified generated module
declarations or conflicting new files stop the command before any files change.
The consumer supplies its gpui and gpui-design Cargo dependencies.";

const ROOT_MODULE: &str = "//! Application-owned GPUI Design presentation sources.

mod presentation;
pub(crate) mod recipes;

pub(crate) use presentation::{Appearance, Palette};
";
const MANIFEST: &str = "recipes/.gpui-design";

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    List,
    Add { names: Vec<String>, to: PathBuf },
}

fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Command> {
    let mut arguments = arguments.into_iter();
    match arguments.next().as_deref().and_then(|value| value.to_str()) {
        None | Some("--help" | "-h") => Ok(Command::Help),
        Some("list") => {
            if arguments.next().is_some() {
                bail!("list does not accept arguments");
            }
            Ok(Command::List)
        }
        Some("add") => {
            let mut names = Vec::new();
            let mut to = None;
            while let Some(argument) = arguments.next() {
                if argument == "--to" {
                    if to.is_some() {
                        bail!("supply --to once");
                    }
                    to = Some(PathBuf::from(
                        arguments.next().context("--to needs a directory")?,
                    ));
                } else {
                    let name = argument
                        .into_string()
                        .map_err(|_| anyhow::anyhow!("recipe names must be UTF-8"))?;
                    if name.starts_with('-') {
                        bail!("unknown option {name}");
                    }
                    names.push(name);
                }
            }
            if names.is_empty() {
                bail!("add needs at least one recipe; use list to see available names");
            }
            Ok(Command::Add {
                names,
                to: to.context("add requires --to <module-directory>")?,
            })
        }
        Some(command) => bail!("unknown command {command}"),
    }
}

fn recipe(name: &str) -> Result<&'static Recipe> {
    RECIPES
        .iter()
        .find(|recipe| recipe.name == name)
        .with_context(|| format!("unknown recipe {name}; use list to see available names"))
}

fn resolve(names: impl IntoIterator<Item = String>) -> Result<BTreeSet<String>> {
    let mut pending: Vec<_> = names.into_iter().collect();
    let mut selected = BTreeSet::new();
    while let Some(name) = pending.pop() {
        let item = recipe(&name)?;
        if selected.insert(name) {
            pending.extend(item.dependencies.iter().map(|name| (*name).to_owned()));
        }
    }
    Ok(selected)
}

fn module_source(names: &BTreeSet<String>) -> Result<String> {
    let mut source = "//! Recipe modules managed by gpui-design add; edit the recipe files to customize them.\n\n".to_owned();
    for name in names {
        let recipe = recipe(name)?;
        source.push_str(&format!("mod {};\n", recipe.module));
    }
    source.push('\n');
    for name in names {
        let recipe = recipe(name)?;
        let exports = if recipe.exports.len() == 1 {
            recipe.exports[0].to_owned()
        } else {
            format!("{{{}}}", recipe.exports.join(", "))
        };
        source.push_str(&format!("pub(crate) use {}::{exports};\n", recipe.module));
    }
    Ok(source)
}

fn manifest_source(names: &BTreeSet<String>) -> String {
    names.iter().map(|name| format!("{name}\n")).collect()
}

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn source_files(names: &BTreeSet<String>) -> Result<BTreeMap<PathBuf, &'static str>> {
    let mut files = BTreeMap::from([(PathBuf::from("presentation.rs"), PRESENTATION)]);
    for name in names {
        let recipe = recipe(name)?;
        files.insert(
            PathBuf::from(format!("recipes/{}.rs", recipe.module)),
            recipe.source,
        );
        for (path, source) in recipe.assets {
            files.insert(PathBuf::from(path), source);
        }
    }
    Ok(files)
}

#[derive(Default)]
struct Installed {
    written: Vec<PathBuf>,
    kept: Vec<PathBuf>,
}

fn add(names: Vec<String>, to: &Path) -> Result<Installed> {
    // Resolve and preflight the complete addition before writing any source.
    let previous_manifest = read(&to.join(MANIFEST))?;
    let previous: BTreeSet<String> = if let Some(contents) = &previous_manifest {
        std::str::from_utf8(contents)
            .context("installed recipe metadata must be UTF-8")?
            .lines()
            .map(str::to_owned)
            .collect()
    } else {
        BTreeSet::new()
    };
    if let Some(contents) = &previous_manifest {
        if *contents != manifest_source(&previous).as_bytes() {
            bail!(
                "{} was modified; keep its recipe names in generated order",
                to.join(MANIFEST).display()
            );
        }
    }
    let selected = resolve(previous.iter().cloned().chain(names))?;
    let previous_files = source_files(&previous)?;
    let mut pending = BTreeMap::new();
    let mut result = Installed::default();
    for (path, source) in source_files(&selected)? {
        match read(&to.join(&path))? {
            Some(_) if previous_manifest.is_some() && previous_files.contains_key(&path) => {
                result.kept.push(path);
            }
            Some(contents) if contents == source.as_bytes() => result.kept.push(path),
            Some(_) => bail!(
                "{} already exists with different contents; choose a dedicated module directory",
                to.join(&path).display()
            ),
            None => {
                pending.insert(path, source.to_owned());
            }
        }
    }
    for (path, old, new) in [
        ("mod.rs", ROOT_MODULE.to_owned(), ROOT_MODULE.to_owned()),
        (
            "recipes/mod.rs",
            module_source(&previous)?,
            module_source(&selected)?,
        ),
        (
            MANIFEST,
            manifest_source(&previous),
            manifest_source(&selected),
        ),
        ("LICENSE-APACHE", LICENSE.to_owned(), LICENSE.to_owned()),
    ] {
        match read(&to.join(path))? {
            Some(contents) if contents == new.as_bytes() => result.kept.push(PathBuf::from(path)),
            Some(contents) if contents == old.as_bytes() => {
                pending.insert(PathBuf::from(path), new);
            }
            Some(_) => bail!(
                "{} was modified; preserve your changes and add the module declarations manually or choose another --to directory",
                to.join(path).display()
            ),
            None => {
                pending.insert(PathBuf::from(path), new);
            }
        }
    }
    for (path, source) in pending {
        let destination = to.join(&path);
        fs::create_dir_all(destination.parent().unwrap())
            .with_context(|| format!("create directory for {}", destination.display()))?;
        fs::write(&destination, source)
            .with_context(|| format!("write {}", destination.display()))?;
        result.written.push(path);
    }
    Ok(result)
}

fn run() -> Result<()> {
    match parse(std::env::args_os().skip(1))? {
        Command::Help => println!("{USAGE}"),
        Command::List => {
            for recipe in RECIPES {
                if recipe.dependencies.is_empty() {
                    println!("{}", recipe.name);
                } else {
                    println!(
                        "{} (includes {})",
                        recipe.name,
                        recipe.dependencies.join(", ")
                    );
                }
            }
        }
        Command::Add { names, to } => {
            let result = add(names, &to)?;
            for path in result.written {
                println!("added {}", to.join(path).display());
            }
            for path in result.kept {
                println!("kept {}", to.join(path).display());
            }
            println!("Declare this directory as a module in its parent (for example, mod ui;).");
            println!(
                "Import its recipes and Appearance::Settings.palette() or Appearance::Compact.palette()."
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}\n\n{USAGE}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().canonicalize().unwrap();
            let path = root.join(format!(
                "gpui-design-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let root = std::env::temp_dir().canonicalize().unwrap();
            let resolved = self.0.canonicalize().unwrap();
            assert!(resolved.starts_with(root) && resolved == self.0);
            fs::remove_dir_all(resolved).unwrap();
        }
    }

    #[test]
    fn arguments_require_named_recipes_and_one_destination() {
        let arguments = |items: &[&str]| items.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(parse(arguments(&["list"])).unwrap(), Command::List);
        assert_eq!(
            parse(arguments(&["add", "card", "badge", "--to", "src/ui"])).unwrap(),
            Command::Add {
                names: vec!["card".into(), "badge".into()],
                to: "src/ui".into()
            }
        );
        for invalid in [
            vec!["list", "card"],
            vec!["add", "card"],
            vec!["add", "--to", "src/ui"],
            vec!["add", "card", "--to"],
            vec!["add", "card", "--to", "a", "--to", "b"],
        ] {
            assert!(parse(arguments(&invalid)).is_err());
        }
    }

    #[test]
    fn addition_includes_transitive_dependencies_and_exact_original_sources() {
        let directory = Directory::new();
        add(vec!["data-table".into(), "avatar".into()], &directory.0).unwrap();
        assert_eq!(
            fs::read_to_string(directory.0.join(MANIFEST)).unwrap(),
            "avatar\nbutton-group\ndata-table\nempty\ninput-group\npagination\n"
        );
        assert_eq!(
            fs::read_to_string(directory.0.join("presentation.rs")).unwrap(),
            PRESENTATION
        );
        assert_eq!(
            fs::read_to_string(directory.0.join("recipes/data_table.rs")).unwrap(),
            recipe("data-table").unwrap().source
        );
        assert_eq!(
            fs::read_to_string(directory.0.join("LICENSE-APACHE")).unwrap(),
            LICENSE
        );
        assert!(directory.0.join("assets/preset-avatar.svg").is_file());
        let modules = fs::read_to_string(directory.0.join("recipes/mod.rs")).unwrap();
        assert!(modules.contains("mod button_group;"));
        assert!(modules.contains("DataTableState"));
    }

    #[test]
    fn repeat_additions_keep_consumer_source_edits_and_merge_new_modules() {
        let directory = Directory::new();
        add(vec!["card".into()], &directory.0).unwrap();
        fs::write(directory.0.join("presentation.rs"), "// my palette\n").unwrap();
        fs::write(directory.0.join("recipes/card.rs"), "// my card\n").unwrap();
        add(vec!["message".into()], &directory.0).unwrap();
        let repeated = add(vec!["card".into()], &directory.0).unwrap();
        assert!(repeated.written.is_empty());
        assert_eq!(
            fs::read_to_string(directory.0.join("presentation.rs")).unwrap(),
            "// my palette\n"
        );
        assert_eq!(
            fs::read_to_string(directory.0.join("recipes/card.rs")).unwrap(),
            "// my card\n"
        );
        assert_eq!(
            fs::read_to_string(directory.0.join(MANIFEST)).unwrap(),
            "card\nmessage\ntypography\n"
        );
    }

    #[test]
    fn conflicts_fail_before_writing_any_additional_files() {
        let directory = Directory::new();
        add(vec!["card".into()], &directory.0).unwrap();
        fs::write(directory.0.join("recipes/mod.rs"), "// custom module\n").unwrap();
        assert!(add(vec!["data-table".into()], &directory.0).is_err());
        assert!(!directory.0.join("recipes/data_table.rs").exists());
        assert_eq!(
            fs::read_to_string(directory.0.join("recipes/mod.rs")).unwrap(),
            "// custom module\n"
        );

        let fresh = Directory::new();
        fs::write(fresh.0.join("presentation.rs"), "// preexisting tokens\n").unwrap();
        assert!(add(vec!["card".into()], &fresh.0).is_err());
        assert!(!fresh.0.join("mod.rs").exists());
        assert!(!fresh.0.join("recipes/card.rs").exists());
    }

    #[test]
    fn unknown_recipes_do_not_create_an_installation() {
        let directory = Directory::new();
        assert!(add(vec!["unknown".into()], &directory.0).is_err());
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
        for item in RECIPES {
            resolve([item.name.to_owned()]).unwrap();
        }
    }
}
