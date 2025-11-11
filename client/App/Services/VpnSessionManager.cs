using System;
using System.Threading.Tasks;
using App.Services.Vpn;

namespace App.Services;

public sealed class VpnSessionManager
{
	private readonly OpenVPN3Engine _engine = new();

	public Task ConnectAsync(string ovpnProfile, string username, string password)
	{
		if (!_engine.IsLoaded)
		{
			throw new InvalidOperationException(_engine.LoadError ?? "OpenVPN 3 Core not available.");
		}
		return _engine.ConnectAsync(ovpnProfile, username, password);
	}

	public Task DisconnectAsync()
	{
		return _engine.DisconnectAsync();
	}
}


