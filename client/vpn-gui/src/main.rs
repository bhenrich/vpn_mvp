use std::sync::Arc;

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Application, Command, Element, Length, Settings, Theme};
use vpn_core::{
	AuthMethod, CredentialStore, DnsSettings, FileProfileStore, OsKeyringCredentialStore, RealCommandExecutor,
	SplitTunnel, VpnAdapter, VpnProfile,
};
use vpn_core::ProfileStore;
use vpn_core::{AuthApiClient, TokenStore, ServerProfile, LoginResponse};

#[cfg(target_os = "windows")]
use platform_windows::WindowsAdapter;
#[cfg(target_os = "macos")]
use platform_macos::MacosAdapter;
#[cfg(target_os = "linux")]
use platform_linux::LinuxNmAdapter;

pub fn main() -> iced::Result {
	VpnApp::run(Settings::default())
}

struct VpnApp {
	adapter: Arc<dyn VpnAdapter>,
	profiles_store: Arc<FileProfileStore>,
	creds_store: Arc<OsKeyringCredentialStore>,
	rt: Arc<tokio::runtime::Runtime>,
	auth: AuthApiClient,
	tokens: Arc<TokenStore>,

	// UI state
	profiles: Vec<VpnProfile>,
	selected: Option<usize>,
	pending_select_name: Option<String>,

	// Auth state
	email: String,
	password: String,
	access_token: Option<String>,
	current_profile_name: Option<String>,

	// Add form state
	add_name: String,
	add_server: String,
	add_remote_id: String,
	add_split_tunnel: bool,
	add_dns: String,
	add_username: String,

	// Creds form state
	creds_username: String,
	creds_password: String,

	status_message: String,
}

#[derive(Debug, Clone)]
enum Msg {
	Noop,
	LoadProfiles,
	ProfilesLoaded(Result<Vec<VpnProfile>, String>),
	SelectProfile(usize),

	// Auth
	EmailChanged(String),
	PasswordChanged(String),
	Login,
	LoggedIn(Result<LoginResponse, String>),
	ProfileFetched(Result<ServerProfile, String>),

	AddNameChanged(String),
	AddServerChanged(String),
	AddRemoteIdChanged(String),
	AddSplitTunnelChanged(bool),
	AddDnsChanged(String),
	AddUsernameChanged(String),
	AddSubmit,
	Added(Result<(), String>),

	SetCredsUsernameChanged(String),
	SetCredsPasswordChanged(String),
	SetCredsSubmit,
	CredsSet(Result<(), String>),

	Connect,
	Connected(Result<(), String>),
	Disconnect,
	Disconnected(Result<(), String>),
	Remove,
	Removed(Result<(), String>),
}

impl Application for VpnApp {
	type Executor = iced::executor::Default;
	type Flags = ();
	type Message = Msg;
	type Theme = Theme;

	fn new(_flags: ()) -> (Self, Command<Self::Message>) {
		let exec: Arc<dyn vpn_core::CommandExecutor> = Arc::new(RealCommandExecutor);
		let profiles_store = Arc::new(FileProfileStore::new("vpn_mvp").expect("config dir"));
		let creds_store = Arc::new(OsKeyringCredentialStore);
		let rt = Arc::new(tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("tokio rt"));
		let auth = AuthApiClient::new_from_env().expect("auth");
		let tokens = Arc::new(TokenStore::new());

		#[cfg(target_os = "windows")]
		let adapter = Arc::new(WindowsAdapter::new(exec.clone(), creds_store.clone(), profiles_store.clone()))
			as Arc<dyn VpnAdapter>;
		#[cfg(target_os = "macos")]
		let adapter = Arc::new(MacosAdapter::new(exec.clone(), creds_store.clone(), profiles_store.clone()))
			as Arc<dyn VpnAdapter>;
		#[cfg(target_os = "linux")]
		let adapter = Arc::new(LinuxNmAdapter::new(exec.clone(), profiles_store.clone())) as Arc<dyn VpnAdapter>;

		(
			Self {
				adapter,
				profiles_store,
				creds_store,
				rt,
				auth,
				tokens,
				profiles: Vec::new(),
				selected: None,
				pending_select_name: None,
				email: String::new(),
				password: String::new(),
				access_token: None,
				current_profile_name: None,
				add_name: String::new(),
				add_server: String::new(),
				add_remote_id: String::new(),
				add_split_tunnel: false,
				add_dns: String::new(),
				add_username: String::new(),
				creds_username: String::new(),
				creds_password: String::new(),
				status_message: String::new(),
			},
			Command::perform(async {}, |_| Msg::LoadProfiles),
		)
	}

