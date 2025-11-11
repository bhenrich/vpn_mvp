//! Core crate for cross-platform VPN client logic.
//! Exposes traits, models, and reusable async APIs for the CLI and future Iced UI.

pub mod types;
pub mod traits;
pub mod error;
pub mod secrets;
pub mod exec;
pub mod profiles;
pub mod netauth;
pub mod tokens;

pub use crate::types::*;
pub use crate::traits::*;
pub use crate::error::*;
pub use crate::secrets::*;
pub use crate::exec::*;
pub use crate::profiles::*;
pub use crate::netauth::*;
pub use crate::tokens::*;



