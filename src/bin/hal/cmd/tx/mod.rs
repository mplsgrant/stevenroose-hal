use clap;

use cmd;

mod decode;
mod encode;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("tx", "manipulate transactions")
		.subcommand(decode::subcommand())
		.subcommand(encode::subcommand())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("decode", Some(ref m)) => decode::execute(&m),
		("encode", Some(ref m)) => encode::execute(&m),
		(_, _) => unreachable!("clap prints help"),
	};
}