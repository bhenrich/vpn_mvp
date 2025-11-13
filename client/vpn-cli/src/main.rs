use std::sync::Arc;

use clap::{Args, Parser, Subcommand};
use vpn_core::{AuthMethod, DnsSettings, FileProfileStore, RealCommandExecutor, SplitTunnel, VpnAdapter, VpnProfile};

#[cfg(target_os = "windows")]
use platform_windows::WindowsAdapter;
#[cfg(target_os = "macos")]
use platform_macos::MacosAdapter;
#[cfg(target_os = "linux")]
use platform_linux::LinuxNmAdapter;

#[derive(Parser, Debug)]
#[command(name = "vpn-cli", version, about = "Cross-platform VPN CLI harness")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// Manage profiles
	Profile {
		#[command(subcommand)]
		command: ProfileSub,
	},
	#[cfg(target_os = "windows")]
	/// Windows-specific operations
	Windows {
		#[command(subcommand)]
		command: WinSub,
	},
	#[cfg(target_os = "linux")]
	/// Linux-specific operations
	Linux {
		#[command(subcommand)]
		command: LinuxSub,
	},
	#[cfg(target_os = "macos")]
	/// macOS-specific operations
	Macos {
		#[command(subcommand)]
		command: MacSub,
	},
	/// Connect to a profile
	Connect(NameArg),
	/// Disconnect a profile
	Disconnect(NameArg),
	/// Show status of a profile
	Status(NameArg),
	/// Background service: auto-connect with trusted SSIDs and optional MTU
	Service {
		#[command(subcommand)]
		command: ServiceSub,
	},
}

#[derive(Args, Debug)]
struct NameArg {
	/// Profile name
	#[arg(long)]
	name: String,
}

#[derive(Subcommand, Debug)]
enum ProfileSub {
	/// Add or update a profile
	Add(ProfileAddArgs),
	/// List profiles
	List,
	/// Remove a profile
	Remove(NameArg),
}

#[derive(Args, Debug)]
struct ProfileAddArgs {
	/// Profile name
	#[arg(long)]
	name: String,
	/// Server hostname or address
	#[arg(long)]
	server: String,
	/// Remote ID (optional)
	#[arg(long)]
	remote_id: Option<String>,
	/// Username for EAP-MSCHAPv2 (optional)
	#[arg(long)]
	username: Option<String>,
	/// One or more DNS servers
	#[arg(long, num_args = 0.., value_delimiter = ',')]
	dns: Vec<String>,
	/// Enable split tunneling
	#[arg(long, default_value_t = false)]
	split_tunnel: bool,
}

#[derive(Subcommand, Debug)]
enum ServiceSub {
	/// Run auto-connect loop
	Run(ServiceRunArgs),
}

#[derive(Args, Debug)]
struct ServiceRunArgs {
	/// Profile name to manage
	#[arg(long)]
	profile: String,
	/// Interface name for MTU/DNS ops (optional)
	#[arg(long)]
	iface: Option<String>,
	/// MTU to set on interface when connected (optional)
	#[arg(long)]
	mtu: Option<u32>,
	/// Trusted SSIDs (comma-separated). If connected to one, stay disconnected.
	#[arg(long, value_delimiter = ',')]
	trusted_ssids: Vec<String>,
	/// Poll interval seconds
	#[arg(long, default_value_t = 10)]
	interval: u64,
}

#[cfg(target_os = "windows")]
#[derive(Subcommand, Debug)]
enum WinSub {
	/// Apply Windows Firewall kill switch
	KillSwitchEnable(WinKillArgs),
	/// Remove Windows Firewall kill switch
	KillSwitchDisable,
	/// Allow an application to bypass kill switch (Windows Firewall allow rule)
	SplitAllowApp(WinAppArgs),
	/// Clear application bypass rules
	SplitClearApp,
	/// Set DNS server addresses for the named interface
	DnsSet(WinDnsArgs),
	/// Reset DNS server addresses on the named interface
	DnsReset(WinDnsResetArgs),
	/// Route-based split tunneling: include only these CIDRs via VPN interface
	SplitInclude(WinSplitArgs),
	/// Remove included CIDR routes from VPN interface
	SplitClear(WinSplitArgs),
}

