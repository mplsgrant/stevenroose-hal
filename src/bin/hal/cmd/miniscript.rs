
use bitcoin::hex::{DisplayHex, FromHex};
use bitcoin::ScriptBuf;
use clap;
use hal::miniscript::{
	DescriptorInfo, MiniscriptInfo, MiniscriptKeyType, Miniscripts, PolicyInfo, ScriptContexts,
};
use miniscript::miniscript::{BareCtx, Legacy, Miniscript, Segwitv0};
use miniscript::policy::Liftable;
use miniscript::{policy, Descriptor, FromStrKey, MiniscriptKey};

use crate::prelude::*;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("miniscript", "work with miniscript (alias: ms)")
		.alias("ms")
		.subcommand(cmd_descriptor())
		.subcommand(cmd_inspect())
		.subcommand(cmd_parse())
		.subcommand(cmd_policy())
		.subcommand(cmd_compile())
}

pub fn execute<'a>(args: &clap::ArgMatches<'a>) {
	match args.subcommand() {
		("descriptor", Some(ref m)) => exec_descriptor(&m),
		("inspect", Some(ref m)) => exec_inspect(&m),
		("parse", Some(ref m)) => exec_parse(&m),
		("policy", Some(ref m)) => exec_policy(&m),
		("compile", Some(ref m)) => exec_compile(&m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_descriptor<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("descriptor", "get information about an output descriptor")
		.args(&[args::arg("descriptor", "the output descriptor to inspect").required(false)])
}

fn exec_descriptor<'a>(args: &clap::ArgMatches<'a>) {
	let desc_str = util::arg_or_stdin(args, "descriptor");
	let network = args.network();

	let info = desc_str
		.parse::<Descriptor<bitcoin::PublicKey>>()
		.map(|desc| DescriptorInfo {
			descriptor: desc.to_string(),
			key_type: MiniscriptKeyType::PublicKey,
			address: desc.address(network).map(|a| a.to_string()).ok(),
			script_pubkey: Some(desc.script_pubkey().into_bytes().into()),
			unsigned_script_sig: Some(desc.unsigned_script_sig().into_bytes().into()),
			witness_script: desc.explicit_script().map(|s| s.into_bytes().into()).ok(),
			max_satisfaction_weight: desc.max_weight_to_satisfy().ok().map(|w| w.to_wu()),
			policy: policy::Liftable::lift(&desc).map(|pol| pol.to_string()).ok(),
		})
		.or_else(|e| {
			debug!("Can't parse descriptor with public keys: {}", e);
			// Then try with strings.
			desc_str.parse::<Descriptor<String>>().map(|desc| DescriptorInfo {
				descriptor: desc.to_string(),
				key_type: MiniscriptKeyType::String,
				address: None,
				script_pubkey: None,
				unsigned_script_sig: None,
				witness_script: None,
				max_satisfaction_weight: desc.max_weight_to_satisfy().ok().map(|w| w.to_wu()),
				policy: policy::Liftable::lift(&desc).map(|pol| pol.to_string()).ok(),
			})
		})
		.need("invalid miniscript");
	args.print_output(&info);
}

fn cmd_inspect<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("inspect", "inspect miniscripts")
		.arg(args::arg("miniscript", "the miniscript to inspect").required(false))
}

