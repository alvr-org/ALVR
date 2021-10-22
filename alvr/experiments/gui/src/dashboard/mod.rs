#[cfg(feature = "gpl")]
mod sixtyfps;
#[cfg(not(feature = "gpl"))]
mod tui;

#[cfg(feature = "gpl")]
pub use self::sixtyfps::*;
#[cfg(not(feature = "gpl"))]
pub use self::tui::*;

use alvr_session::SessionDesc;
use std::net::IpAddr;

pub enum ClientListAction {
    AddIfMissing { display_name: String },
    TrustAndMaybeAddIp(Option<IpAddr>),
    RemoveIpOrEntry(Option<IpAddr>),
}

pub struct ConnectionsEvent {
    pub hostname: String,
    pub action: ClientListAction,
}

pub enum FirewallRulesEvent {
    Add,
    Remove,
}

pub enum DriverRegistrationEvent {
    RegisterAlvr,
    Unregister(String),
}

pub enum DashboardEvent {
    Connections(ConnectionsEvent),
    SessionUpdated(Box<SessionDesc>),
    ApplyPreset(String),
    Driver(DriverRegistrationEvent),
    FirewallRules(FirewallRulesEvent),
    RestartSteamVR,
    UpgradeServer { url: String },
    Command(String),
    Quit,
}