#[cfg(target_os = "windows")]
#[derive(Args, Debug)]
struct WinKillArgs {
	/// Interface alias (name)
	#[arg(long)]
	iface: String,
	/// Allow-listed IPv4 addresses (comma-separated)
	#[arg(long, value_delimiter = ',')]
	allow_v4: Vec<String>,
	/// Allow-listed IPv6 addresses (comma-separated)
	#[arg(long, value_delimiter = ',')]
	allow_v6: Vec<String>,
}

#[cfg(target_os = "windows")]
#[derive(Args, Debug)]
struct WinDnsArgs {
	/// Interface alias (name)
	#[arg(long)]
	iface: String,
	/// DNS servers (comma-separated)
	#[arg(long, value_delimiter = ',')]
	servers: Vec<String>,
}

#[cfg(target_os = "windows")]
#[derive(Args, Debug)]
struct WinAppArgs {
	/// Full path to the executable to allow
	#[arg(long)]
	program: String,
}

#[cfg(target_os = "windows")]
#[derive(Args, Debug)]
struct WinDnsResetArgs {
	/// Interface alias (name)
	#[arg(long)]
	iface: String,
}

#[cfg(target_os = "windows")]
#[derive(Args, Debug)]
struct WinSplitArgs {
	/// Interface alias (name)
	#[arg(long)]
	iface: String,
	/// CIDRs to include or clear (comma-separated)
	#[arg(long, value_delimiter = ',')]
	cidrs: Vec<String>,
}

#[cfg(target_os = "linux")]
#[derive(Subcommand, Debug)]
enum LinuxSub {
	/// Apply nftables-based kill switch (allow DNS/control-plane and VPN iface traffic only)
	KillSwitchEnable(KillSwitchArgs),
	/// Remove nftables kill switch
	KillSwitchDisable,
	/// Set DNS servers on interface via systemd-resolved and enable routing-only domains
	DnsSet(LinuxDnsArgs),
	/// Revert DNS on interface via systemd-resolved
	DnsRevert(LinuxDnsRevertArgs),
	/// Route-based split tunneling: include only these CIDRs via VPN interface
	SplitInclude(LinuxSplitArgs),
	/// Remove included CIDR routes from VPN interface
	SplitClear(LinuxSplitArgs),
}

#[cfg(target_os = "linux")]
#[derive(Args, Debug)]
struct KillSwitchArgs {
	/// Interface name (e.g., tun0)
	#[arg(long)]
	iface: String,
	/// Allow-listed IPv4 addresses (comma-separated)
	#[arg(long, value_delimiter = ',')]
	allow_v4: Vec<String>,
	/// Allow-listed IPv6 addresses (comma-separated)
	#[arg(long, value_delimiter = ',')]
	allow_v6: Vec<String>,
	/// Allow-listed UIDs whose traffic may bypass the kill switch
	#[arg(long, value_delimiter = ',')]
	allow_uids: Vec<u32>,
}

#[cfg(target_os = "linux")]
#[derive(Args, Debug)]
struct LinuxDnsArgs {
	/// Interface name (e.g., tun0)
	#[arg(long)]
	iface: String,
	/// DNS servers (comma-separated)
	#[arg(long, value_delimiter = ',')]
	servers: Vec<String>,
}

#[cfg(target_os = "linux")]
#[derive(Args, Debug)]
struct LinuxDnsRevertArgs {
	/// Interface name (e.g., tun0)
	#[arg(long)]
	iface: String,
}

#[cfg(target_os = "linux")]
#[derive(Args, Debug)]
struct LinuxSplitArgs {
	/// Interface name (e.g., tun0)
	#[arg(long)]
	iface: String,
	/// CIDRs to include or clear (comma-separated)
	#[arg(long, value_delimiter = ',')]
	cidrs: Vec<String>,
}