fn exec_inspect<'a>(args: &clap::ArgMatches<'a>) {
	let input = util::arg_or_stdin(args, "miniscript");
	let miniscript_str = input.as_ref();

	// First try with pubkeys.
	let bare_info = Miniscript::<bitcoin::PublicKey, BareCtx>::from_str_insane(miniscript_str)
		.map_err(|e| debug!("Cannot parse as Bare Miniscript {}", e))
		.map(|x| {
			let script = x.encode();
			MiniscriptInfo::from_bare(x, MiniscriptKeyType::PublicKey, Some(script))
		})
		.ok();
	let p2sh_info = Miniscript::<bitcoin::PublicKey, Legacy>::from_str_insane(miniscript_str)
		.map_err(|e| debug!("Cannot parse as Legacy/p2sh Miniscript {}", e))
		.map(|x| {
			let script = x.encode();
			MiniscriptInfo::from_p2sh(x, MiniscriptKeyType::PublicKey, Some(script))
		})
		.ok();
	let segwit_info = Miniscript::<bitcoin::PublicKey, Segwitv0>::from_str_insane(miniscript_str)
		.map_err(|e| info!("Cannot parse as Segwitv0 Miniscript {}", e))
		.map(|x| {
			let script = x.encode();
			MiniscriptInfo::from_segwitv0(x, MiniscriptKeyType::PublicKey, Some(script))
		})
		.ok();
	let info = if bare_info.is_none() && p2sh_info.is_none() && segwit_info.is_none() {
		// Try as Strings
		let bare_info = Miniscript::<String, BareCtx>::from_str_insane(miniscript_str)
			.map_err(|e| debug!("Cannot parse as Bare Miniscript {}", e))
			.map(|x| MiniscriptInfo::from_bare(x, MiniscriptKeyType::String, None))
			.ok();
		let p2sh_info = Miniscript::<String, Legacy>::from_str_insane(miniscript_str)
			.map_err(|e| debug!("Cannot parse as Legacy/p2sh Miniscript {}", e))
			.map(|x| MiniscriptInfo::from_p2sh(x, MiniscriptKeyType::String, None))
			.ok();
		let segwit_info = Miniscript::<String, Segwitv0>::from_str_insane(miniscript_str)
			.map_err(|e| info!("Cannot parse as Segwitv0 Miniscript {}", e))
			.map(|x| MiniscriptInfo::from_segwitv0(x, MiniscriptKeyType::String, None))
			.ok();

		MiniscriptInfo::combine(MiniscriptInfo::combine(bare_info, p2sh_info), segwit_info)
			.need("Invalid Miniscript")
	} else {
		MiniscriptInfo::combine(MiniscriptInfo::combine(bare_info, p2sh_info), segwit_info)
			.unwrap()
	};
	args.print_output(&info);
}

fn cmd_parse<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("parse", "parse a script into a miniscript")
		.arg(args::arg("script", "hex script to parse").required(false))
}

fn exec_parse<'a>(args: &clap::ArgMatches<'a>) {
	let script_hex = util::arg_or_stdin(args, "script");
	let script = ScriptBuf::from_hex(&script_hex).need("invalid hex script");

	let segwit_info = Miniscript::<_, Segwitv0>::parse_insane(&script)
		.map_err(|e| info!("Cannot parse as segwit Miniscript {}", e))
		.map(|x| {
			MiniscriptInfo::from_segwitv0(x, MiniscriptKeyType::PublicKey, Some(script.clone()))
		})
		.ok();
	let legacy_info = Miniscript::<_, Legacy>::parse_insane(&script)
		.map_err(|e| debug!("Cannot parse as Legacy Miniscript {}", e))
		.map(|x| MiniscriptInfo::from_p2sh(x, MiniscriptKeyType::PublicKey, Some(script.clone())))
		.ok();
	let bare_info = Miniscript::<_, BareCtx>::parse_insane(&script)
		.map_err(|e| debug!("Cannot parse as Bare Miniscript {}", e))
		.map(|x| MiniscriptInfo::from_bare(x, MiniscriptKeyType::PublicKey, Some(script)))
		.ok();
	if segwit_info.is_none() && legacy_info.is_none() && bare_info.is_none() {
		exit!("Invalid Miniscript under all script contexts")
	}

	let comb_info =
		MiniscriptInfo::combine(MiniscriptInfo::combine(bare_info, legacy_info), segwit_info)
			.unwrap();
	args.print_output(&comb_info);
}

fn cmd_policy<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("policy", "inspect policies")
		.arg(args::arg("policy", "the miniscript policy to inspect").required(false))
}

