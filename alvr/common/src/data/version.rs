use crate::*;
use semver::{Version, VersionReq};

pub const ALVR_NAME: &str = "ALVR";

pub const ALVR_SERVER_VERSION: &str = env!("SERVER_VERSION");
pub const ALVR_CLIENT_VERSION: &str = env!("CLIENT_VERSION");

pub const ALVR_SERVER_VERSION_REQ: &str = ">=12.5.0";
pub const ALVR_CLIENT_VERSION_REQ: &str = ">=12.5.0";

pub fn is_version_compatible(version: &str, requirement: &str) -> StrResult<bool> {
    let version = trace_err!(Version::parse(version))?;
    let requirement = trace_err!(VersionReq::parse(requirement))?;
    Ok(requirement.matches(&version))
}
