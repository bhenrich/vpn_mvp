using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace App.Services.Vpn;

public sealed class OpenVPN3Engine
{
	private bool _libraryLoaded;
	private string? _loadError;
	private static IntPtr _openvpn3Handle;
	private static bool _resolverSet;

	public bool IsLoaded => _libraryLoaded;
	public string? LoadError => _loadError;

	public OpenVPN3Engine()
	{
		_libraryLoaded = TryLoad(out _loadError);
	}

	public async Task ConnectAsync(string ovpnProfile, string username, string password)
	{
		if (!_libraryLoaded)
		{
			throw new InvalidOperationException($"OpenVPN3 core not loaded: {_loadError ?? "unknown error"}");
		}

		// TODO: Use OpenVPN3Native to create and start a session with provided credentials.
		await Task.CompletedTask;
	}

	public Task DisconnectAsync()
	{
		// TODO: Stop and dispose the OpenVPN3 session.
		return Task.CompletedTask;
	}

	private static bool TryLoad(out string? error)
	{
		error = null;
		if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
		{
			error = "Only Windows is supported in this MVP.";
			return false;
		}

		// Preferred probing locations
		var baseDir = AppContext.BaseDirectory;
		var programFiles = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles);
		var envOverride = Environment.GetEnvironmentVariable("OPENVPN3_DLL");
		var candidates = new[]
		{
			// Explicit override
			envOverride ?? string.Empty,
			// App-local copies
			Path.Combine(baseDir, "openvpn3.dll"),
			Path.Combine(baseDir, "ovpncli.dll"),
			Path.Combine(baseDir, "runtimes", "win-x64", "native", "openvpn3.dll"),
			Path.Combine(baseDir, "runtimes", "win-x64", "native", "ovpncli.dll"),
			Path.Combine(baseDir, "Native", "openvpn3", "runtimes", "win-x64", "native", "openvpn3.dll"),
			Path.Combine(baseDir, "Native", "openvpn3", "runtimes", "win-x64", "native", "ovpncli.dll"),
			// Typical OpenVPN Connect installation path
			Path.Combine(programFiles, "OpenVPN Connect", "ovpncli.dll"),
			Path.Combine(programFiles, "OpenVPN Connect", "openvpn3.dll"),
		};

		foreach (var path in candidates)
		{
			if (!string.IsNullOrWhiteSpace(path) && File.Exists(path))
			{
				if (NativeLibrary.TryLoad(path, out var handle))
				{
					_openvpn3Handle = handle;
					// Provide a resolver so DllImport("openvpn3") binds to the loaded handle
					if (!_resolverSet)
					{
						try
						{
							NativeLibrary.SetDllImportResolver(typeof(OpenVPN3Native).Assembly, (name, assembly, searchPath) =>
							{
								if (string.Equals(name, "openvpn3", StringComparison.OrdinalIgnoreCase))
								{
									return _openvpn3Handle;
								}
								return IntPtr.Zero;
							});
							_resolverSet = true;
						}
						catch
						{
							// ignore; resolver may already be set
						}
					}
					return true;
				}
			}
		}

		// Fallback to default resolver (PATH/LD_LIBRARY_PATH)
		if (NativeLibrary.TryLoad("openvpn3", out _))
		{
			return true;
		}

		error = "OpenVPN 3 Core DLL not found. Install OpenVPN Connect (includes ovpncli.dll) or place openvpn3.dll/ovpncli.dll under Native/openvpn3/runtimes/win-x64/native/ or next to the executable. You can also set OPENVPN3_DLL to the full path.";
		return false;
	}
}


