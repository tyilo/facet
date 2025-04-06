use minijinja::Environment;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    // Check if current directory has a Cargo.toml with [workspace]
    let cargo_toml_path = std::env::current_dir().unwrap().join("Cargo.toml");
    let cargo_toml_content =
        std::fs::read_to_string(cargo_toml_path).expect("Failed to read Cargo.toml");
    if !cargo_toml_content.contains("[workspace]") {
        panic!(
            "Cargo.toml does not contain [workspace] (you must run codegen from the workspace root)"
        );
    }

    // Generate tuple implementations
    generate_tuple_impls();

    // Generate README files from templates
    generate_readme_files();
}

fn generate_tuple_impls() {
    // Initialize minijinja environment
    let mut env = Environment::empty();
    env.add_function("range", minijinja::functions::range);

    // Load the template from file
    let template_path = "facet-codegen/src/tuples_impls.rs.j2";
    let template_content =
        std::fs::read_to_string(template_path).expect("Failed to read template file");

    // Add the template to the environment
    env.add_template("tuples_impls", &template_content)
        .expect("Failed to add template");

    // Get the template
    let template = env
        .get_template("tuples_impls")
        .expect("Failed to get template");

    // Render the template with context
    let output = template
        .render(minijinja::context! {
            max_tuple_size => 12
        })
        .expect("Failed to render template");

    // Format the generated code using rustfmt
    let mut fmt = std::process::Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn rustfmt");

    // Write to rustfmt's stdin
    fmt.stdin
        .take()
        .expect("Failed to get stdin")
        .write_all(output.as_bytes())
        .expect("Failed to write to rustfmt stdin");

    // Get formatted output
    let formatted_output = fmt.wait_with_output().expect("Failed to wait for rustfmt");
    if !formatted_output.status.success() {
        eprintln!("rustfmt failed with exit code: {}", formatted_output.status);
        std::process::exit(1);
    }

    let path = "facet-trait/src/impls/tuples_impls.rs";
    std::fs::write(path, formatted_output.stdout).expect("Failed to write file");
}

fn generate_readme_files() {
    // Get all crate directories in the workspace
    let workspace_dir = std::env::current_dir().unwrap();
    let entries = fs::read_dir(&workspace_dir).expect("Failed to read workspace directory");

    // Create a new MiniJinja environment for README templates
    let mut env = Environment::empty();

    // Add template functions
    env.add_function("badges", |crate_name: String| {
        format!(
            "[![experimental](https://img.shields.io/badge/status-highly%20experimental-orange)](https://github.com/fasterthanlime/facet)\n\
             [![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)\n\
             [![crates.io](https://img.shields.io/crates/v/{0}.svg)](https://crates.io/crates/{0})\n\
             [![documentation](https://docs.rs/{0}/badge.svg)](https://docs.rs/{0})\n\
             [![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/{0}.svg)](./LICENSE)",
            crate_name
        )
    });

    env.add_function("footer", || {
        r#"## Funding

Thanks to all individual sponsors, without whom this work could not exist:

<a href="https://ko-fi.com/fasterthanlime">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/ko-fi-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/ko-fi-light.svg" height="40" alt="Ko-fi">
    </picture>
</a> <a href="https://github.com/sponsors/fasterthanlime">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/github-sponsors-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/github-sponsors-light.svg" height="40" alt="GitHub Sponsors">
    </picture>
</a> <a href="https://patreon.com/fasterthanlime">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/patreon-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/patreon-light.svg" height="40" alt="Patreon">
    </picture>
</a>

Thanks to corporate sponsors: Zed Industries for free credits, and Namespace for providing fast GitHub Actions workers.

<a href="https://zed.dev">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/zed-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/zed-light.svg" height="40" alt="Zed">
    </picture>
</a> <a href="https://namespace.so">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/namespace-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/namespace-light.svg" height="40" alt="Namespace">
    </picture>
</a>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/facet-rs/facet/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/facet-rs/facet/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option."#.to_string()
    });

    // Process each crate in the workspace
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        // Skip non-directories and entries starting with '.' or '_'
        if !path.is_dir()
            || path.file_name().is_none_or(|name| {
                let name = name.to_string_lossy();
                name.starts_with('.') || name.starts_with('_')
            })
        {
            continue;
        }

        // Skip target directory and facet-codegen itself
        let dir_name = path.file_name().unwrap().to_string_lossy();
        if dir_name == "target" || dir_name == "facet-codegen" {
            continue;
        }

        // Check if this is a crate directory (has a Cargo.toml)
        let cargo_toml_path = path.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            continue;
        }

        // Check if there's a template directory with a README.md.j2 file
        let template_path = path.join("templates").join("README.md.j2");
        if template_path.exists() {
            process_readme_template(&mut env, &path, &template_path);
        } else {
            // Special case for the main facet crate
            let crate_name = dir_name.to_string();
            if crate_name == "facet" {
                let alternative_template_path = Path::new("facet/templates/README.md.j2");
                if alternative_template_path.exists() {
                    process_readme_template(
                        &mut env,
                        &path,
                        Path::new("facet/templates/README.md.j2"),
                    );
                }
            }
        }
    }

    // Also check for the special case where template is in facet/facet/templates
    let facet_crate_path = workspace_dir.join("facet");
    if facet_crate_path.exists() && facet_crate_path.is_dir() {
        let template_path = facet_crate_path.join("templates").join("README.md.j2");
        if template_path.exists() {
            process_readme_template(&mut env, &facet_crate_path, &template_path);
        }
    }

    // Generate workspace README if template exists
    let workspace_template_path = workspace_dir.join("templates").join("README.md.j2");
    if workspace_template_path.exists() {
        process_readme_template(&mut env, &workspace_dir, &workspace_template_path);
    }
}

fn process_readme_template(env: &mut Environment, crate_path: &Path, template_path: &Path) {
    println!("Processing template: {:?}", template_path);

    // Get crate name from directory name
    let crate_name = crate_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Read the template
    let template_content = fs::read_to_string(template_path)
        .unwrap_or_else(|_| panic!("Failed to read template file: {:?}", template_path));

    // Create a template ID
    let template_id = format!("{}_readme", crate_name);

    // Add template to environment
    env.add_template(
        Box::leak(template_id.clone().into_boxed_str()),
        Box::leak(template_content.into_boxed_str()),
    )
    .unwrap_or_else(|_| panic!("Failed to add template: {}", template_id));

    // Get the template
    let template = env
        .get_template(&template_id)
        .expect("Failed to get template");

    // Render the template with context
    let output = template
        .render(minijinja::context! {
            crate_name => crate_name
        })
        .expect("Failed to render template");

    // Save the rendered content to README.md
    let readme_path = crate_path.join("README.md");
    fs::write(&readme_path, output)
        .unwrap_or_else(|_| panic!("Failed to write README.md to {:?}", readme_path));

    println!("Generated README.md for {}", crate_name);
}
