use crate::{QintState, db::AddIdentityMsg};
use anyhow::Result;
use proxy_codegen::book_events::{deserialize_u64, serialize_u64};
use serde::{Deserialize, Serialize};
use serde_ini::de::from_str;

// TODO multiidentity

#[derive(Debug, Serialize, Deserialize)]
struct TsExportIdentityFile {
	#[serde(rename(deserialize = "Identity"))]
	identity: TsExportIdentity,
}

#[derive(Debug, Serialize, Deserialize)]
struct TsExportIdentity {
	id: String,
	identity: String,
	nickname: String,
	phonetic_nickname: String,
}

/// Used to get a nicely de-/serializable view of an identity to get from the
/// web api.
/// Does not include the private key for security and usability reasons.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiIdentity {
	// Readonly
	#[serde(deserialize_with = "deserialize_u64", serialize_with = "serialize_u64")]
	pub id: u64,
	pub name: String,
	// Readonly
	pub uid: Vec<u8>,
	// Readonly
	pub level: u8,
}

impl ApiIdentity {
	pub fn from_identity(id: u64, name: String, identity: tsclientlib::Identity) -> ApiIdentity {
		ApiIdentity {
			id,
			name,
			uid: identity.key().to_pub().get_uid_no_base64(),
			level: identity.level(),
		}
	}
}

// TODO thiserror

pub async fn import_ts_identities_from_string(state: &QintState, any: &str) -> Result<()> {
	let import_result = if let Ok(exp) = from_str::<TsExportIdentityFile>(any) {
		let exp = exp.identity;
		match tsclientlib::Identity::new_from_str(&exp.identity.trim_matches('"')) {
			Ok(identity) => Ok((identity, exp.id, Some(exp.nickname), Some(exp.phonetic_nickname))),
			Err(err) => Err(err),
		}
	} else {
		match tsclientlib::Identity::new_from_str(any) {
			Ok(identity) => Ok((identity, "Import".into(), None, None)), // TODO allow none
			Err(err) => Err(err),
		}
	};

	let (identity, id, nickname, phonetic_nickname) = import_result?;
	let _ = state
		.database
		.send(AddIdentityMsg { identity, name: id, nickname, phonetic_nickname })
		.await??;
	Ok(())
}
