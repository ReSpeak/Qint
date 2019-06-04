use failure::{format_err, Error};
use ring::aead;
use ring::aead::CHACHA20_POLY1305 as ALG;
use ring::rand::{SecureRandom, SystemRandom};

pub struct Secret(pub Vec<u8>);

impl Secret {
	pub fn new() -> Result<Self, Error> {
		let rand = SystemRandom::new();
		let mut key = vec![0; ALG.key_len()];
		rand.fill(&mut key)?;
		Ok(Self(key))
	}

	/// Encrypt and mac
	pub fn seal(&self, mut data: Vec<u8>) -> Result<Vec<u8>, Error> {
		let rand = SystemRandom::new();
		let key = aead::SealingKey::new(&ALG, &self.0)?;
		let mut nonce_data = [0; 12];
		rand.fill(&mut nonce_data[..])?;
		let nonce = aead::Nonce::assume_unique_for_key(nonce_data.clone());

		data.resize(data.len() + ALG.tag_len(), 0);
		aead::seal_in_place(&key, nonce, aead::Aad::empty(),
			&mut data, ALG.tag_len())?;
		data.extend_from_slice(&nonce_data);

		Ok(data)
	}

	/// Mac and decrypt
	pub fn open(&self, mut data: Vec<u8>) -> Result<Vec<u8>, Error> {
		let mut nonce_data = [0; 12];
		let nonce_len = nonce_data.len();
		if data.len() < nonce_data.len() {
			return Err(format_err!("Cannot decrypt too short data"));
		}
		nonce_data.copy_from_slice(&data[data.len() - nonce_len..]);
		let nonce = aead::Nonce::assume_unique_for_key(nonce_data);
		data.truncate(data.len() - nonce_len);
		let key = aead::OpeningKey::new(&ALG, &self.0)?;

		let len = aead::open_in_place(&key, nonce, aead::Aad::empty(), 0, &mut data)?
			.len();
		data.truncate(len);
		Ok(data)
	}
}
