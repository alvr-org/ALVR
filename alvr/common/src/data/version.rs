use crate::*;
use semver::{Version, VersionReq};

pub const ALVR_NAME: &str = "ALVR";

pub const ALVR_SERVER_VERSION: &str = env!("SERVER_VERSION");
pub const ALVR_CLIENT_VERSION: &str = env!("CLIENT_VERSION");

pub const ALVR_SERVER_VERSION_REQ: &str = ">=12.0.0";
pub const ALVR_CLIENT_VERSION_REQ: &str = ">=12.0.0";

pub fn is_version_compatible(version: &str, requirement: &str) -> StrResult<bool> {
    let version = trace_err!(Version::parse(version))?;
    let requirement = trace_err!(VersionReq::parse(requirement))?;
    Ok(requirement.matches(&version))
}

pub fn bumped_versions(server_version: &str, client_version: &str) -> Result<(String, String), String>{
    let server_req: String = format!(">{}", ALVR_SERVER_VERSION).into();
    let server_req = trace_err!(VersionReq::parse(&server_req))?;
    let server_version = trace_err!(Version::parse(server_version))?;

    let client_req: String = format!(">{}", ALVR_CLIENT_VERSION).into();
    let client_req = trace_err!(VersionReq::parse(&client_req))?;
    let client_version = trace_err!(Version::parse(client_version))?;

    let client_version = if client_req.matches(&client_version) {
        format!("{}", client_version)
    } else {
        String::from(ALVR_CLIENT_VERSION)
    };

    let server_version = if server_req.matches(&server_version) {
        format!("{}", server_version)
    } else {
        String::from(ALVR_SERVER_VERSION)
    };

    if client_version == ALVR_CLIENT_VERSION && server_version == ALVR_SERVER_VERSION {
        Err(String::from("Didn't bump any version!"))
    } else {
        Ok((client_version, server_version))
    }
}
