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

// Error and status message helpers
function showError(element: HTMLElement, message: string) {
	element.textContent = message;
	element.style.color = "#d32f2f";
	show(element);
	setTimeout(() => hide(element), 5000);
}

function showSuccess(element: HTMLElement, message: string) {
	element.textContent = message;
	element.style.color = "#2e7d32";
	show(element);
	setTimeout(() => hide(element), 3000);
}

function setLoading(button: HTMLButtonElement, loading: boolean) {
	if (!button.dataset.originalText) {
		button.dataset.originalText = button.textContent || "";
	}
	button.disabled = loading;
	button.textContent = loading ? "Loading..." : button.dataset.originalText;
}

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
const logoutBtn = $("logout-btn") as HTMLButtonElement;
const logoutBtnRegions = $("logout-btn-regions") as HTMLButtonElement;

// Error/success message elements
const serverMessageEl = $("server-message") as HTMLElement;
const loginErrorEl = $("login-error") as HTMLElement;
const deviceErrorEl = $("device-error") as HTMLElement;
const regionErrorEl = $("region-error") as HTMLElement;
const connectionErrorEl = $("connection-error") as HTMLElement;
const connectionSuccessEl = $("connection-success") as HTMLElement;
const connectionStatusEl = $("connection-status") as HTMLElement;

let deviceState: DeviceStartOut | null = null;
let selectedRegion: Region | null = null;
let selectedMode: "single" | "multi" = "single";
let devicePollInterval: number | null = null;

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
		showSuccess(serverMessageEl, "Cleared server override; using default localhost configuration.");
		return;
	}
	setLoading(serverSaveBtn, true);
	try {
		await invoke("set_server_host", { host });
		try {
			localStorage.setItem("server-host", host);
		} catch {
			// ignore
		}
		showSuccess(serverMessageEl, "Server address saved successfully.");
	} catch (e: any) {
		showError(serverMessageEl, `Failed to save server: ${e}`);
	} finally {
		setLoading(serverSaveBtn, false);
	}
});

async function refreshAppStatus() {
	try {
		const status = await invoke<string>("app_status");
		const session = await invoke<string>("session_status");
		appStatusEl.textContent = `${status} • tunnel:${session}`;
		
		// Update connection status
		if (session.includes("connected") || session.includes("Connected")) {
			connectionStatusEl.textContent = "Connected";
			connectionStatusEl.style.color = "#2e7d32";
		} else if (session.includes("disconnected") || session.includes("Disconnected") || session === "unavailable") {
			connectionStatusEl.textContent = "Disconnected";
			connectionStatusEl.style.color = "#666";
		} else {
			connectionStatusEl.textContent = session;
			connectionStatusEl.style.color = "#666";
		}
	} catch (e) {
		appStatusEl.textContent = "error";
		connectionStatusEl.textContent = "Unknown";
		connectionStatusEl.style.color = "#666";
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
		showError(loginErrorEl, "Please enter both email and password");
		return;
	}
	hide(loginErrorEl);
	setLoading(loginSubmit, true);
	try {
		await invoke<TokenResponse>("login", { req: { email, password, totpCode: totp } });
		hide(loginEl);
		show(regionsEl);
		await populateRegions();
		await refreshAppStatus();
	} catch (e: any) {
		const errorMsg = typeof e === "string" ? e : e?.message || "Login failed. Please check your credentials.";
		showError(loginErrorEl, errorMsg);
	} finally {
		setLoading(loginSubmit, false);
	}
});

checkUpdateBtn.addEventListener("click", async () => {
	setLoading(checkUpdateBtn, true);
	try {
		await invoke("update_check");
		showSuccess(connectionSuccessEl, "Update check completed");
	} catch (e: any) {
		const errorMsg = typeof e === "string" ? e : e?.message || "Update check failed";
		showError(connectionErrorEl, errorMsg);
	} finally {
		setLoading(checkUpdateBtn, false);
	}
});

async function performLogout() {
	try {
		// Clear token from backend
		await invoke("logout");
		// Clear local storage state
		localStorage.removeItem("selected-region-id");
		localStorage.removeItem("selected-mode");
		// Redirect to login
		hide(onboardingEl);
		hide(regionsEl);
		hide(statusEl);
		show(loginEl);
		await refreshAppStatus();
	} catch (e: any) {
		console.error("Logout error:", e);
		// Still show login on error
		hide(onboardingEl);
		hide(regionsEl);
		hide(statusEl);
		show(loginEl);
	}
}

logoutBtn.addEventListener("click", performLogout);
logoutBtnRegions.addEventListener("click", performLogout);