#[cfg(target_os = "macos")]
#[derive(Subcommand, Debug)]
enum MacSub {
	/// Bring up a utun interface (creation via tun crate; link up via ifconfig)
	TunUp(MacTunArgs),
	/// Bring down a utun interface
	TunDown(MacTunArgs),
	/// Apply PF kill switch (allow utun egress and optional control-plane IPs)
	KillSwitchEnable(MacKillArgs),
	/// Revert PF rules to system defaults
	KillSwitchDisable,
	/// List network services recognized by macOS
	ListServices,
	/// Set DNS servers for a specific macOS network service
	DnsSet(MacDnsArgs),
	/// Clear DNS servers (reset to system) for a service
	DnsClear(MacDnsClearArgs),
}

#[cfg(target_os = "macos")]
#[derive(Args, Debug)]
struct MacTunArgs {
	/// Interface name (e.g., utun8)
	#[arg(long)]
	iface: String,
}

#[cfg(target_os = "macos")]
#[derive(Args, Debug)]
struct MacKillArgs {
	/// Interface name (e.g., utun8)
	#[arg(long)]
	iface: String,
	/// Allow-listed IPv4 addresses (comma-separated)
	#[arg(long, value_delimiter = ',')]
	allow_v4: Vec<String>,
	/// Allow-listed IPv6 addresses (comma-separated)
	#[arg(long, value_delimiter = ',')]
	allow_v6: Vec<String>,
}

#[cfg(target_os = "macos")]
#[derive(Args, Debug)]
struct MacDnsArgs {
	/// Service name (e.g., Wi-Fi)
	#[arg(long)]
	service: String,
	/// DNS servers (comma-separated)
	#[arg(long, value_delimiter = ',')]
	servers: Vec<String>,
}

#[cfg(target_os = "macos")]
#[derive(Args, Debug)]
struct MacDnsClearArgs {
	/// Service name (e.g., Wi-Fi)
	#[arg(long)]
	service: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();
	let exec = Arc::new(RealCommandExecutor);
	let profiles = Arc::new(FileProfileStore::new("vpn_mvp")?);

	#[cfg(target_os = "windows")]
	let adapter = {
		let creds = Arc::new(vpn_core::OsKeyringCredentialStore);
		WindowsAdapter::new(exec.clone(), creds, profiles.clone())
	};
	#[cfg(target_os = "macos")]
	let adapter = {
		let creds = Arc::new(vpn_core::OsKeyringCredentialStore);
		MacosAdapter::new(exec.clone(), creds, profiles.clone())
	};
	#[cfg(target_os = "linux")]
	let adapter = {
		LinuxNmAdapter::new(exec.clone(), profiles.clone())
	};

