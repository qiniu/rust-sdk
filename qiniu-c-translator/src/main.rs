use clang::{Clang, Index};
use clap::{App, Arg, SubCommand};

mod ast;
mod dump_entity;
use ast::dump_ast;
use dump_entity::dump_entity;

fn main() {
    let matches = App::new("Qiniu C Translator")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS").split(':').last().unwrap())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("header-file")
                .long("header-file")
                .required(true)
                .value_name("FILE")
                .help("To generate bindings")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("dump-entity")
                .about("Show clang entities, only for debug")
                .arg(
                    Arg::with_name("pretty-print")
                        .long("pretty")
                        .help("Pretty-printed output"),
                ),
        )
        .subcommand(
            SubCommand::with_name("dump-ast")
                .about("Show clang ast, only for debug")
                .arg(
                    Arg::with_name("pretty-print")
                        .long("pretty")
                        .help("Pretty-printed output"),
                ),
        )
        .get_matches();
    let cl = Clang::new().unwrap();
    let idx = Index::new(&cl, true, false);
    let tu = idx.parser(matches.value_of_os("header-file").unwrap()).parse().unwrap();
    let entity = tu.get_entity();
    match matches.subcommand() {
        ("dump-entity", args) => dump_entity(
            &entity,
            args.map(|args| args.is_present("pretty-print")).unwrap_or(false),
        ),
        ("dump-ast", args) => {
            dump_ast(
                &entity,
                args.map(|args| args.is_present("pretty-print")).unwrap_or(false),
            );
        }
        ("", _) => {}
        (subcommand, _) => panic!("Unrecognized subcommand: {}", subcommand),
    }
}