deviceStartBtn.addEventListener("click", async () => {
	const userHint = (document.getElementById("user-hint") as HTMLInputElement).value.trim() || undefined;
	hide(deviceErrorEl);
	setLoading(deviceStartBtn, true);
	try {
		deviceState = await invoke<DeviceStartOut>("device_login_start", { userHint });
		deviceCodeEl.textContent = deviceState.device_code;
		deviceStatusEl.textContent = "Waiting for authorization...";
		show(deviceLoginStarted);
		show(devicePollBtn);
		
		// Start auto-polling
		if (devicePollInterval) {
			clearInterval(devicePollInterval);
		}
		devicePollInterval = window.setInterval(async () => {
			if (!deviceState) return;
			try {
				await invoke<TokenResponse>("device_login_poll", {
					deviceCode: deviceState.device_code,
					codeVerifier: deviceState.code_verifier
				});
				deviceStatusEl.textContent = "Authorized";
				if (devicePollInterval) {
					clearInterval(devicePollInterval);
					devicePollInterval = null;
				}
				hide(loginEl);
				show(regionsEl);
				await populateRegions();
				await refreshAppStatus();
			} catch (e: any) {
				// Still pending, continue polling
				deviceStatusEl.textContent = "Waiting for authorization...";
			}
		}, deviceState.interval * 1000);
	} catch (e: any) {
		const errorMsg = typeof e === "string" ? e : e?.message || "Failed to start device login";
		showError(deviceErrorEl, errorMsg);
	} finally {
		setLoading(deviceStartBtn, false);
	}
});

devicePollBtn.addEventListener("click", async () => {
	if (!deviceState) return;
	setLoading(devicePollBtn, true);
	try {
		await invoke<TokenResponse>("device_login_poll", {
			deviceCode: deviceState.device_code,
			codeVerifier: deviceState.code_verifier
		});
		deviceStatusEl.textContent = "Authorized";
		if (devicePollInterval) {
			clearInterval(devicePollInterval);
			devicePollInterval = null;
		}
		hide(loginEl);
		show(regionsEl);
		await populateRegions();
		await refreshAppStatus();
	} catch (e: any) {
		deviceStatusEl.textContent = "Authorization pending...";
	} finally {
		setLoading(devicePollBtn, false);
	}
});

saveRegionBtn.addEventListener("click", async () => {
	const id = regionSelect.value;
	if (!id) {
		showError(regionErrorEl, "Please select a region");
		return;
	}
	const mode = (modeSelect.value === "multi" ? "multi" : "single") as "single" | "multi";
	hide(regionErrorEl);
	setLoading(saveRegionBtn, true);
	// Validate via backend (directory-api) path selection
	try {
		await invoke("validate_mesh_selection", { regionId: id, mode });
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
	} catch (e: any) {
		const errorMsg = typeof e === "string" ? e : e?.message || "Invalid region selection";
		showError(regionErrorEl, errorMsg);
	} finally {
		setLoading(saveRegionBtn, false);
	}
});

bgStartBtn.addEventListener("click", async () => {
	const regionId = localStorage.getItem("selected-region-id");
	if (!regionId) {
		showError(connectionErrorEl, "Please select a region before connecting");
		return;
	}
	const mode = (localStorage.getItem("selected-mode") === "multi" ? "multi" : "single");
	hide(connectionErrorEl);
	hide(connectionSuccessEl);
	setLoading(bgStartBtn, true);
	try {
		await invoke("connect_session", { regionId, mode, fullTunnel: true });
		showSuccess(connectionSuccessEl, "Successfully connected to VPN");
		await refreshAppStatus();
	} catch (e: any) {
		const errorMsg = typeof e === "string" ? e : e?.message || "Failed to connect. Please try again.";
		showError(connectionErrorEl, errorMsg);
	} finally {
		setLoading(bgStartBtn, false);
	}
});

bgStopBtn.addEventListener("click", async () => {
	hide(connectionErrorEl);
	hide(connectionSuccessEl);
	setLoading(bgStopBtn, true);
	try {
		await invoke("disconnect_session");
		showSuccess(connectionSuccessEl, "Disconnected from VPN");
		await refreshAppStatus();
	} catch (e: any) {
		const errorMsg = typeof e === "string" ? e : e?.message || "Failed to disconnect";
		showError(connectionErrorEl, errorMsg);
	} finally {
		setLoading(bgStopBtn, false);
	}
});

async function init() {
	loadServerHost();
	loadConsent();
	await refreshAppStatus();
	
	// Check authentication status first
	try {
		const status = await invoke<string>("app_status");
		const isAuthorized = status.includes("authorized");
		
		if (!isAuthorized) {
			// Not authenticated - show login page
			hide(onboardingEl);
			hide(regionsEl);
			hide(statusEl);
			show(loginEl);
			return;
		}
		
		// Authenticated - check if region is selected
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
		} else {
			// Authenticated but no region selected - show regions page
			hide(onboardingEl);
			hide(loginEl);
			hide(statusEl);
			show(regionsEl);
			await populateRegions();
		}
	} catch (e) {
		// Error checking status - default to login
		console.error("Failed to check auth status:", e);
		hide(onboardingEl);
		hide(regionsEl);
		hide(statusEl);
		show(loginEl);
	}
	
	// Set up periodic status refresh (every 5 seconds)
	setInterval(async () => {
		try {
			await refreshAppStatus();
		} catch (e) {
			console.error("Status refresh failed:", e);
		}
	}, 5000);
}

init();