	match cli.command {
		Commands::Service { command } => match command {
			ServiceSub::Run(args) => {
				run_service_loop(exec.clone(), adapter, args).await?;
				return Ok(());
			}
		},
		#[cfg(target_os = "windows")]
		Commands::Windows { command } => {
			match command {
				WinSub::KillSwitchEnable(args) => {
					use platform_windows::firewall::WindowsFirewall;
					let fw = WindowsFirewall::new(exec.clone());
					let allow_v4: Vec<&str> = args.allow_v4.iter().map(|s| s.as_str()).collect();
					let allow_v6: Vec<&str> = args.allow_v6.iter().map(|s| s.as_str()).collect();
					fw.apply_killswitch(&args.iface, &allow_v4, &allow_v6).await.map_err(err)?;
					println!("Windows kill switch applied");
				}
				WinSub::KillSwitchDisable => {
					use platform_windows::firewall::WindowsFirewall;
					let fw = WindowsFirewall::new(exec.clone());
					fw.remove_killswitch().await.map_err(err)?;
					println!("Windows kill switch removed");
				}
				WinSub::SplitAllowApp(args) => {
					use platform_windows::firewall::WindowsFirewall;
					let fw = WindowsFirewall::new(exec.clone());
					fw.allow_program(&args.program).await.map_err(err)?;
					println!("Windows app allowed: {}", args.program);
				}
				WinSub::SplitClearApp => {
					use platform_windows::firewall::WindowsFirewall;
					let fw = WindowsFirewall::new(exec.clone());
					fw.clear_app_rules().await.map_err(err)?;
					println!("Windows app allow rules cleared");
				}
				WinSub::DnsSet(args) => {
					use platform_windows::dns::WindowsDns;
					let wdns = WindowsDns::new(exec.clone());
					let servers: Vec<&str> = args.servers.iter().map(|s| s.as_str()).collect();
					wdns.set_for_interface(&args.iface, &servers).await.map_err(err)?;
					println!("Windows DNS set for {}", args.iface);
				}
				WinSub::DnsReset(args) => {
					use platform_windows::dns::WindowsDns;
					let wdns = WindowsDns::new(exec.clone());
					wdns.set_for_interface(&args.iface, &[]).await.map_err(err)?;
					println!("Windows DNS reset for {}", args.iface);
				}
				WinSub::SplitInclude(args) => {
					use platform_windows::split::WindowsSplitRoutes;
					let split = WindowsSplitRoutes::new(exec.clone());
					let cidrs: Vec<&str> = args.cidrs.iter().map(|s| s.as_str()).collect();
					split.set_include(&args.iface, &cidrs).await.map_err(err)?;
					println!("Windows split include routes applied");
				}
				WinSub::SplitClear(args) => {
					use platform_windows::split::WindowsSplitRoutes;
					let split = WindowsSplitRoutes::new(exec.clone());
					let cidrs: Vec<&str> = args.cidrs.iter().map(|s| s.as_str()).collect();
					split.clear(&args.iface, &cidrs).await.map_err(err)?;
					println!("Windows split include routes cleared");
				}
			}
		}
		Commands::Profile { command } => match command {
			ProfileSub::Add(args) => {
				let profile = VpnProfile {
					name: args.name,
					server: args.server,
					remote_id: args.remote_id,
					auth: AuthMethod::EapMsChapV2 { username: args.username },
					split_tunnel: SplitTunnel {
						enabled: args.split_tunnel,
					},
					dns: DnsSettings { servers: args.dns },
				};
				adapter.add_profile(profile).await.map_err(err)?;
				println!("Profile added");
			}
			ProfileSub::List => {
				let list = adapter.list_profiles().await.map_err(err)?;
				for p in list {
					println!("{} -> {}", p.name, p.server);
				}
			}
			ProfileSub::Remove(NameArg { name }) => {
				adapter.remove_profile(&name).await.map_err(err)?;
				println!("Profile removed");
			}
		},
		#[cfg(target_os = "linux")]
		Commands::Linux { command } => {
			match command {
				LinuxSub::KillSwitchEnable(args) => {
					use platform_linux::firewall::LinuxNftables;
					let fw = LinuxNftables::new(exec.clone());
					let allow_v4: Vec<&str> = args.allow_v4.iter().map(|s| s.as_str()).collect();
					let allow_v6: Vec<&str> = args.allow_v6.iter().map(|s| s.as_str()).collect();
					if args.allow_uids.is_empty() {
						fw.apply_killswitch(&args.iface, &allow_v4, &allow_v6).await.map_err(err)?;
					} else {
						fw.apply_killswitch_with_uids(&args.iface, &allow_v4, &allow_v6, &args.allow_uids).await.map_err(err)?;
					}
					println!("Kill switch applied");
				}
				LinuxSub::KillSwitchDisable => {
					use platform_linux::firewall::LinuxNftables;
					let fw = LinuxNftables::new(exec.clone());
					fw.remove_killswitch().await.map_err(err)?;
					println!("Kill switch removed");
				}
				LinuxSub::DnsSet(args) => {
					use platform_linux::dns::LinuxResolvedDns;
					let dns = LinuxResolvedDns::new(exec.clone());
					let servers: Vec<&str> = args.servers.iter().map(|s| s.as_str()).collect();
					dns.set_dns(&args.iface, &servers).await.map_err(err)?;
					println!("DNS set via systemd-resolved");
				}
				LinuxSub::DnsRevert(args) => {
					use platform_linux::dns::LinuxResolvedDns;
					let dns = LinuxResolvedDns::new(exec.clone());
					dns.revert(&args.iface).await.map_err(err)?;
					println!("DNS reverted for interface");
				}
				LinuxSub::SplitInclude(args) => {
					use platform_linux::split::LinuxSplitRoutes;
					let split = LinuxSplitRoutes::new(exec.clone());
					let cidrs: Vec<&str> = args.cidrs.iter().map(|s| s.as_str()).collect();
					split.set_include(&args.iface, &cidrs).await.map_err(err)?;
					println!("Split include routes applied");
				}
				LinuxSub::SplitClear(args) => {
					use platform_linux::split::LinuxSplitRoutes;
					let split = LinuxSplitRoutes::new(exec.clone());
					let cidrs: Vec<&str> = args.cidrs.iter().map(|s| s.as_str()).collect();
					split.clear(&args.iface, &cidrs).await.map_err(err)?;
					println!("Split include routes cleared");
				}
			}
		}
		#[cfg(target_os = "macos")]
		Commands::Macos { command } => {
			match command {
				MacSub::TunUp(args) => {
					use platform_macos::tun_backend::MacOsUtunBackend;
					let tun = MacOsUtunBackend::new(exec.clone(), args.iface.clone());
					tun.open(&args.iface).await.map_err(err)?;
					tun.up().await.map_err(err)?;
					println!("{} up", args.iface);
				}
				MacSub::TunDown(args) => {
					use platform_macos::tun_backend::MacOsUtunBackend;
					let tun = MacOsUtunBackend::new(exec.clone(), args.iface.clone());
					tun.down().await.map_err(err)?;
					println!("{} down", args.iface);
				}
				MacSub::KillSwitchEnable(args) => {
					use platform_macos::firewall::MacPfFirewall;
					let fw = MacPfFirewall::new(exec.clone());
					let allow_v4: Vec<&str> = args.allow_v4.iter().map(|s| s.as_str()).collect();
					let allow_v6: Vec<&str> = args.allow_v6.iter().map(|s| s.as_str()).collect();
					fw.apply_killswitch(&args.iface, &allow_v4, &allow_v6).await.map_err(err)?;
					println!("PF kill switch applied");
				}
				MacSub::KillSwitchDisable => {
					use platform_macos::firewall::MacPfFirewall;
					let fw = MacPfFirewall::new(exec.clone());
					fw.revert().await.map_err(err)?;
					println!("PF reverted");
				}
				MacSub::ListServices => {
					use platform_macos::dns::MacDns;
					let mdns = MacDns::new(exec.clone());
					for svc in mdns.list_services().await.map_err(err)? {
						println!("{svc}");
					}
				}
				MacSub::DnsSet(args) => {
					use platform_macos::dns::MacDns;
					let mdns = MacDns::new(exec.clone());
					let servers: Vec<&str> = args.servers.iter().map(|s| s.as_str()).collect();
					mdns.set_dns_for_service(&args.service, &servers).await.map_err(err)?;
					println!("DNS set for {}", args.service);
				}
				MacSub::DnsClear(args) => {
					use platform_macos::dns::MacDns;
					let mdns = MacDns::new(exec.clone());
					mdns.set_dns_for_service(&args.service, &[]).await.map_err(err)?;
					println!("DNS cleared for {}", args.service);
				}
			}
		}
		Commands::Connect(NameArg { name }) => {
			adapter.connect(&name).await.map_err(err)?;
			println!("Connected");
		}
		Commands::Disconnect(NameArg { name }) => {
			adapter.disconnect(&name).await.map_err(err)?;
			println!("Disconnected");
		}
		Commands::Status(NameArg { name }) => {
			let s = adapter.status(&name).await.map_err(err)?;
			println!("{s:?}");
		}
	}

