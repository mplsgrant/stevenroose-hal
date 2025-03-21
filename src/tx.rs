use bitcoin::consensus::encode::serialize;
use bitcoin::{address, Address, Amount, Network, Script, Transaction, TxIn, TxOut, Txid, Wtxid};
use serde::{Deserialize, Serialize};

use crate::{GetInfo, HexBytes};

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct InputScriptInfo {
	pub hex: Option<HexBytes>,
	pub asm: Option<String>,
}

pub struct InputScript<'a>(pub &'a Script);

impl<'a> GetInfo<InputScriptInfo> for InputScript<'a> {
	fn get_info(&self, _network: Network) -> InputScriptInfo {
		InputScriptInfo {
			hex: Some(self.0.to_bytes().into()),
			asm: Some(self.0.to_asm_string()),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct InputInfo {
	pub prevout: Option<String>,
	pub txid: Option<Txid>,
	pub vout: Option<u32>,
	pub script_sig: Option<InputScriptInfo>,
	pub sequence: Option<u32>,
	pub witness: Option<Vec<HexBytes>>,
}

impl GetInfo<InputInfo> for TxIn {
	fn get_info(&self, network: Network) -> InputInfo {
		InputInfo {
			prevout: Some(self.previous_output.to_string()),
			txid: Some(self.previous_output.txid),
			vout: Some(self.previous_output.vout),
			sequence: Some(self.sequence.to_consensus_u32()),
			script_sig: Some(InputScript(&self.script_sig).get_info(network)),
			witness: if self.witness.len() > 0 {
				Some(self.witness.iter().map(|h| h.into()).collect())
			} else {
				None
			},
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OutputScriptInfo {
	pub hex: Option<HexBytes>,
	pub asm: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none", rename = "type")]
	pub type_: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub address: Option<Address<address::NetworkUnchecked>>,
}

pub struct OutputScript<'a>(pub &'a Script);

impl<'a> GetInfo<OutputScriptInfo> for OutputScript<'a> {
	fn get_info(&self, network: Network) -> OutputScriptInfo {
		OutputScriptInfo {
			hex: Some(self.0.to_bytes().into()),
			asm: Some(self.0.to_asm_string()),
			type_: Some(
				if self.0.is_p2pk() {
					"p2pk"
				} else if self.0.is_p2pkh() {
					"p2pkh"
				} else if self.0.is_op_return() {
					"opreturn"
				} else if self.0.is_p2sh() {
					"p2sh"
				} else if self.0.is_p2wpkh() {
					"p2wpkh"
				} else if self.0.is_p2wsh() {
					"p2wsh"
				} else if self.0.is_p2tr() {
					"p2tr"
				} else {
					"unknown"
				}
				.to_owned(),
			),
			address: Address::from_script(&self.0, network).ok().map(|a| a.as_unchecked().clone()),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OutputInfo {
	#[serde(with = "bitcoin::amount::serde::as_sat::opt")]
	pub value: Option<Amount>,
	pub script_pub_key: Option<OutputScriptInfo>,
}

impl GetInfo<OutputInfo> for TxOut {
	fn get_info(&self, network: Network) -> OutputInfo {
		OutputInfo {
			value: Some(self.value),
			script_pub_key: Some(OutputScript(&self.script_pubkey).get_info(network)),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct TransactionInfo {
	pub txid: Option<Txid>,
	pub wtxid: Option<Wtxid>,
	pub size: Option<usize>,
	pub weight: Option<usize>,
	pub vsize: Option<usize>,
	pub version: Option<i32>,
	pub locktime: Option<u32>,
	pub inputs: Option<Vec<InputInfo>>,
	pub outputs: Option<Vec<OutputInfo>>,
	pub total_output_value: Option<u64>,
}

impl GetInfo<TransactionInfo> for Transaction {
	fn get_info(&self, network: Network) -> TransactionInfo {
		let weight = self.weight().to_wu() as usize;
		TransactionInfo {
			txid: Some(self.compute_txid()),
			wtxid: Some(self.compute_wtxid()),
			version: Some(self.version.0),
			locktime: Some(self.lock_time.to_consensus_u32()),
			size: Some(serialize(self).len()),
			weight: Some(weight),
			vsize: Some(weight / 4),
			inputs: Some(self.input.iter().map(|i| i.get_info(network)).collect()),
			outputs: Some(self.output.iter().map(|o| o.get_info(network)).collect()),
			total_output_value: Some(self.output.iter().map(|o| o.value.to_sat()).sum()),
		}
	}
}
