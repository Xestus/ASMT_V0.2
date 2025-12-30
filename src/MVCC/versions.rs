#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Version {
    pub value: String,
    pub xmin: u32,
    pub xmax: Option<u32>,
    pub version_info: VersionInfo
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct VersionInfo {
    pub version_status: VersionStatus,
    pub vid: u32,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum VersionStatus {Active, Commit, Abort, DeleteActive, DeleteCommit}