fn get_policy_info<Pk: MiniscriptKey + FromStrKey>(
	policy_str: &str,
	key_type: MiniscriptKeyType,
) -> Result<PolicyInfo, miniscript::Error>
where
	Pk: std::str::FromStr,
	<Pk as std::str::FromStr>::Err: std::fmt::Display,
	<Pk as MiniscriptKey>::Sha256: std::str::FromStr,
	<Pk as MiniscriptKey>::Hash256: std::str::FromStr,
	<Pk as MiniscriptKey>::Ripemd160: std::str::FromStr,
	<Pk as MiniscriptKey>::Hash160: std::str::FromStr,
	<<Pk as MiniscriptKey>::Sha256 as std::str::FromStr>::Err: std::fmt::Display,
	<<Pk as MiniscriptKey>::Hash256 as std::str::FromStr>::Err: std::fmt::Display,
	<<Pk as MiniscriptKey>::Ripemd160 as std::str::FromStr>::Err: std::fmt::Display,
	<<Pk as MiniscriptKey>::Hash160 as std::str::FromStr>::Err: std::fmt::Display,
{
	let concrete_pol: Option<policy::Concrete<Pk>> = policy_str.parse().ok();
	let policy = match concrete_pol {
		Some(ref concrete) => policy::Liftable::lift(concrete)?,
		None => policy_str.parse()?,
	};
	Ok(PolicyInfo {
		is_concrete: concrete_pol.is_some(),
		key_type: key_type,
		is_trivial: policy.is_trivial(),
		is_unsatisfiable: policy.is_unsatisfiable(),
		relative_timelocks: policy.relative_timelocks(),
		n_keys: policy.n_keys(),
		minimum_n_keys: policy.minimum_n_keys().ok_or(miniscript::Error::CouldNotSatisfy)?,
		sorted: policy.clone().sorted().to_string(),
		normalized: policy.clone().normalized().to_string(),
		miniscript: concrete_pol.map(|p| Miniscripts {
			bare: match policy::compiler::best_compilation::<Pk, BareCtx>(&p) {
				Ok(ms) => Some(ms.to_string()),
				Err(e) => {
					debug!("Compiler error: {}", e);
					None
				}
			},
			p2sh: match policy::compiler::best_compilation::<Pk, Legacy>(&p) {
				Ok(ms) => Some(ms.to_string()),
				Err(e) => {
					debug!("Compiler error: {}", e);
					None
				}
			},
			segwitv0: match policy::compiler::best_compilation::<Pk, Segwitv0>(&p) {
				Ok(ms) => Some(ms.to_string()),
				Err(e) => {
					debug!("Compiler error: {}", e);
					None
				}
			},
		}),
	})
}

fn exec_policy<'a>(args: &clap::ArgMatches<'a>) {
	let input = util::arg_or_stdin(args, "policy");
	let policy_str = input.as_ref();

	// First try a concrete policy with pubkeys.
	if let Ok(info) =
		get_policy_info::<bitcoin::PublicKey>(policy_str, MiniscriptKeyType::PublicKey)
	{
		args.print_output(&info)
	} else {
		// Then try with strings.
		match get_policy_info::<String>(policy_str, MiniscriptKeyType::String) {
			Ok(info) => args.print_output(&info),
			Err(e) => exit!("Invalid policy: {}", e),
		}
	}
}

fn cmd_compile<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("compile", "compile a policy into a script")
		.arg(args::arg("policy", "the miniscript policy to compile").required(false))
		.arg(
			clap::Arg::with_name("type")
				.long("type")
				.takes_value(true)
				.possible_values(&["bare", "p2sh", "segwitv0", "tapscript"])
				.default_value("tapscript")
				.help("script type to compile to"),
		)
}