	fn title(&self) -> String {
		"VPN Client".into()
	}

	fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
		match message {
			Msg::Noop => Command::none(),
			Msg::LoadProfiles => {
				let store = self.profiles_store.clone();
				return Command::perform(async move { store.list().await.map_err(|e| e.to_string()) }, Msg::ProfilesLoaded);
			}
			Msg::ProfilesLoaded(res) => {
				match res {
					Ok(list) => {
						self.profiles = list;
						if let Some(name) = self.pending_select_name.take() {
							if let Some((idx, _)) = self.profiles.iter().enumerate().find(|(_, p)| p.name == name) {
								self.selected = Some(idx);
							}
						}
					},
					Err(err) => self.status_message = format!("Failed to load profiles: {err}"),
				}
				Command::none()
			}
			Msg::EmailChanged(s) => {
				self.email = s;
				Command::none()
			}
			Msg::PasswordChanged(s) => {
				self.password = s;
				Command::none()
			}
			Msg::Login => {
				let email = self.email.clone();
				let password = self.password.clone();
				let auth = self.auth.clone();
				return Command::perform(
					async move {
						auth.login(&email, &password).await.map_err(|e| e.to_string())
					},
					Msg::LoggedIn,
				);
			}
			Msg::LoggedIn(res) => {
				match res {
					Ok(resp) => {
						self.status_message = "Logged in".into();
						self.access_token = Some(resp.access_token.clone());
						// Save tokens and fetch profile
						let email = self.email.clone();
						let tokens = self.tokens.clone();
						let access = resp.access_token.clone();
						let refresh = resp.refresh_token.clone();
						let auth = self.auth.clone();
						let token_for_profile = resp.access_token.clone();
						return Command::perform(async move {
							// Save tokens first
							let _ = tokens.save(&email, &access, &refresh).await;
							// Then fetch profile
							auth.fetch_profile(&token_for_profile).await.map_err(|e| e.to_string())
						}, Msg::ProfileFetched);
					}
					Err(err) => self.status_message = format!("Login failed: {err}"),
				}
				Command::none()
			}
			Msg::ProfileFetched(res) => {
				match res {
					Ok(sp) => {
						let server = sp.server.as_deref().unwrap_or("").to_string();
						if server.is_empty() {
							self.status_message = "Profile missing server address".into();
							return Command::none();
						}
						// Map Docker service name to localhost for client
						let server = if server == "openvpn" { "localhost".to_string() } else { server };
						let name = sp.infer_profile_name();
						let profile = VpnProfile {
							name: name.clone(),
							server,
							remote_id: sp.remote_id,
							auth: AuthMethod::EapMsChapV2 {
								username: sp.username.clone().or_else(|| if self.email.trim().is_empty() { None } else { Some(self.email.trim().to_string()) }),
							},
							split_tunnel: SplitTunnel { enabled: sp.split_tunnel.unwrap_or(false) },
							dns: DnsSettings { servers: sp.dns },
						};
						let adapter = self.adapter.clone();
						let store = self.creds_store.clone();
						let email = self.email.clone();
						let pass = self.password.clone();
						self.current_profile_name = Some(name.clone());
						return Command::perform(async move {
							adapter.add_profile(profile).await.map_err(|e| e.to_string())?;
							let user = sp.username.as_deref().unwrap_or_else(|| email.as_str());
							let pwd = sp.password.as_deref().unwrap_or_else(|| pass.as_str());
							store.set_password(&name, user, pwd).await.map_err(|e| e.to_string())?;
							Ok::<(), String>(())
						}, Msg::Added);
					}
					Err(err) => {
						self.status_message = format!("Profile fetch failed: {err}");
						return Command::none();
					}
				}
			}
			Msg::SelectProfile(idx) => {
				self.selected = Some(idx);
				Command::none()
			}
			Msg::AddNameChanged(s) => {
				self.add_name = s;
				Command::none()
			}
			Msg::AddServerChanged(s) => {
				self.add_server = s;
				Command::none()
			}
			Msg::AddRemoteIdChanged(s) => {
				self.add_remote_id = s;
				Command::none()
			}
			Msg::AddSplitTunnelChanged(b) => {
				self.add_split_tunnel = b;
				Command::none()
			}
			Msg::AddDnsChanged(s) => {
				self.add_dns = s;
				Command::none()
			}
			Msg::AddUsernameChanged(s) => {
				self.add_username = s;
				Command::none()
			}
			Msg::AddSubmit => {
				let profile = VpnProfile {
					name: self.add_name.trim().to_string(),
					server: self.add_server.trim().to_string(),
					remote_id: if self.add_remote_id.trim().is_empty() { None } else { Some(self.add_remote_id.trim().to_string()) },
					auth: AuthMethod::EapMsChapV2 { username: if self.add_username.trim().is_empty() { None } else { Some(self.add_username.trim().to_string()) } },
					split_tunnel: SplitTunnel { enabled: self.add_split_tunnel },
					dns: DnsSettings {
						servers: self.add_dns.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
					},
				};
				let adapter = self.adapter.clone();
				return Command::perform(async move { adapter.add_profile(profile).await.map_err(|e| e.to_string()) }, Msg::Added);
			}
			Msg::Added(res) => {
				match res {
					Ok(_) => {
						self.status_message = "Profile added".into();
						return Command::perform(async {}, |_| Msg::LoadProfiles);
					}
					Err(err) => self.status_message = format!("Add failed: {err}"),
				}
				Command::none()
			}
			Msg::SetCredsUsernameChanged(s) => {
				self.creds_username = s;
				Command::none()
			}
			Msg::SetCredsPasswordChanged(s) => {
				self.creds_password = s;
				Command::none()
			}
			Msg::SetCredsSubmit => {
				let Some(idx) = self.selected else { return Command::none() };
				let name = self.profiles[idx].name.clone();
				let user = self.creds_username.clone();
				let pass = self.creds_password.clone();
				let store = self.creds_store.clone();
				return Command::perform(async move { store.set_password(&name, &user, &pass).await.map_err(|e| e.to_string()) }, Msg::CredsSet);
			}
			Msg::CredsSet(res) => {
				match res {
					Ok(_) => self.status_message = "Credentials saved".into(),
					Err(err) => self.status_message = format!("Credentials save failed: {err}"),
				}
				Command::none()
			}
			Msg::Connect => {
				let Some(name) = self.current_profile_name.clone() else {
					self.status_message = "No profile available. Login first.".into();
					return Command::none();
				};
				let adapter = self.adapter.clone();
				return Command::perform(async move { adapter.connect(&name).await.map_err(|e| e.to_string()) }, Msg::Connected);
			}
			Msg::Connected(res) => {
				match res {
					Ok(_) => self.status_message = "Connected".into(),
					Err(err) => self.status_message = format!("Connect failed: {err}"),
				}
				Command::none()
			}
			Msg::Disconnect => {
				let Some(idx) = self.selected else { return Command::none() };
				let name = self.profiles[idx].name.clone();
				let adapter = self.adapter.clone();
				return Command::perform(async move { adapter.disconnect(&name).await.map_err(|e| e.to_string()) }, Msg::Disconnected);
			}
			Msg::Disconnected(res) => {
				match res {
					Ok(_) => self.status_message = "Disconnected".into(),
					Err(err) => self.status_message = format!("Disconnect failed: {err}"),
				}
				Command::none()
			}
			Msg::Remove => {
				let Some(idx) = self.selected else { return Command::none() };
				let name = self.profiles[idx].name.clone();
				let adapter = self.adapter.clone();
				return Command::perform(async move { adapter.remove_profile(&name).await.map_err(|e| e.to_string()) }, Msg::Removed);
			}
			Msg::Removed(res) => {
				match res {
					Ok(_) => {
						self.status_message = "Removed".into();
						return Command::perform(async {}, |_| Msg::LoadProfiles);
					}
					Err(err) => self.status_message = format!("Remove failed: {err}"),
				}
				Command::none()
			}
		}
	}

	fn view(&self) -> Element<Self::Message> {
		let controls = column![
			text("Login").size(20),
			text_input("Email", &self.email).on_input(Msg::EmailChanged),
			text_input("Password", &self.password).on_input(Msg::PasswordChanged),
			row![
				button(text("Login")).on_press(Msg::Login),
				button(text("Connect")).on_press(Msg::Connect),
			]
			.spacing(8),
			text(&self.status_message),
		]
		.spacing(8)
		.align_items(Alignment::Start);

		let content = container(controls).width(Length::Fill).padding(16);

		content.into()
	}
}


