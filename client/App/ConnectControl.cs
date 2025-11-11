using System;
using System.Drawing;
using System.Windows.Forms;
using App.Services;
using App.ViewModels;

namespace App;

public sealed class ConnectControl : UserControl
{
	private readonly ConnectViewModel _viewModel;
	private readonly Label _status;
	private readonly Button _connectButton;
	private readonly Button _disconnectButton;

	public ConnectControl(AuthApiClient authApiClient, TokenStorage tokenStorage)
	{
		_viewModel = new ConnectViewModel(authApiClient, tokenStorage);

		Dock = DockStyle.Fill;

		var title = new Label
		{
			Text = "VPN Connection",
			Font = new Font(Font, FontStyle.Bold),
			AutoSize = true,
			Left = 16,
			Top = 16
		};

		_status = new Label
		{
			Text = _viewModel.StatusText,
			AutoSize = true,
			Left = 16,
			Top = 48
		};

		_connectButton = new Button
		{
			Text = "Connect",
			Left = 16,
			Top = 84,
			Width = 120
		};
		_disconnectButton = new Button
		{
			Text = "Disconnect",
			Left = 152,
			Top = 84,
			Width = 120
		};

		_connectButton.Click += (_, __) =>
		{
			if (_viewModel.ConnectCommand.CanExecute(null))
			{
				_viewModel.ConnectCommand.Execute(null);
			}
		};
		_disconnectButton.Click += (_, __) =>
		{
			if (_viewModel.DisconnectCommand.CanExecute(null))
			{
				_viewModel.DisconnectCommand.Execute(null);
			}
		};

		_viewModel.PropertyChanged += (_, e) =>
		{
			if (e.PropertyName == nameof(ConnectViewModel.StatusText))
			{
				_status.Text = _viewModel.StatusText;
			}
			if (e.PropertyName == nameof(ConnectViewModel.CanConnect) ||
				e.PropertyName == nameof(ConnectViewModel.CanDisconnect))
			{
				UpdateControlsEnabled();
			}
		};

		_viewModel.ConnectCommand.CanExecuteChanged += (_, __) => UpdateControlsEnabled();
		_viewModel.DisconnectCommand.CanExecuteChanged += (_, __) => UpdateControlsEnabled();

		Controls.Add(title);
		Controls.Add(_status);
		Controls.Add(_connectButton);
		Controls.Add(_disconnectButton);

		UpdateControlsEnabled();
	}

	private void UpdateControlsEnabled()
	{
		_connectButton.Enabled = _viewModel.CanConnect && _viewModel.ConnectCommand.CanExecute(null);
		_disconnectButton.Enabled = _viewModel.CanDisconnect && _viewModel.DisconnectCommand.CanExecute(null);
	}
}


