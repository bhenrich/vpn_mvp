using System.Drawing;
using System.Windows.Forms;
using App.Services;

namespace App;

public class MainForm : Form
{
	private readonly AuthApiClient _authApiClient = new();
	private readonly TokenStorage _tokenStorage = new();
	private readonly Panel _contentHost;

	public MainForm()
	{
		Text = "VPN MVP (WinForms)";
		StartPosition = FormStartPosition.CenterScreen;
		ClientSize = new Size(420, 240);

		_contentHost = new Panel
		{
			Dock = DockStyle.Fill,
			Padding = new Padding(12)
		};

		Controls.Add(_contentHost);

		ShowLogin();
	}

	private void ShowLogin()
	{
		var login = new LoginControl(_authApiClient, _tokenStorage);
		login.LoginSucceeded += ShowConnect;
		SwitchContent(login);
	}

	private void ShowConnect()
	{
		SwitchContent(new ConnectControl(_authApiClient, _tokenStorage));
	}

	private void SwitchContent(Control next)
	{
		_contentHost.Controls.Clear();
		_contentHost.Controls.Add(next);
	}
}

