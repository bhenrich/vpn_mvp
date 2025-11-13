// Use Tauri API from @tauri-apps/api
import { invoke as tauriInvoke } from '@tauri-apps/api/tauri';

const invoke = <T>(cmd: string, args?: Record<string, unknown>) => {
	try {
		return tauriInvoke<T>(cmd, args);
	} catch (e) {
		return Promise.reject(`Tauri API not available: ${e}`);
	}
};

type DeviceStartOut = {
	device_code: string;
	expires_in: number;
	interval: number;
	code_verifier: string;
	code_challenge: string;
	code_challenge_method: string;
};

type TokenResponse = {
	access_token: string;
	refresh_token: string;
	expires_in: number;
};

type Region = {
	id: string;
	country_code: string;
	city: string;
	status: string;
	latency_score?: number;
};

const $ = (id: string) => document.getElementById(id)!;
const show = (el: HTMLElement) => el.classList.remove("hidden");
const hide = (el: HTMLElement) => el.classList.add("hidden");

const onboardingEl = $("onboarding") as HTMLElement;
const loginEl = $("login") as HTMLElement;
const regionsEl = $("regions") as HTMLElement;
const statusEl = $("status") as HTMLElement;

const serverHostInput = $("server-host") as HTMLInputElement;
const serverSaveBtn = $("server-save") as HTMLButtonElement;

const telemetryToggle = $("telemetry-consent") as HTMLInputElement;
const onboardingContinue = $("onboarding-continue") as HTMLButtonElement;
const loginSubmit = $("login-submit") as HTMLButtonElement;
const deviceStartBtn = $("device-start") as HTMLButtonElement;
const devicePollBtn = $("device-poll") as HTMLButtonElement;
const deviceLoginStarted = $("device-login-started") as HTMLElement;
const deviceCodeEl = $("device-code") as HTMLElement;
const deviceStatusEl = $("device-status") as HTMLElement;
const regionSelect = $("region") as HTMLSelectElement;
const modeSelect = $("mode") as HTMLSelectElement;
const saveRegionBtn = $("save-region") as HTMLButtonElement;
const appStatusEl = $("app-status") as HTMLElement;
const selectedRegionEl = $("selected-region") as HTMLElement;
const bgStartBtn = $("bg-start") as HTMLButtonElement;
const bgStopBtn = $("bg-stop") as HTMLButtonElement;
const checkUpdateBtn = $("check-update") as HTMLButtonElement;

let deviceState: DeviceStartOut | null = null;
let selectedRegion: Region | null = null;
let selectedMode: "single" | "multi" = "single";

function loadServerHost() {
	try {
		const raw = localStorage.getItem("server-host");
		if (raw && serverHostInput) {
			serverHostInput.value = raw;
			// Best-effort push to backend on startup
			invoke("set_server_host", { host: raw }).catch(() => {
				// ignore; user can re-save explicitly
			});
		}
	} catch {
		// ignore
	}
}

function loadConsent() {
	try {
		const raw = localStorage.getItem("telemetry-consent");
		if (raw) {
			telemetryToggle.checked = raw === "true";
			hide(onboardingEl);
			show(loginEl);
		}
	} catch {
		// ignore
	}
}

function saveConsent() {
	try {
		localStorage.setItem("telemetry-consent", telemetryToggle.checked ? "true" : "false");
	} catch {
		// ignore
	}
}

serverSaveBtn.addEventListener("click", async () => {
	const host = serverHostInput.value.trim();
	if (!host) {
		// Clear override and fall back to defaults (env / localhost)
		try {
			localStorage.removeItem("server-host");
		} catch {
			// ignore
		}
		alert("Cleared server override; using default localhost configuration.");
		return;
	}
	try {
		await invoke("set_server_host", { host });
		try {
			localStorage.setItem("server-host", host);
		} catch {
			// ignore
		}
		alert("Server address saved.");
	} catch (e: any) {
		alert(`Failed to save server: ${e}`);
	}
});

async function refreshAppStatus() {
	try {
		const status = await invoke<string>("app_status");
		appStatusEl.textContent = status;
	} catch (e) {
		appStatusEl.textContent = "error";
	}
}

async function populateRegions() {
	try {
		const regions = await invoke<Region[]>("fetch_regions");
		regionSelect.innerHTML = "";
		for (const r of regions) {
			if (r.status !== "active") continue;
			const opt = document.createElement("option");
			opt.value = r.id;
			opt.textContent = `${r.country_code} • ${r.city}${typeof r.latency_score === "number" ? ` (${r.latency_score.toFixed(1)})` : ""}`;
			regionSelect.appendChild(opt);
		}
		// restore prior choices if present
		try {
			const savedRegionId = localStorage.getItem("selected-region-id");
			const savedMode = localStorage.getItem("selected-mode");
			if (savedRegionId) {
				for (let i = 0; i < regionSelect.options.length; i++) {
					if (regionSelect.options[i].value === savedRegionId) {
						regionSelect.selectedIndex = i;
						break;
					}
				}
			}
			if (savedMode === "single" || savedMode === "multi") {
				modeSelect.value = savedMode;
				selectedMode = savedMode;
			}
		} catch {}
	} catch (e) {
		console.error("Failed to load regions", e);
	}
}