	Ok(())
}

fn err(e: vpn_core::CoreError) -> Box<dyn std::error::Error> {
	Box::new(e)
}

async fn run_service_loop<T: VpnAdapter>(
	exec: Arc<dyn vpn_core::CommandExecutor>,
	adapter: T,
	args: ServiceRunArgs,
) -> Result<(), Box<dyn std::error::Error>> {
	use vpn_core::ConnectionState;
	loop {
		let ssid = current_ssid(&*exec).await.unwrap_or_default();
		let trusted = !ssid.is_empty() && args.trusted_ssids.iter().any(|t| t == &ssid);
		let status = adapter.status(&args.profile).await.unwrap_or(ConnectionState::Unknown);
		if trusted {
			if matches!(status, ConnectionState::Connected | ConnectionState::Connecting) {
				let _ = adapter.disconnect(&args.profile).await;
			}
		} else {
			if !matches!(status, ConnectionState::Connected | ConnectionState::Connecting) {
				adapter.connect(&args.profile).await.map_err(err)?;
				if let (Some(iface), Some(mtu)) = (&args.iface, args.mtu) {
					let _ = set_mtu(&*exec, iface, mtu).await;
				}
			}
		}
		tokio::time::sleep(std::time::Duration::from_secs(args.interval)).await;
	}
}

