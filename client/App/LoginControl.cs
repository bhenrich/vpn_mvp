using System;
using System.Drawing;
using System.Windows.Forms;
using App.Services;
using App.ViewModels;
using System.Windows.Input;

namespace App;

public sealed class LoginControl : UserControl
{
	private readonly LoginViewModel _viewModel;
	private readonly TextBox _email;
	private readonly TextBox _password;
	private readonly Button _loginButton;
	private readonly Label _errorLabel;

	public event Action? LoginSucceeded;

	public LoginControl(AuthApiClient authApiClient, TokenStorage tokenStorage)
	{
		_viewModel = new LoginViewModel(authApiClient, tokenStorage, () => LoginSucceeded?.Invoke());

		Dock = DockStyle.Fill;

		var title = new Label
		{
			Text = "Sign in",
			Font = new Font(Font, FontStyle.Bold),
			AutoSize = true,
			Left = 16,
			Top = 16
		};

		_email = new TextBox
		{
			PlaceholderText = "Email",
			Left = 16,
			Top = 48,
			Width = 360
		};

		_password = new TextBox
		{
			UseSystemPasswordChar = true,
			Left = 16,
			Top = 84,
			Width = 360
		};

		_loginButton = new Button
		{
			Text = "Login",
			Left = 16,
			Top = 120,
			Width = 120
		};

		_errorLabel = new Label
		{
			ForeColor = Color.Red,
			AutoSize = true,
			Left = 16,
			Top = 156
		};

		_email.TextChanged += (_, __) =>
		{
			_viewModel.Email = _email.Text;
			UpdateControlsEnabled();
		};
		_password.TextChanged += (_, __) =>
		{
			_viewModel.Password = _password.Text;
			UpdateControlsEnabled();
		};

		_loginButton.Click += (_, __) =>
		{
			var cmd = _viewModel.LoginCommand;
			if (cmd.CanExecute(null))
			{
				cmd.Execute(null);
			}
		};

		_viewModel.PropertyChanged += (_, e) =>
		{
			if (e.PropertyName == nameof(LoginViewModel.ErrorMessage))
			{
				_errorLabel.Text = _viewModel.ErrorMessage;
			}
			if (e.PropertyName == nameof(LoginViewModel.CanLogin))
			{
				UpdateControlsEnabled();
			}
		};

		_viewModel.LoginCommand.CanExecuteChanged += (_, __) => UpdateControlsEnabled();

		Controls.Add(title);
		Controls.Add(_email);
		Controls.Add(_password);
		Controls.Add(_loginButton);
		Controls.Add(_errorLabel);

		UpdateControlsEnabled();
	}

	private void UpdateControlsEnabled()
	{
		_loginButton.Enabled = _viewModel.CanLogin && _viewModel.LoginCommand.CanExecute(null);
	}
}


