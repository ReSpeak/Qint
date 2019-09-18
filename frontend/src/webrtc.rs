use qint_shared::*;
use stdweb::{js, Value};
use yew::callback::Callback;

pub struct Webrtc {
	callback: Callback<Option<WebrtcMsg>>,
	con: Value,
}

impl Webrtc {
	pub fn new(callback: Callback<Option<WebrtcMsg>>) -> Self {
		let call = callback.clone();
		let send_ice = move |i, c| {
			call.emit(Some(WebrtcMsg::Ice {
				candidate: c,
				sdp_mline_index: i,
			}));
		};

		/*let call = callback.clone();
		let send_sdp = move |typ, sdp| {
			call.emit(Some(WebrtcMsg::Sdp {
				typ,
				sdp,
			}));
		};*/

		let call = callback.clone();
		let send_ready = move || {
			call.emit(None);
		};

		let con = js! {
			var peerConnectionConfig = {"iceServers": [
				{"urls": "stun:stun.services.mozilla.com"},
				{"urls": "stun:stun.l.google.com:19302"}
			]};
			var con = new RTCPeerConnection(peerConnectionConfig);
			con.onicecandidate = function(e) {
				if (e.candidate != null) {
					@{send_ice}(e.candidate.sdpMLineIndex, e.candidate.candidate);
				}
			};
			con.ontrack = function(ev) {
				var playback = document.getElementById("audio-playback");
				if (ev.streams && ev.streams[0]) {
					playback.srcObject = ev.streams[0];
				} else {
					// Add the track to a stream (group of track) if there is no
					// stream.
					let inboundStream = new MediaStream(track);
					playback.srcObject = inboundStream;
				}
			};

			var constraints = {
				video: false,
				audio: true,
			};
			navigator.mediaDevices.getUserMedia(constraints).then(function(stream) {
				stream.getTracks().forEach(function(track) {
					con.addTrack(track, stream);
					// Pause by default
					//track.enabled = false;
					console.log("Track added");
				});
				@{send_ready}();
			})
			.catch(function (e) {
				console.log("Failed to get user media " + e);
			});

			con.onnegotiationneeded = function() {
				console.log("Renegotiation is not yet supported by gstreamer");
				/*con.createOffer().then(function(description) {
					@{send_sdp}(description.type, description.sdp);
					return con.setLocalDescription(description);
				}).catch(function(e) {
					console.log("Failed to negotiate " + e);
				});*/
			};
			return con;
		};

		Self {
			callback,
			con,
		}
	}

	pub fn handle(&mut self, msg: WebrtcMsg) {
		match msg {
			WebrtcMsg::Ice { candidate, sdp_mline_index } => {
				js! { @(no_return)
					var con = @{&self.con};
					var ice = {candidate: @{candidate}, sdpMLineIndex: @{sdp_mline_index}};
					con.addIceCandidate(new RTCIceCandidate(ice));
				};
			}
			WebrtcMsg::Sdp { typ, sdp } => {
				let call = self.callback.clone();
				let send_sdp = move |typ, sdp| {
					call.emit(Some(WebrtcMsg::Sdp {
						typ,
						sdp,
					}));
				};

				js! { @(no_return)
					var con = @{&self.con};
					var sdp = @{sdp};
					var sdp = {type: @{typ}, sdp: sdp};
					con.setRemoteDescription(new RTCSessionDescription(sdp)).then(function() {
						// Only create answers in response to offers
						if(sdp.type == "offer") {
							return con.createAnswer().then(function(description) {
								@{send_sdp}(description.type, description.sdp);
								return con.setLocalDescription(description);
							});
						}
					}).catch(function(e) {
						console.log("Failed to set remote description " + e);
					});
				};
			}
		}
	}

	pub fn set_talking(&mut self, talk: bool) {
		js! { @(no_return)
			let senders = @{&self.con}.getSenders();
			if (senders[0])
				senders[0].track.enabled = @{talk};
		}
	}
}
