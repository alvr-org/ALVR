use crate::*;
use semver::Version;

pub const ALVR_NAME: &str = "ALVR";

pub const ALVR_SERVER_VERSION: &str = env!("SERVER_VERSION");
pub const ALVR_CLIENT_VERSION: &str = "13.0.0-alpha.0";

// accept semver-compatible versions
// Note: by not having to set the requirement manually, the major version of server and client is
// constrained to be bumped when the packet layouts or some critical behaviour has changed.
pub fn is_version_compatible(version: &str, base: &str) -> StrResult<bool> {
    let version = trace_err!(Version::parse(version))?;
    let base_version = trace_err!(Version::parse(base))?;

    Ok(version.major == base_version.major && version.pre == base_version.pre)
}
