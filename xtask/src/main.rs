use cargo_metadata::{MetadataCommand, Package};
use clap::{Arg, ArgAction, Command, command};

fn main() {
    let matches = command!()
        .subcommand(
            Command::new("version")
                .about("Print the versions of crates in the workspace.")
                .arg(
                    Arg::new("packages")
                        .short('p')
                        .long("package")
                        .value_name("SPEC")
                        .action(ArgAction::Append)
                        .help("Package(s) for which to get the version information"),
                ),
        )
        .get_matches();

    // TODO: Refactor. First, collect all package name and version pairs - erroring on invalid
    // packages. Second, print them as "package: version" if the crates have different versions or
    // print a single line with just "version" if all crates have the same version.
    // Departure from current strategy, where only the version is printed if there is a single
    // crate.
    if let Some(matches) = matches.subcommand_matches("version") {
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("Failed to run cargo metadata");

        if matches.contains_id("packages") {
            // TODO: Only print out version number if only a single package is provided.
            let packages = matches
                .get_many::<String>("packages")
                .unwrap()
                .collect::<Vec<&String>>();

            if packages.len() == 1 {
                let name = packages.first().unwrap();
                if let Some(spec) = metadata.packages.iter().find(|i| i.name == **name) {
                    println!("{}", spec.version);
                } else {
                    println!("Not found");
                }
            } else {
                for p in matches
                    .get_many::<String>("packages")
                    .unwrap()
                    .collect::<Vec<&String>>()
                {
                    if let Some(spec) = metadata.packages.iter().find(|i| i.name == *p) {
                        println!("{}: {}", p, spec.version);
                    } else {
                        println!("{}: Not found", p);
                    }
                }
            }
        } else {
            // Skip internal packages (i.e. those not meant to be published)
            let packages = metadata
                .packages
                .iter()
                .filter(|p| p.publish != Some(vec![]))
                .collect::<Vec<&Package>>();

            // TODO: Only print out version number if only a single package is provided.
            if packages.len() == 1 {
                println!("{}", packages.first().unwrap().version);
            } else {
                for p in packages {
                    println!("{}: {}", p.name, p.version);
                }
            }
        }
    }
}
