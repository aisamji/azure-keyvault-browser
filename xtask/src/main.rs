use cargo_metadata::MetadataCommand;
use clap::{Command, command};

fn main() {
    let matches = command!()
        .subcommand(
            Command::new("version")
                .about("Print the versions of the specified crates.")
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("version") {
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("Failed to run cargo metadata");

        for p in metadata.packages {
            // TODO: Restrict to only printing out packages in the list of packages passed in if
            // available.
            println!("{}: {}", p.name, p.version);
            // TODO: Only print out version number if only a single package is provided.
        }
    }
}
