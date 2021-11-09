mod description;
mod json;
mod traits;

use anyhow::Result;
use description::*;
use serde_yaml::from_reader as yaml_from_reader;
use std::{
    borrow::Cow,
    env::current_dir,
    fs::File,
    path::{Component, Path},
};
use walkdir::{DirEntry, WalkDir};

fn main() -> Result<()> {
    let api_spec_dir = current_dir()?.join("..").join("api-specs");

    for read_result in WalkDir::new(&api_spec_dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = read_result?;
        let namespace: Vec<_> =
            if let Some(dir_path) = entry.path().strip_prefix(&api_spec_dir)?.parent() {
                dir_path
                    .components()
                    .map(|component| {
                        if let Component::Normal(component) = component {
                            component.to_string_lossy()
                        } else {
                            unreachable!()
                        }
                    })
                    .collect()
            } else {
                vec![]
            };
        let file_name = entry.file_name().to_string_lossy();
        if let Some(base_name) = file_name.strip_suffix(".yml") {
            print_api_description(base_name, &namespace, &entry.path())?;
        } else if let Some(base_name) = file_name.strip_suffix(".yaml") {
            print_api_description(base_name, &namespace, &entry.path())?;
        }
    }

    return Ok(());

    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with("."))
            .unwrap_or(false)
    }
}

fn print_api_description(
    base_name: &str,
    namespace: &[Cow<str>],
    api_spec_path: &Path,
) -> Result<()> {
    println!("****** api_spec_path:  {:?}", api_spec_path);
    let mut file = File::open(api_spec_path)?;
    let api_detailed_spec: ApiDetailedDescription = yaml_from_reader(&mut file)?;
    println!(
        "Name: {}\nNamespace: {:?}\nSpec: {:#?}",
        base_name, namespace, api_detailed_spec
    );
    Ok(())
}