fn exec_compile<'a>(args: &clap::ArgMatches<'a>) {
	let policy_str = util::arg_or_stdin(args, "policy");
	let script_type = args.value_of("type").unwrap();

	let result = policy_str
		.parse::<policy::Concrete<bitcoin::PublicKey>>()
		.map_err(|e| format!("Invalid concrete policy: {}", e))
		.and_then(|concrete| {
			match script_type {
				"bare" => policy::compiler::best_compilation::<_, BareCtx>(&concrete)
					.map(|ms| ms.encode()),
				"p2sh" => policy::compiler::best_compilation::<_, Legacy>(&concrete)
					.map(|ms| ms.encode()),
				"segwitv0" => policy::compiler::best_compilation::<_, Segwitv0>(&concrete)
					.map(|ms| ms.encode()),
				"tapscript" => policy::compiler::best_compilation::<_, miniscript::Tap>(&concrete)
					.map(|ms| ms.encode()),
				_ => unreachable!("clap enforces valid values"),
			}
			.map_err(|e| format!("Compilation error: {}", e))
		});

	match result {
		Ok(script) => {
			#[derive(serde::Serialize)]
			struct ScriptOutput {
				hex: String,
				asm: String,
			}
			let output = ScriptOutput {
				hex: script.as_bytes().as_hex().to_string(),
				asm: script.to_string(), 
			};
			
			args.print_output(&output)
		}
		Err(e) => exit!("{}", e),
	}
}

trait FromScriptContexts: Sized {
	fn from_bare<Pk: MiniscriptKey>(
		ms: Miniscript<Pk, BareCtx>,
		key_type: MiniscriptKeyType,
		script: Option<bitcoin::ScriptBuf>,
	) -> Self;
	fn from_p2sh<Pk: MiniscriptKey>(
		ms: Miniscript<Pk, Legacy>,
		key_type: MiniscriptKeyType,
		script: Option<bitcoin::ScriptBuf>,
	) -> Self;
	fn from_segwitv0<Pk: MiniscriptKey>(
		ms: Miniscript<Pk, Segwitv0>,
		key_type: MiniscriptKeyType,
		script: Option<bitcoin::ScriptBuf>,
	) -> Self;
	fn combine(a: Option<Self>, b: Option<Self>) -> Option<Self>;
}

impl FromScriptContexts for MiniscriptInfo {
	fn from_bare<Pk: MiniscriptKey>(
		ms: Miniscript<Pk, BareCtx>,
		key_type: MiniscriptKeyType,
		script: Option<bitcoin::ScriptBuf>,
	) -> Self {
		Self {
			key_type: key_type,
			valid_script_contexts: ScriptContexts::from_bare(true),
			script_size: ms.script_size(),
			max_satisfaction_witness_elements: ms.max_satisfaction_witness_elements().ok(),
			max_satisfaction_size_segwit: None,
			max_satisfaction_size_non_segwit: ms.max_satisfaction_size().ok(),
			script: script.map(|x| x.into_bytes().into()),
			policy: match ms.lift() {
				Ok(pol) => Some(pol.to_string()),
				Err(e) => {
					info!("Lift error {}: BareCtx", e);
					None
				}
			},
			requires_sig: ms.requires_sig(),
			non_malleable: ScriptContexts::from_bare(ms.is_non_malleable()),
			within_resource_limits: ScriptContexts::from_bare(ms.within_resource_limits()),
			has_mixed_timelocks: ms.has_mixed_timelocks(),
			has_repeated_keys: ms.has_repeated_keys(),
			sane_miniscript: ScriptContexts::from_bare(ms.sanity_check().is_ok()),
		}
	}

