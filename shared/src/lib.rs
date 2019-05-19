use std::fmt;

use serde::{Deserialize, Serialize};
use tsproto_packets::packets::{Direction, InCommand, OutPacket, PacketType};
//use tsproto_types::versions::Version;

// TODO
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Version {
	Linux3_2_1,
}
impl fmt::Display for Version {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Linux3_2_1")?;
		Ok(())
	}
}

/// A message sent over a websocket connection from the frontend to the proxy.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MessageF2P {
	Connect(ConnectOptions),
	Packet(OutPacket),
}

/// A message sent over a websocket connection from the proxy to the frontend.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MessageP2F {
	ConnectFailed(),
	Packet(InCommandMsg),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InCommandMsg {
	content: Vec<u8>,
	p_type: PacketType,
	newprotocol: bool,
	dir: Direction,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ConnectOptions {
	pub address: String,
	pub name: String,
	pub version: Version,
	pub log_commands: bool,
	pub log_packets: bool,
	pub log_udp_packets: bool,
}

impl From<&'_ InCommand> for InCommandMsg {
	fn from(cmd: &InCommand) -> Self {
		InCommandMsg {
			content: cmd.content().to_vec(),
			p_type: cmd.packet_type(),
			newprotocol: cmd.newprotocol(),
			dir: cmd.direction(),
		}
	}
}

impl Into<InCommand> for InCommandMsg {
	fn into(self) -> InCommand {
		InCommand::new(self.content, self.p_type, self.newprotocol, self.dir)
			.unwrap()
	}
}

impl ConnectOptions {
	/// Start creating the configuration of a new connection.
	///
	/// # Arguments
	/// The address of the server has to be supplied. The address can be a
	/// [`SocketAddr`], a string or directly a [`ServerAddress`]. A string
	/// will automatically be resolved from all formats supported by TeamSpeak.
	/// For details, see [`resolver::resolve`].
	///
	/// [`SocketAddr`]: ../../std/net/enum.SocketAddr.html
	/// [`ServerAddress`]: enum.ServerAddress.html
	/// [`resolver::resolve`]: resolver/method.resolve.html
	#[inline]
	pub fn new(address: String) -> Self {
		Self {
			address,
			name: String::from("TeamSpeakUser"),
			version: Version::Linux3_2_1,
			log_commands: false,
			log_packets: false,
			log_udp_packets: false,
		}
	}

	/// The address of the server.
	#[inline]
	pub fn address(&mut self, address: String) -> &mut Self {
		self.address = address;
		self
	}

	/// The name of the user.
	///
	/// # Default
	/// `TeamSpeakUser`
	#[inline]
	pub fn name(&mut self, name: String) -> &mut Self {
		self.name = name;
		self
	}

	/// The displayed version of the client.
	///
	/// # Default
	/// `3.2.1 on Linux`
	#[inline]
	pub fn version(&mut self, version: Version) -> &mut Self {
		self.version = version;
		self
	}

	/// If the content of all commands should be written to the logger.
	///
	/// # Default
	/// `false`
	#[inline]
	pub fn log_commands(&mut self, log_commands: bool) -> &mut Self {
		self.log_commands = log_commands;
		self
	}

	/// If the content of all packets in high-level form should be written to
	/// the logger.
	///
	/// # Default
	/// `false`
	#[inline]
	pub fn log_packets(&mut self, log_packets: bool) -> &mut Self {
		self.log_packets = log_packets;
		self
	}

	/// If the content of all udp packets in byte-array form should be written
	/// to the logger.
	///
	/// # Default
	/// `false`
	#[inline]
	pub fn log_udp_packets(&mut self, log_udp_packets: bool) -> &mut Self {
		self.log_udp_packets = log_udp_packets;
		self
	}
}

impl fmt::Debug for ConnectOptions {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// Error if attributes are added
		let ConnectOptions {
			address,
			name,
			version,
			log_commands,
			log_packets,
			log_udp_packets,
		} = self;
		write!(
			f,
			"ConnectOptions {{ address: {:?}, \
			 name: {}, version: {}, \
			 log_commands: {}, log_packets: {}, log_udp_packets: {},",
			address,
			name,
			version,
			log_commands,
			log_packets,
			log_udp_packets,
		)?;
		write!(f, " }}")?;
		Ok(())
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DisconnectOptions {
	// TODO
	//reason: Option<Reason>,
	message: Option<String>,
}

impl Default for DisconnectOptions {
	#[inline]
	fn default() -> Self {
		Self {
			//reason: None,
			message: None,
		}
	}
}

impl DisconnectOptions {
	#[inline]
	pub fn new() -> Self { Self::default() }

	/*/// Set the reason for leaving.
	///
	/// # Default
	///
	/// None
	#[inline]
	pub fn reason(&mut self, reason: Reason) -> &mut Self {
		self.reason = Some(reason);
		self
	}*/

	/// Set the leave message.
	///
	/// You also have to set the reason, otherwise the message will not be
	/// displayed.
	///
	/// # Default
	///
	/// None
	#[inline]
	pub fn message<S: Into<String>>(&mut self, message: S) -> &mut Self {
		self.message = Some(message.into());
		self
	}
}
