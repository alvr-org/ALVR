use lazy_static::lazy_static;
use semver::Version;

pub const ALVR_NAME: &str = "ALVR";

lazy_static! {
    pub static ref ALVR_SERVER_VERSION: Version = Version::parse(env!("SERVER_VERSION")).unwrap();
    pub static ref ALVR_CLIENT_VERSION: Version = Version::parse("12.5.0").unwrap();
}

// accept semver-compatible versions
// Note: by not having to set the requirement manually, the major version of server and client is
// constrained to be bumped when the packet layouts or some critical behaviour has changed.
pub fn is_version_compatible(test_version: &Version, base_version: &Version) -> bool {
    if test_version.is_prerelease() || base_version.is_prerelease() {
        test_version == base_version
    } else {
        test_version.major == base_version.major
    }
}