onboardingContinue.addEventListener("click", () => {
	saveConsent();
	hide(onboardingEl);
	show(loginEl);
});

loginSubmit.addEventListener("click", async () => {
	const email = (document.getElementById("email") as HTMLInputElement).value.trim();
	const password = (document.getElementById("password") as HTMLInputElement).value;
	const totp = (document.getElementById("totp") as HTMLInputElement).value.trim() || undefined;
	if (!email || !password) {
		alert("Enter email and password");
		return;
	}
	try {
		await invoke<TokenResponse>("login", { req: { email, password, totpCode: totp } });
		hide(loginEl);
		show(regionsEl);
		await populateRegions();
		await refreshAppStatus();
	} catch (e: any) {
		alert(`Login failed: ${e}`);
	}
});

checkUpdateBtn.addEventListener("click", async () => {
	try {
		await invoke("update_check");
	} catch (e: any) {
		alert(`Update check failed: ${e}`);
	}
});

deviceStartBtn.addEventListener("click", async () => {
	const userHint = (document.getElementById("user-hint") as HTMLInputElement).value.trim() || undefined;
	try {
		deviceState = await invoke<DeviceStartOut>("device_login_start", { userHint });
		deviceCodeEl.textContent = deviceState.device_code;
		deviceStatusEl.textContent = "Waiting for authorization...";
		show(deviceLoginStarted);
		show(devicePollBtn);
	} catch (e: any) {
		alert(`Device login start failed: ${e}`);
	}
});

devicePollBtn.addEventListener("click", async () => {
	if (!deviceState) return;
	try {
		await invoke<TokenResponse>("device_login_poll", {
			deviceCode: deviceState.device_code,
			codeVerifier: deviceState.code_verifier
		});
		deviceStatusEl.textContent = "Authorized";
		hide(loginEl);
		show(regionsEl);
		await populateRegions();
		await refreshAppStatus();
	} catch (e: any) {
		deviceStatusEl.textContent = "Authorization pending...";
	}
});

saveRegionBtn.addEventListener("click", async () => {
	const id = regionSelect.value;
	if (!id) return;
	const mode = (modeSelect.value === "multi" ? "multi" : "single") as "single" | "multi";
	// Validate via backend (directory-api) path selection
	try {
		await invoke("validate_mesh_selection", { regionId: id, mode });
	} catch (e: any) {
		alert(`Selection invalid: ${e}`);
		return;
	}
	// persist locally
	try {
		localStorage.setItem("selected-region-id", id);
		localStorage.setItem("selected-mode", mode);
	} catch {
		// ignore
	}
	// reflect in UI
	const text = regionSelect.options[regionSelect.selectedIndex]?.textContent || "None";
	selectedRegionEl.textContent = `${text} • ${mode === "multi" ? "Multi-hop" : "Single"}` || "None";
	hide(regionsEl);
	show(statusEl);
	await refreshAppStatus();
});

bgStartBtn.addEventListener("click", async () => {
	try {
		const profileName = "default";
		// Start background autoconnect loop directly inside the Tauri process,
		// instead of going through the separate desktop-service sidecar.
		await invoke("start_autoconnect", {
			args: {
				profile: profileName,
				iface: null as any,
				mtu: null as any,
				trusted_ssids: [],
				interval: 10
			}
		});
		await refreshAppStatus();
	} catch (e: any) {
		alert(`Failed to start background: ${e}`);
	}
});

bgStopBtn.addEventListener("click", async () => {
	try {
		// Stop the in-process background autoconnect loop.
		await invoke("stop_autoconnect");
		await refreshAppStatus();
	} catch (e: any) {
		alert(`Failed to stop background: ${e}`);
	}
});

async function init() {
	loadServerHost();
	loadConsent();
	await refreshAppStatus();
	const savedRegionId = localStorage.getItem("selected-region-id");
	if (savedRegionId) {
		await populateRegions();
		for (let i = 0; i < regionSelect.options.length; i++) {
			if (regionSelect.options[i].value === savedRegionId) {
				regionSelect.selectedIndex = i;
				const savedMode = localStorage.getItem("selected-mode");
				const modeLabel = savedMode === "multi" ? "Multi-hop" : "Single";
				selectedRegionEl.textContent = `${regionSelect.options[i].textContent} • ${modeLabel}` || "None";
				break;
			}
		}
		hide(onboardingEl);
		hide(loginEl);
		hide(regionsEl);
		show(statusEl);
	}
}

init();


