using System;
using System.Runtime.InteropServices;

namespace App.Services.Vpn;

internal static class OpenVPN3Native
{
	// Placeholder P/Invoke signatures for OpenVPN 3 Core.
	// These entry points and structures will be filled in once the official DLL is available.
	// Runtime calls should be guarded by a loader check to avoid EntryPointNotFoundException.

	[DllImport("openvpn3", EntryPoint = "ovpn_version", CallingConvention = CallingConvention.Cdecl)]
	internal static extern IntPtr OvpnVersion();
}