	fn from_p2sh<Pk: MiniscriptKey>(
		ms: Miniscript<Pk, Legacy>,
		key_type: MiniscriptKeyType,
		script: Option<bitcoin::ScriptBuf>,
	) -> Self {
		Self {
			key_type: key_type,
			valid_script_contexts: ScriptContexts::from_p2sh(true),
			script_size: ms.script_size(),
			max_satisfaction_witness_elements: ms.max_satisfaction_witness_elements().ok(),
			max_satisfaction_size_segwit: None,
			max_satisfaction_size_non_segwit: ms.max_satisfaction_size().ok(),
			script: script.map(|x| x.into_bytes().into()),
			policy: match ms.lift() {
				Ok(pol) => Some(pol.to_string()),
				Err(e) => {
					info!("Lift error {}: Legacy(p2sh) context", e);
					None
				}
			},
			requires_sig: ms.requires_sig(),
			non_malleable: ScriptContexts::from_p2sh(ms.is_non_malleable()),
			within_resource_limits: ScriptContexts::from_p2sh(ms.within_resource_limits()),
			has_mixed_timelocks: ms.has_mixed_timelocks(),
			has_repeated_keys: ms.has_repeated_keys(),
			sane_miniscript: ScriptContexts::from_p2sh(ms.sanity_check().is_ok()),
		}
	}

	fn from_segwitv0<Pk: MiniscriptKey>(
		ms: Miniscript<Pk, Segwitv0>,
		key_type: MiniscriptKeyType,
		script: Option<bitcoin::ScriptBuf>,
	) -> Self {
		Self {
			key_type: key_type,
			valid_script_contexts: ScriptContexts::from_segwitv0(true),
			script_size: ms.script_size(),
			max_satisfaction_witness_elements: ms.max_satisfaction_witness_elements().ok(),
			max_satisfaction_size_segwit: ms.max_satisfaction_size().ok(),
			max_satisfaction_size_non_segwit: None,
			script: script.map(|x| x.into_bytes().into()),
			policy: match ms.lift() {
				Ok(pol) => Some(pol.to_string()),
				Err(e) => {
					info!("Lift error {}: Segwitv0 Context", e);
					None
				}
			},
			requires_sig: ms.requires_sig(),
			non_malleable: ScriptContexts::from_segwitv0(ms.is_non_malleable()),
			within_resource_limits: ScriptContexts::from_segwitv0(ms.within_resource_limits()),
			has_mixed_timelocks: ms.has_mixed_timelocks(),
			has_repeated_keys: ms.has_repeated_keys(),
			sane_miniscript: ScriptContexts::from_segwitv0(ms.sanity_check().is_ok()),
		}
	}

	// Helper function to combine two Miniscript Infos of same key types
	// Used to combine Infos from different scriptContexts
	fn combine(a: Option<Self>, b: Option<Self>) -> Option<Self> {
		match (a, b) {
			(None, None) => None,
			(None, Some(b)) => Some(b),
			(Some(a), None) => Some(a),
			(Some(a), Some(b)) => {
				debug_assert!(a.key_type == b.key_type);
				Some(Self {
					key_type: a.key_type,
					valid_script_contexts: ScriptContexts::or(
						a.valid_script_contexts,
						b.valid_script_contexts,
					),
					script_size: a.script_size,
					max_satisfaction_witness_elements: a
						.max_satisfaction_witness_elements
						.or(b.max_satisfaction_witness_elements),
					max_satisfaction_size_segwit: a
						.max_satisfaction_size_segwit
						.or(b.max_satisfaction_size_segwit),
					max_satisfaction_size_non_segwit: a
						.max_satisfaction_size_non_segwit
						.or(b.max_satisfaction_size_non_segwit),
					script: a.script,
					policy: a.policy.or(b.policy),
					requires_sig: a.requires_sig,
					non_malleable: ScriptContexts::or(a.non_malleable,b.non_malleable),
					within_resource_limits: ScriptContexts::or(a.within_resource_limits,b.within_resource_limits),
					has_mixed_timelocks: a.has_mixed_timelocks,
					has_repeated_keys: b.has_repeated_keys,
					sane_miniscript: ScriptContexts::or(a.sane_miniscript,b.sane_miniscript),
				})
			}
		}
	}
}
