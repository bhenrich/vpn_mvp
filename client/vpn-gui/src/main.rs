use std::sync::Arc;

use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Application, Command, Element, Length, Settings, Theme};
use vpn_core::{
	AuthMethod, CredentialStore, DnsSettings, FileProfileStore, OsKeyringCredentialStore, RealCommandExecutor,
	SplitTunnel, VpnAdapter, VpnProfile,
};
use vpn_core::ProfileStore;
use vpn_core::{AuthApiClient, TokenStore, ServerProfile};

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
	tokens: TokenStore,

	// UI state
	profiles: Vec<VpnProfile>,
	selected: Option<usize>,

	// Auth state
	email: String,
	password: String,
	access_token: Option<String>,

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
	LoggedIn(Result<String, String>),
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
		let tokens = TokenStore::new();

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
				email: String::new(),
				password: String::new(),
				access_token: None,
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
				let rt = self.rt.clone();
				return Command::perform(async move { rt.block_on(store.list()).map_err(|e| e.to_string()) }, Msg::ProfilesLoaded);
			}
			Msg::ProfilesLoaded(res) => {
				match res {
					Ok(list) => self.profiles = list,
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
				let rt = self.rt.clone();
				return Command::perform(
					async move {
						let resp = rt.block_on(auth.login(&email, &password)).map_err(|e| e.to_string())?;
						Ok::<String, String>(resp.access_token)
					},
					Msg::LoggedIn,
				);
			}
			Msg::LoggedIn(res) => {
				match res {
					Ok(token) => {
						self.status_message = "Logged in".into();
						self.access_token = Some(token.clone());
						let auth = self.auth.clone();
						let rt = self.rt.clone();
						return Command::perform(async move { rt.block_on(auth.fetch_profile(&token)).map_err(|e| e.to_string()) }, Msg::ProfileFetched);
					}
					Err(err) => self.status_message = format!("Login failed: {err}"),
				}
				Command::none()
			}
			Msg::ProfileFetched(res) => {
				match res {
					Ok(sp) => {
						let name = sp.infer_profile_name();
						let profile = VpnProfile {
							name: name.clone(),
							server: sp.server.unwrap_or_default(),
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
						let rt = self.rt.clone();
						return Command::perform(async move {
							rt.block_on(adapter.add_profile(profile)).map_err(|e| e.to_string())?;
							let user = sp.username.as_deref().unwrap_or_else(|| email.as_str());
							let pwd = sp.password.as_deref().unwrap_or_else(|| pass.as_str());
							rt.block_on(store.set_password(&name, user, pwd)).map_err(|e| e.to_string())?;
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
				let rt = self.rt.clone();
				return Command::perform(async move { rt.block_on(adapter.add_profile(profile)).map_err(|e| e.to_string()) }, Msg::Added);
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
				let rt = self.rt.clone();
				return Command::perform(async move { rt.block_on(store.set_password(&name, &user, &pass)).map_err(|e| e.to_string()) }, Msg::CredsSet);
			}
			Msg::CredsSet(res) => {
				match res {
					Ok(_) => self.status_message = "Credentials saved".into(),
					Err(err) => self.status_message = format!("Credentials save failed: {err}"),
				}
				Command::none()
			}
			Msg::Connect => {
				let Some(idx) = self.selected else { return Command::none() };
				let name = self.profiles[idx].name.clone();
				let adapter = self.adapter.clone();
				let rt = self.rt.clone();
				return Command::perform(async move { rt.block_on(adapter.connect(&name)).map_err(|e| e.to_string()) }, Msg::Connected);
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
				let rt = self.rt.clone();
				return Command::perform(async move { rt.block_on(adapter.disconnect(&name)).map_err(|e| e.to_string()) }, Msg::Disconnected);
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
				let rt = self.rt.clone();
				return Command::perform(async move { rt.block_on(adapter.remove_profile(&name)).map_err(|e| e.to_string()) }, Msg::Removed);
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
		let children: Vec<Element<Msg>> = self
			.profiles
			.iter()
			.enumerate()
			.map(|(i, p)| {
				let selected = self.selected == Some(i);
				let label = format!("{} -> {}", p.name, p.server);
				button(text(label))
					.on_press(Msg::SelectProfile(i))
					.style(if selected { iced::theme::Button::Primary } else { iced::theme::Button::Secondary })
					.width(Length::Fill)
					.into()
			})
			.collect();

		let list = scrollable(column(children).spacing(8))
		.height(Length::Fill);

		let controls = column![
			text("Login").size(20),
			text_input("Email", &self.email).on_input(Msg::EmailChanged),
			text_input("Password", &self.password).on_input(Msg::PasswordChanged),
			row![
				button(text("Login")).on_press(Msg::Login),
			]
			.spacing(8),
			text("Add Profile").size(20),
			text_input("Name", &self.add_name).on_input(Msg::AddNameChanged),
			text_input("Server", &self.add_server).on_input(Msg::AddServerChanged),
			text_input("Remote ID (optional)", &self.add_remote_id).on_input(Msg::AddRemoteIdChanged),
			checkbox("Split Tunnel", self.add_split_tunnel).on_toggle(Msg::AddSplitTunnelChanged),
			text_input("DNS servers (comma-separated)", &self.add_dns).on_input(Msg::AddDnsChanged),
			text_input("Username (optional)", &self.add_username).on_input(Msg::AddUsernameChanged),
			row![
				button(text("Add")).on_press(Msg::AddSubmit),
				button(text("Remove")).on_press(Msg::Remove),
			]
			.spacing(8),
			text("Credentials").size(20),
			text_input("Username", &self.creds_username).on_input(Msg::SetCredsUsernameChanged),
			text_input("Password", &self.creds_password).on_input(Msg::SetCredsPasswordChanged),
			row![
				button(text("Save Creds")).on_press(Msg::SetCredsSubmit),
				button(text("Connect")).on_press(Msg::Connect),
				button(text("Disconnect")).on_press(Msg::Disconnect),
			]
			.spacing(8),
			text(&self.status_message),
		]
		.spacing(8)
		.align_items(Alignment::Start);

		let content = row![list.width(Length::FillPortion(2)), container(controls).width(Length::FillPortion(3)).padding(16)]
			.spacing(12)
			.align_items(Alignment::Start);

		container(content).padding(16).into()
	}
}


