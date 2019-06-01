use qint_shared::*;
use stdweb::{js, _js_impl, Value};
use yew::callback::Callback;

pub struct Webrtc {
	callback: Callback<WebrtcMsg>,
	con: Value,
}

impl Webrtc {
	pub fn new(callback: Callback<WebrtcMsg>) -> Self {
		let call = callback.clone();
		let got_ice = move |i, c| {
			call.emit(WebrtcMsg::Ice {
				candidate: c,
				sdp_mline_index: i,
			});
		};

		let call = callback.clone();
		let got_sdp = move |typ, sdp| {
			call.emit(WebrtcMsg::Sdp {
				typ,
				sdp,
			});
		};

		// TODO Also handle received sdp offers?
		let con = js! {
			var peerConnectionConfig = {"iceServers": [{"urls": "stun:stun.services.mozilla.com"}, {"urls": "stun:stun.l.google.com:19302"}]};
			var con = new RTCPeerConnection(peerConnectionConfig);
			con.onicecandidate = function(e) {
				if (e.candidate != null) {
					console.log("Local ICE");
					console.log(e.candidate);
					@{got_ice}(e.candidate.sdpMLineIndex, e.candidate.candidate);
				}
			};
			con.ontrack = function(event) {
				console.log("Got remote stream");
				console.log(event);
				var playback = document.getElementById("audio-playback");
				playback.srcObject = event.streams[0];
			};

			var constraints = {
				video: false,
				audio: true,
			};
			navigator.mediaDevices.getUserMedia(constraints).then(function(stream) {
				con.addStream(stream);
				console.log("Added stream");
			})
			.catch(function (e) {
				console.log("Failed to get user media " + e);
			});

			/*con.createOffer(function(description) {
				con.setLocalDescription(description, function () {
					@{got_sdp}(description.type, JSON.stringify(description.sdp));
				}, function() { console.log("set description error"); });
			}, function(e) {
				console.log("Failed to get offer" + e);
			});*/
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
					console.log("Got ice ");
					console.log(ice);
					con.addIceCandidate(new RTCIceCandidate(ice));
				};
			}
			WebrtcMsg::Sdp { typ, sdp } => {
				let call = self.callback.clone();
				let got_sdp = move |typ, sdp| {
					call.emit(WebrtcMsg::Sdp {
						typ,
						sdp,
					});
				};

				let on_error = |e: Value| {
					// TODO Use logger
					log::error!("Failed");
				};

				js! { @(no_return)
					var con = @{&self.con};
					var sdp = @{sdp};
					console.log("Got sdp " + sdp);
					var sdp = {type: @{typ}, sdp: sdp};
					con.setRemoteDescription(new RTCSessionDescription(sdp)).then(function() {
						// Only create answers in response to offers
						if(sdp.type == "offer") {
							con.createAnswer(function(description) {
								con.setLocalDescription(description, function () {
									@{got_sdp}(description.type, description.sdp);
								}, function() { console.log("set description error"); });
							}, @{on_error});
						}
					}).catch(@{on_error});
				};
			}
		}
	}
}
