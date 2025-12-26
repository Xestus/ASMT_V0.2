#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct Version {
    pub value: String,
    pub xmin: u32,
    pub xmax: Option<u32>,
    pub version_status: VersionStatus,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub enum VersionStatus {Active, Commit, Abort, Delete}
