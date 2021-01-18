use lazy_static::lazy_static;
use semver::Version;

pub const ALVR_NAME: &str = "ALVR";

lazy_static! {
    pub static ref ALVR_VERSION: Version = Version::parse(env!("VERSION")).unwrap();
}

// accept semver-compatible versions
// Note: by not having to set the requirement manually, the major version is constrained to be
// bumped when the packet layouts or some critical behaviour has changed.
pub fn is_version_compatible(other_version: &Version) -> bool {
    if other_version.is_prerelease() || ALVR_VERSION.is_prerelease() {
        *other_version == *ALVR_VERSION
    } else {
        other_version.major == ALVR_VERSION.major
    }
}
