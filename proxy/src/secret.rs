use anyhow::{format_err, Result};
use chacha20poly1305::{ChaCha20Poly1305, Key};
use chacha20poly1305::aead::{AeadInPlace, NewAead};
use rand::Rng;

#[derive(Clone)]
pub struct Secret(pub Key);

impl Secret {
	pub fn new() -> Self {
		let key = rand::thread_rng().gen::<[u8; 32]>();
		Self(key.into())
	}

	pub fn from_slice(key: &[u8]) -> Result<Self> {
		if key.len() != 32 {
			return Err(format_err!("Invalid key length"));
		}
		Ok(Self(generic_array::GenericArray::clone_from_slice(key)))
	}

	/// Encrypt and mac
	pub fn seal(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
		let cipher = ChaCha20Poly1305::new(&self.0);
		let nonce = rand::thread_rng().gen::<[u8; 12]>();
		let nonce = nonce.into();
		cipher.encrypt_in_place(&nonce, &[], &mut data)
			.map_err(|_| format_err!("Failed to encrypt secret"))?;
		data.extend_from_slice(nonce.as_slice());
		Ok(data)
	}

	/// Mac and decrypt
	pub fn open(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
		let cipher = ChaCha20Poly1305::new(&self.0);
		let mut nonce = [0; 12];
		let nonce_len = nonce.len();
		if data.len() < nonce_len {
			return Err(format_err!("Cannot decrypt too short data"));
		}
		nonce.copy_from_slice(&data[data.len() - nonce_len..]);
		data.truncate(data.len() - nonce_len);

		cipher.decrypt_in_place(&nonce.into(), &[], &mut data)
			.map_err(|_| format_err!("Failed to decrypt secret"))?;
		Ok(data)
	}
}