async fn current_ssid(exec: &dyn vpn_core::CommandExecutor) -> Result<String, vpn_core::CoreError> {
	#[cfg(target_os = "windows")]
	{
		let out = exec.run("netsh", &["wlan", "show", "interfaces"]).await?;
		if out.status == 0 {
			for line in out.stdout.lines() {
				let l = line.trim();
				if l.starts_with("SSID") {
					if let Some(idx) = l.find(':') {
						return Ok(l[idx + 1..].trim().to_string());
					}
				}
			}
		}
	}
	#[cfg(target_os = "macos")]
	{
		let out = exec.run("networksetup", &["-getairportnetwork", "Wi-Fi"]).await?;
		if out.status == 0 {
			if let Some(idx) = out.stdout.find(':') {
				return Ok(out.stdout[idx + 1..].trim().to_string());
			}
		}
	}
	#[cfg(target_os = "linux")]
	{
		let out = exec.run("nmcli", &["-t", "-f", "ACTIVE,SSID", "dev", "wifi"]).await?;
		if out.status == 0 {
			for line in out.stdout.lines() {
				if let Some((active, ssid)) = line.split_once(':') {
					if active == "yes" {
						return Ok(ssid.to_string());
					}
				}
			}
		}
	}
	Ok(String::new())
}

async fn set_mtu(exec: &dyn vpn_core::CommandExecutor, iface: &str, mtu: u32) -> Result<(), vpn_core::CoreError> {
	#[cfg(target_os = "windows")]
	{
		let args = [
			"interface",
			"ipv4",
			"set",
			"subinterface",
			&format!("\"{}\"", iface),
			&format!("mtu={}", mtu),
			"store=active",
		];
		let out = exec.run("netsh", &args).await?;
		if out.status != 0 {
			return Err(vpn_core::CoreError::CommandFailed(out.stderr));
		}
	}
	#[cfg(target_os = "macos")]
	{
		let out = exec.run("ifconfig", &[iface, "mtu", &mtu.to_string()]).await?;
		if out.status != 0 {
			return Err(vpn_core::CoreError::CommandFailed(out.stderr));
		}
	}
	#[cfg(target_os = "linux")]
	{
		let out = exec.run("ip", &["link", "set", "dev", iface, "mtu", &mtu.to_string()]).await?;
		if out.status != 0 {
			return Err(vpn_core::CoreError::CommandFailed(out.stderr));
		}
	}
	Ok(())
}


