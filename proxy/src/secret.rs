use anyhow::{format_err, Result};
use ring::aead::CHACHA20_POLY1305 as ALG;
use ring::aead::*;
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};

#[derive(Clone)]
pub struct Secret(pub Vec<u8>);

struct SingleNonce(Option<Nonce>);

impl NonceSequence for SingleNonce {
	fn advance(&mut self) -> std::result::Result<Nonce, Unspecified> {
		self.0.take().map(Ok).unwrap_or(Err(Unspecified))
	}
}

impl Secret {
	pub fn new() -> Result<Self> {
		let rand = SystemRandom::new();
		let mut key = vec![0; ALG.key_len()];
		rand.fill(&mut key).map_err(|_| format_err!("Failed to create random numbers"))?;
		Ok(Self(key))
	}

	/// Encrypt and mac
	pub fn seal(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
		let rand = SystemRandom::new();
		let mut nonce_data = [0; 12];
		rand.fill(&mut nonce_data[..])
			.map_err(|_| format_err!("Failed to create random numbers"))?;
		let nonce = Nonce::assume_unique_for_key(nonce_data);
		let nonce = SingleNonce(Some(nonce));

		let mut key = SealingKey::new(
			UnboundKey::new(&ALG, &self.0).map_err(|_| format_err!("Failed to create key"))?,
			nonce,
		);

		key.seal_in_place_append_tag(Aad::empty(), &mut data)
			.map_err(|_| format_err!("Failed to create key"))?;
		data.extend_from_slice(&nonce_data);

		Ok(data)
	}

	/// Mac and decrypt
	pub fn open(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
		let mut nonce_data = [0; 12];
		let nonce_len = nonce_data.len();
		if data.len() < nonce_data.len() {
			return Err(format_err!("Cannot decrypt too short data"));
		}
		nonce_data.copy_from_slice(&data[data.len() - nonce_len..]);
		let nonce = Nonce::assume_unique_for_key(nonce_data);
		let nonce = SingleNonce(Some(nonce));
		data.truncate(data.len() - nonce_len);

		let mut key = OpeningKey::new(
			UnboundKey::new(&ALG, &self.0).map_err(|_| format_err!("Failed to create key"))?,
			nonce,
		);

		let len = key
			.open_in_place(Aad::empty(), &mut data)
			.map_err(|_| format_err!("Failed to create key"))?
			.len();
		data.truncate(len);
		Ok(data)
	}
}
